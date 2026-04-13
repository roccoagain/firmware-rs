/// Accelerometer full-scale range.
///
/// These map directly to the two range bits in `ACCEL_CONFIG_1`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum AccelRange {
    /// +/-4 g
    G4 = 0,
    /// +/-8 g
    G8 = 1,
    /// +/-16 g
    G16 = 2,
    /// +/-30 g
    G30 = 3,
}

impl AccelRange {
    pub(crate) fn from_bits(bits: u8) -> Self {
        match bits & 0b11 {
            0 => Self::G4,
            1 => Self::G8,
            2 => Self::G16,
            _ => Self::G30,
        }
    }

    pub(crate) fn lsb_per_g(self) -> f32 {
        match self {
            Self::G4 => 8192.0,
            Self::G8 => 4096.0,
            Self::G16 => 2048.0,
            Self::G30 => 1024.0,
        }
    }
}

/// Gyroscope full-scale range.
///
/// These map directly to the two range bits in `GYRO_CONFIG_1`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum GyroRange {
    /// +/-500 dps
    Dps500 = 0,
    /// +/-1000 dps
    Dps1000 = 1,
    /// +/-2000 dps
    Dps2000 = 2,
    /// +/-4000 dps
    Dps4000 = 3,
}

impl GyroRange {
    pub(crate) fn from_bits(bits: u8) -> Self {
        match bits & 0b11 {
            0 => Self::Dps500,
            1 => Self::Dps1000,
            2 => Self::Dps2000,
            _ => Self::Dps4000,
        }
    }

    pub(crate) fn lsb_per_dps(self) -> f32 {
        match self {
            Self::Dps500 => 65.5,
            Self::Dps1000 => 32.8,
            Self::Dps2000 => 16.4,
            Self::Dps4000 => 8.2,
        }
    }
}

/// Accelerometer DLPF cutoff selection.
///
/// These values map directly to the accelerometer DLPF configuration bits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum AccelDlpf {
    /// 246.0 Hz
    Hz246_0 = 0x1,
    /// 111.4 Hz
    Hz111_4 = 0x2,
    /// 50.4 Hz
    Hz50_4 = 0x3,
    /// 23.9 Hz
    Hz23_9 = 0x4,
    /// 11.5 Hz
    Hz11_5 = 0x5,
    /// 5.7 Hz
    Hz5_7 = 0x6,
    /// 473 Hz
    Hz473 = 0x7,
}

/// Gyroscope DLPF cutoff selection.
///
/// These values map directly to the gyroscope DLPF configuration bits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum GyroDlpf {
    /// 196.6 Hz
    Hz196_6 = 0x0,
    /// 151.8 Hz
    Hz151_8 = 0x1,
    /// 119.5 Hz
    Hz119_5 = 0x2,
    /// 51.2 Hz
    Hz51_2 = 0x3,
    /// 23.9 Hz
    Hz23_9 = 0x4,
    /// 11.6 Hz
    Hz11_6 = 0x5,
    /// 5.7 Hz
    Hz5_7 = 0x6,
    /// 361.4 Hz
    Hz361_4 = 0x7,
}
