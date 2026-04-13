pub const DEFAULT_ADDRESS: u8 = 0x68;

pub const WHO_AM_I: u8 = 0x00;
pub const USER_CTRL: u8 = 0x03;
pub const PWR_MGMT_1: u8 = 0x06;
pub const INT_PIN_CFG: u8 = 0x0F;
pub const REG_BANK_SEL: u8 = 0x7F;
pub const ACCEL_XOUT_H: u8 = 0x2D;

pub const GYRO_SMPLRT_DIV: u8 = 0x00;
pub const GYRO_CONFIG_1: u8 = 0x01;
pub const ACCEL_SMPLRT_DIV_1: u8 = 0x10;
pub const ACCEL_CONFIG_1: u8 = 0x14;

pub const CHIP_ID: u8 = 0xE1;
pub const RESET_BIT: u8 = 1 << 7;
pub const SLEEP_BIT: u8 = 1 << 6;
pub const I2C_MST_EN_BIT: u8 = 1 << 5;
pub const I2C_BYPASS_EN_BIT: u8 = 1 << 1;
