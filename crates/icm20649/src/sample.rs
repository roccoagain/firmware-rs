use crate::types::{AccelRange, GyroRange};

const GRAVITY_M_S2: f32 = 9.80665;
const DEG_TO_RAD: f32 = core::f32::consts::PI / 180.0;

/// Raw sensor sample straight from the device registers.
///
/// Values are unscaled signed register readings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RawSample {
    /// Raw accelerometer X axis.
    pub accel_x: i16,
    /// Raw accelerometer Y axis.
    pub accel_y: i16,
    /// Raw accelerometer Z axis.
    pub accel_z: i16,
    /// Raw gyroscope X axis.
    pub gyro_x: i16,
    /// Raw gyroscope Y axis.
    pub gyro_y: i16,
    /// Raw gyroscope Z axis.
    pub gyro_z: i16,
    /// Raw temperature reading.
    pub temperature: i16,
}

/// Scaled sensor sample.
///
/// The same sample is exposed in both the native engineering units typically
/// consumed by robotics code (`m/s^2`, `rad/s`) and the intermediate units used
/// by many IMU datasheets and Arduino libraries (`g`, `deg/s`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sample {
    /// Acceleration in g.
    pub accel_g: [f32; 3],
    /// Acceleration in m/s^2.
    pub accel_mps2: [f32; 3],
    /// Angular rate in deg/s.
    pub gyro_dps: [f32; 3],
    /// Angular rate in rad/s.
    pub gyro_rads: [f32; 3],
    /// Temperature in Celsius.
    pub temperature_c: f32,
}

impl Sample {
    /// Scale a raw sample according to the provided ranges.
    ///
    /// Temperature conversion uses the same formula as the Adafruit driver:
    /// `raw / 333.87 + 21.0`.
    pub fn from_raw(raw: RawSample, accel_range: AccelRange, gyro_range: GyroRange) -> Self {
        let accel_scale = accel_range.lsb_per_g();
        let gyro_scale = gyro_range.lsb_per_dps();

        let accel_g = [
            raw.accel_x as f32 / accel_scale,
            raw.accel_y as f32 / accel_scale,
            raw.accel_z as f32 / accel_scale,
        ];
        let gyro_dps = [
            raw.gyro_x as f32 / gyro_scale,
            raw.gyro_y as f32 / gyro_scale,
            raw.gyro_z as f32 / gyro_scale,
        ];

        Self {
            accel_mps2: accel_g.map(|value| value * GRAVITY_M_S2),
            gyro_rads: gyro_dps.map(|value| value * DEG_TO_RAD),
            temperature_c: raw.temperature as f32 / 333.87 + 21.0,
            accel_g,
            gyro_dps,
        }
    }
}
