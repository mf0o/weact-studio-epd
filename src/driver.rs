use core::iter;

#[cfg(not(feature = "blocking"))]
use display_interface::AsyncWriteOnlyDataCommand;
use display_interface::DataFormat;
#[cfg(feature = "blocking")]
use display_interface::WriteOnlyDataCommand;
#[cfg(feature = "blocking")]
use embedded_hal::delay::DelayNs;
use embedded_hal::digital::{InputPin, OutputPin};
#[cfg(not(feature = "blocking"))]
use embedded_hal_async::{delay::DelayNs, digital::Wait};

#[cfg(feature = "graphics")]
use crate::graphics::Display;
use crate::{
    color::{self, ColorType},
    command, flag, Color, Result, TriColor,
};

/// Display driver for the WeAct Studio 2.9 inch B/W display.
pub type WeActStudio290BlackWhiteDriver<DI, BSY, RST, DELAY> = DisplayDriver<DI, BSY, RST, DELAY, 128, 128, 296, Color>;
/// Display driver for the WeAct Studio 2.9 inch Tri-Color display.
pub type WeActStudio290TriColorDriver<DI, BSY, RST, DELAY> =
    DisplayDriver<DI, BSY, RST, DELAY, 128, 128, 296, TriColor>;
/// Display driver for the WeAct Studio 2.13 inch B/W display.
pub type WeActStudio213BlackWhiteDriver<DI, BSY, RST, DELAY> = DisplayDriver<DI, BSY, RST, DELAY, 128, 122, 250, Color>;
/// Display driver for the WeAct Studio 2.13 inch Tri-Color display.
pub type WeActStudio213TriColorDriver<DI, BSY, RST, DELAY> =
    DisplayDriver<DI, BSY, RST, DELAY, 128, 122, 250, TriColor>;

/// The main driver struct. Use one of the display-specific type aliases instead.
pub struct DisplayDriver<DI, BSY, RST, DELAY, const WIDTH: u32, const VISIBLE_WIDTH: u32, const HEIGHT: u32, C> {
    _color: core::marker::PhantomData<C>,
    interface: DI,
    busy: BSY,
    reset: RST,
    delay: DELAY,
    using_partial_mode: bool,
    initial_full_refresh_done: bool,
}

#[maybe_async_cfg::maybe(
    sync(
        feature = "blocking",
        keep_self,
        idents(AsyncWriteOnlyDataCommand(sync = "WriteOnlyDataCommand"), Wait(sync = "InputPin"))
    ),
    async(not(feature = "blocking"), keep_self)
)]
impl<DI, BSY, RST, DELAY, const WIDTH: u32, const VISIBLE_WIDTH: u32, const HEIGHT: u32, C>
    DisplayDriver<DI, BSY, RST, DELAY, WIDTH, VISIBLE_WIDTH, HEIGHT, C>
where
    DI: AsyncWriteOnlyDataCommand,
    BSY: InputPin + Wait,
    RST: OutputPin,
    DELAY: DelayNs,
    C: ColorType,
{
    const RESET_DELAY_MS: u32 = 50;

    /// Create a new display driver. Call [`Self::init`] before use.
    pub fn new(interface: DI, busy: BSY, reset: RST, delay: DELAY) -> Self {
        Self {
            _color: core::marker::PhantomData,
            interface,
            busy,
            reset,
            delay,
            using_partial_mode: false,
            initial_full_refresh_done: false,
        }
    }

    /// Initialize the display.
    pub async fn init(&mut self) -> Result<()> {
        self.hw_reset().await;
        self.command(command::SW_RESET).await?;
        self.delay.delay_ms(10).await;
        self.wait_until_idle().await;
        self.command_with_data(command::DRIVER_CONTROL, &[(HEIGHT - 1) as u8, ((HEIGHT - 1) >> 8) as u8, 0x00]).await?;
        self.command_with_data(command::DATA_ENTRY_MODE, &[flag::DATA_ENTRY_INCRY_INCRX]).await?;
        // 0x05: border follows full-refresh LUT waveform.
        self.command_with_data(command::BORDER_WAVEFORM_CONTROL, &[0x05]).await?;
        // Source bypass for FPC-7519 colstart=8 on this PCB; POR = [0x00, 0x00] shifts output.
        self.command_with_data(command::DISPLAY_UPDATE_CONTROL, &[0x00, 0x80]).await?;
        // Internal temperature sensor. POR = 0x48 (external); floating TEMP pin gives bad readings.
        self.command_with_data(command::TEMP_CONTROL, &[flag::INTERNAL_TEMP_SENSOR]).await?;
        self.use_full_frame().await?;
        self.wait_until_idle().await;
        Ok(())
    }

    /// Perform a hardware reset.
    pub async fn hw_reset(&mut self) {
        self.reset.set_low().unwrap();
        self.delay.delay_ms(Self::RESET_DELAY_MS).await;
        self.reset.set_high().unwrap();
        self.delay.delay_ms(Self::RESET_DELAY_MS).await;
    }

    /// Write to the B/W buffer (DTM1, 0x24).
    pub async fn write_bw_buffer(&mut self, buffer: &[u8]) -> Result<()> {
        self.use_full_frame().await?;
        self.command_with_data(command::WRITE_BW_DATA, buffer).await?;
        Ok(())
    }

    /// Write to the red buffer (DTM2, 0x26). On B/W displays this is the differential reference.
    pub async fn write_red_buffer(&mut self, buffer: &[u8]) -> Result<()> {
        self.use_full_frame().await?;
        self.command_with_data(command::WRITE_RED_DATA, buffer).await?;
        Ok(())
    }

    /// Write to the B/W buffer at the given position. `x` and `width` must be multiples of 8.
    pub async fn write_partial_bw_buffer(
        &mut self,
        buffer: &[u8],
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    ) -> Result<()> {
        self.use_partial_frame(x, y, width, height).await?;
        self.command_with_data(command::WRITE_BW_DATA, buffer).await?;
        Ok(())
    }

    /// Write to the red buffer at the given position. `x` and `width` must be multiples of 8.
    pub async fn write_partial_red_buffer(
        &mut self,
        buffer: &[u8],
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    ) -> Result<()> {
        self.use_partial_frame(x, y, width, height).await?;
        self.command_with_data(command::WRITE_RED_DATA, buffer).await?;
        Ok(())
    }

    /// Fill the B/W buffer with white.
    pub async fn clear_bw_buffer(&mut self) -> Result<()> {
        self.use_full_frame().await?;
        let color = color::Color::White.byte_value().0;
        self.command(command::WRITE_BW_DATA).await?;
        self.data_x_times(color, WIDTH / 8 * HEIGHT).await?;
        Ok(())
    }

    /// Fill the red buffer with white.
    pub async fn clear_red_buffer(&mut self) -> Result<()> {
        self.use_full_frame().await?;
        let color = color::Color::White.byte_value().1;
        self.command(command::WRITE_RED_DATA).await?;
        self.data_x_times(color, WIDTH / 8 * HEIGHT).await?;
        Ok(())
    }

    /// Full refresh using the panel OTP LUT (0xF7).
    pub async fn full_refresh(&mut self) -> Result<()> {
        self.initial_full_refresh_done = true;
        self.using_partial_mode = false;
        self.command_with_data(command::BORDER_WAVEFORM_CONTROL, &[0x05]).await?;
        self.command_with_data(command::UPDATE_DISPLAY_CTRL2, &[flag::DISPLAY_MODE_1]).await?;
        self.command(command::MASTER_ACTIVATE).await?;
        self.wait_until_idle().await;
        Ok(())
    }

    /// Enter deep sleep mode 1 (RAM retained). Use [`Self::wake_up`] or [`Self::init`] to resume.
    pub async fn sleep(&mut self) -> Result<()> {
        // Cannot use command_with_data: BUSY stays HIGH in deep sleep so data() would hang.
        self.interface.send_commands(DataFormat::U8(&[command::DEEP_SLEEP])).await?;
        self.interface.send_data(DataFormat::U8(&[flag::DEEP_SLEEP_MODE_1])).await?;
        Ok(())
    }

    /// Wake from deep sleep mode 1 via hardware reset (registers reset, RAM retained).
    pub async fn wake_up(&mut self) -> Result<()> {
        self.hw_reset().await;
        Ok(())
    }

    async fn use_full_frame(&mut self) -> Result<()> {
        self.use_partial_frame(0, 0, WIDTH, HEIGHT).await
    }

    async fn use_partial_frame(&mut self, x: u32, y: u32, width: u32, height: u32) -> Result<()> {
        self.set_ram_area(x, y, x + width - 1, y + height - 1).await?;
        self.set_ram_counter(x, y).await?;
        Ok(())
    }

    async fn set_ram_area(&mut self, start_x: u32, start_y: u32, end_x: u32, end_y: u32) -> Result<()> {
        assert!(start_x < end_x);
        assert!(start_y < end_y);
        self.command_with_data(command::SET_RAMXPOS, &[(start_x >> 3) as u8, (end_x >> 3) as u8]).await?;
        self.command_with_data(
            command::SET_RAMYPOS,
            &[start_y as u8, (start_y >> 8) as u8, end_y as u8, (end_y >> 8) as u8],
        )
        .await?;
        Ok(())
    }

    async fn set_ram_counter(&mut self, x: u32, y: u32) -> Result<()> {
        self.command_with_data(command::SET_RAMX_COUNTER, &[(x >> 3) as u8]).await?;
        self.command_with_data(command::SET_RAMY_COUNTER, &[y as u8, (y >> 8) as u8]).await?;
        Ok(())
    }

    async fn command(&mut self, command: u8) -> Result<()> {
        self.interface.send_commands(DataFormat::U8(&[command])).await?;
        Ok(())
    }

    async fn data(&mut self, data: &[u8]) -> Result<()> {
        self.interface.send_data(DataFormat::U8(data)).await?;
        self.wait_until_idle().await;
        Ok(())
    }

    async fn wait_until_idle(&mut self) {
        #[cfg(feature = "blocking")]
        while self.busy.is_high().unwrap_or(true) {
            self.delay.delay_ms(1)
        }
        #[cfg(not(feature = "blocking"))]
        let _ = self.busy.wait_for_low().await;
    }

    async fn command_with_data(&mut self, command: u8, data: &[u8]) -> Result<()> {
        self.command(command).await?;
        self.data(data).await?;
        Ok(())
    }

    async fn data_x_times(&mut self, data: u8, repetitions: u32) -> Result<()> {
        let mut iter = iter::repeat(data).take(repetitions as usize);
        self.interface.send_data(DataFormat::U8Iter(&mut iter)).await?;
        Ok(())
    }
}

/// Functions available only for B/W displays.
#[maybe_async_cfg::maybe(
    sync(
        feature = "blocking",
        keep_self,
        idents(AsyncWriteOnlyDataCommand(sync = "WriteOnlyDataCommand"), Wait(sync = "InputPin"))
    ),
    async(not(feature = "blocking"), keep_self)
)]
impl<DI, BSY, RST, DELAY, const WIDTH: u32, const VISIBLE_WIDTH: u32, const HEIGHT: u32>
    DisplayDriver<DI, BSY, RST, DELAY, WIDTH, VISIBLE_WIDTH, HEIGHT, Color>
where
    DI: AsyncWriteOnlyDataCommand,
    BSY: InputPin + Wait,
    RST: OutputPin,
    DELAY: DelayNs,
{
    /// Trigger a fast differential refresh (0xFC). Runs a full refresh first if needed.
    pub async fn fast_refresh(&mut self) -> Result<()> {
        if !self.initial_full_refresh_done {
            self.full_refresh().await?;
        }
        self.using_partial_mode = true;
        self.command_with_data(command::UPDATE_DISPLAY_CTRL2, &[flag::DISPLAY_MODE_2]).await?;
        self.command(command::MASTER_ACTIVATE).await?;
        self.wait_until_idle().await;
        Ok(())
    }

    /// Full-frame update using the OTP 0xF7 waveform.
    ///
    /// Sleeps the display before calling init() so the hw_reset inside init() is
    /// issued from SLEEP MODE 1. The SSD1680 spec only guarantees DTM2 RAM retention
    /// from sleep mode 1; on the DEPG0290BBS800F6HP-M7 batch hw_reset from active
    /// state clears DTM2, which causes the 0xF7 LUT to apply weak no-change waveforms
    /// and leave residual charge that produces an inverted ghost after power-off.
    ///
    /// DTM2 is NOT written before the refresh so the LUT sees the real previous-frame
    /// content and applies correct strong drive waveforms to all changed pixels.
    /// DTM2 is updated after the refresh as the basemap for the next partial cycle.
    pub async fn full_update_from_buffer(&mut self, buffer: &[u8]) -> Result<()> {
        // Sleep before hw_reset to guarantee DTM2 is preserved (spec: RAM retained in mode 1).
        // 10 ms pause lets the controller fully latch into sleep before RST goes low.
        self.sleep().await?;
        self.delay.delay_ms(10).await;
        self.init().await?;
        self.write_bw_buffer(buffer).await?;
        self.full_refresh().await?;
        self.write_red_buffer(buffer).await?;
        Ok(())
    }

    /// Full-frame fast update using the 0xFC differential waveform.
    ///
    /// No deep sleep between updates: keeping the display active avoids DTM2 corruption
    /// on the DEPG0290BBS800F6HP-M7 batch that occurs on deep sleep + hw_reset wakeup.
    /// hw_reset is still issued every frame (matches Arduino EPD_Dis_Part) to stabilise
    /// the gate driver. Registers lost at POR are restored before writing the frame.
    pub async fn fast_update_from_buffer(&mut self, buffer: &[u8]) -> Result<()> {
        self.hw_reset().await;
        self.wait_until_idle().await;
        // Restore registers reset to POR by hw_reset.
        // 0x80: border floating (partial mode).
        self.command_with_data(command::BORDER_WAVEFORM_CONTROL, &[0x80]).await?;
        // [0x00, 0x80]: source bypass for FPC-7519 colstart=8; POR shifts output 8 px.
        self.command_with_data(command::DISPLAY_UPDATE_CONTROL, &[0x00, 0x80]).await?;
        self.command_with_data(command::TEMP_CONTROL, &[flag::INTERNAL_TEMP_SENSOR]).await?;
        self.write_bw_buffer(buffer).await?;
        self.fast_refresh().await?;
        self.write_red_buffer(buffer).await?;
        Ok(())
    }

    /// Partial-region fast update. `x` and `width` must be multiples of 8.
    pub async fn fast_partial_update_from_buffer(
        &mut self,
        buffer: &[u8],
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    ) -> Result<()> {
        self.hw_reset().await;
        self.wait_until_idle().await;
        self.command_with_data(command::BORDER_WAVEFORM_CONTROL, &[0x80]).await?;
        self.write_partial_bw_buffer(buffer, x, y, width, height).await?;
        self.fast_refresh().await?;
        self.write_partial_red_buffer(buffer, x, y, width, height).await?;
        Ok(())
    }

    /// Update the display using a full refresh.
    #[cfg_attr(docsrs, doc(cfg(feature = "graphics")))]
    #[cfg(feature = "graphics")]
    pub async fn full_update<const BUFFER_SIZE: usize>(
        &mut self,
        display: &Display<WIDTH, HEIGHT, BUFFER_SIZE, Color>,
    ) -> Result<()> {
        self.full_update_from_buffer(display.buffer()).await
    }

    /// Update the display using a fast refresh.
    #[cfg_attr(docsrs, doc(cfg(feature = "graphics")))]
    #[cfg(feature = "graphics")]
    pub async fn fast_update<const BUFFER_SIZE: usize>(
        &mut self,
        display: &Display<WIDTH, HEIGHT, BUFFER_SIZE, Color>,
    ) -> Result<()> {
        self.fast_update_from_buffer(display.buffer()).await
    }

    /// Update a partial region using a fast refresh. `x` and display width `W` must be multiples of 8.
    #[cfg_attr(docsrs, doc(cfg(feature = "graphics")))]
    #[cfg(feature = "graphics")]
    pub async fn fast_partial_update<const W: u32, const H: u32, const BUFFER_SIZE: usize>(
        &mut self,
        display: &Display<W, H, BUFFER_SIZE, Color>,
        x: u32,
        y: u32,
    ) -> Result<()> {
        self.fast_partial_update_from_buffer(display.buffer(), x, y, W, H).await
    }
}

/// Functions available only for tri-color displays.
#[maybe_async_cfg::maybe(
    sync(
        feature = "blocking",
        keep_self,
        idents(AsyncWriteOnlyDataCommand(sync = "WriteOnlyDataCommand"), Wait(sync = "InputPin"))
    ),
    async(not(feature = "blocking"), keep_self)
)]
impl<DI, BSY, RST, DELAY, const WIDTH: u32, const VISIBLE_WIDTH: u32, const HEIGHT: u32>
    DisplayDriver<DI, BSY, RST, DELAY, WIDTH, VISIBLE_WIDTH, HEIGHT, TriColor>
where
    DI: AsyncWriteOnlyDataCommand,
    BSY: InputPin + Wait,
    RST: OutputPin,
    DELAY: DelayNs,
{
    /// Full-frame update using the OTP 0xF7 waveform.
    pub async fn full_update_from_buffer(&mut self, bw_buffer: &[u8], red_buffer: &[u8]) -> Result<()> {
        self.write_red_buffer(red_buffer).await?;
        self.write_bw_buffer(bw_buffer).await?;
        self.full_refresh().await?;
        Ok(())
    }

    /// Update the display using a full refresh.
    #[cfg_attr(docsrs, doc(cfg(feature = "graphics")))]
    #[cfg(feature = "graphics")]
    pub async fn full_update<const BUFFER_SIZE: usize>(
        &mut self,
        display: &Display<WIDTH, HEIGHT, BUFFER_SIZE, TriColor>,
    ) -> Result<()> {
        self.full_update_from_buffer(display.bw_buffer(), display.red_buffer()).await
    }
}
