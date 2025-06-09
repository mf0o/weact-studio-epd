#[allow(unused)]
mod commands {

    pub const DRIVER_CONTROL: u8 = 0x01;
    pub const SET_SOFTSTART: u8 = 0x0C;
    pub const DATA_ENTRY_MODE: u8 = 0x11;
    pub const SW_RESET: u8 = 0x12;
    pub const TEMP_CONTROL: u8 = 0x18;
    pub const DISPLAY_UPDATE_CONTROL: u8 = 0x21;
    pub const UPDATE_DISPLAY_CTRL2: u8 = 0x22;
    pub const WRITE_BW_DATA: u8 = 0x24;
    pub const WRITE_RED_DATA: u8 = 0x26;
    pub const WRITE_VCOM: u8 = 0x2C;
    pub const WRITE_LUT: u8 = 0x32;
    pub const SET_RAMXPOS: u8 = 0x44;
    pub const SET_RAMYPOS: u8 = 0x45;
    pub const SET_RAMX_COUNTER: u8 = 0x4E;
    pub const SET_RAMY_COUNTER: u8 = 0x4F;
    pub const NOP: u8 = 0xFF;



    // Power and Booster
    pub const POWER_SETTING: u8 = 0x01;
    pub const POWER_OFF: u8 = 0x02;
    pub const POWER_OFF_SEQUENCE: u8 = 0x03;
    pub const POWER_ON: u8 = 0x04;
    pub const BOOSTER_SOFT_START: u8 = 0x0C;

    // Panel/Display Settings
    pub const PANEL_SETTING: u8 = 0x00;
    pub const VCOM_AND_DATA_INTERVAL_SETTING: u8 = 0x50;
    // pub const VCM_DC_SETTING: u8 = 0x82;
    pub const DISPLAY_UPDATE_CONTROL_1: u8 = 0x21;
    pub const DISPLAY_UPDATE_CONTROL_2: u8 = 0x22;
    
    pub const MASTER_ACTIVATE: u8 = 0x20;
    pub const MASTER_ACTIVATION: u8 = 0x20;

    pub const PARTIAL_IN: u8 = 0x91;
    pub const PARTIAL_OUT: u8 = 0x92;

    // Resolution & addressing
    pub const TCON_RESOLUTION: u8 = 0x61;
    pub const VCOM_LUT: u8 = 0x20;
    pub const LUT_FOR_VCOM: u8 = 0x32; // Load LUT

    // RAM
    pub const DATA_START_TRANSMISSION_1: u8 = 0x10; // Black
    pub const DATA_START_TRANSMISSION_2: u8 = 0x13; // Red or overlay
    pub const WRITE_RAM: u8 = 0x24;
    pub const WRITE_RAM_RED: u8 = 0x26;

    // Other
    pub const DEEP_SLEEP: u8 = 0x10;
    pub const PLL_CONTROL: u8 = 0x30;
    pub const TEMPERATURE_SENSOR_CONTROL: u8 = 0x18;
    pub const BORDER_WAVEFORM_CONTROL: u8 = 0x3C;
    pub const SET_RAM_X_ADDRESS: u8 = 0x44;
    pub const SET_RAM_Y_ADDRESS: u8 = 0x45;
    pub const SET_RAM_X_COUNTER: u8 = 0x4E;
    pub const SET_RAM_Y_COUNTER: u8 = 0x4F;    


    pub const GATE_DRIVING_VOLTAGE: u8 = 0x03;
    pub const SOURCE_DRIVING_VOLTAGE: u8 = 0x04;

    // 0x20 - Master activation (trigger screen update)
    // MASTER_ACTIVATION = 0x20,


    // 0x2C - VCOM DC setting 
    // this was 0x82 previously
    pub const VCM_DC_SETTING: u8 = 0x2C;


}

pub(crate) use commands::*;
