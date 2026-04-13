//! I2C driver for the TDK InvenSense ICM-20649 6-DoF IMU.
//!
//! This crate is a focused Rust port of the Adafruit ICM20X driver for the
//! ICM-20649 only. It exposes the device bring-up flow, range and filter
//! configuration, sample-rate divisors, and typed sensor readouts over
//! `embedded-hal` 1.0 I2C.
//!
//! The crate is intentionally small:
//!
//! - I2C transport only
//! - ICM-20649 only
//! - typed accel / gyro range and DLPF settings
//! - raw and scaled sensor reads
//! - direct register access when you need to go off the paved path
//!
//! `Icm20649::init()` follows the defaults from the Adafruit driver this was
//! ported from:
//!
//! - verifies `WHO_AM_I == 0xE1`
//! - resets the device
//! - exits sleep mode
//! - sets gyro range to `GyroRange::Dps4000`
//! - sets accel range to `AccelRange::G30`
//! - sets gyro sample-rate divisor to `10`
//! - sets accel sample-rate divisor to `20`
//!
//! Typical usage looks like this:
//!
//! ```ignore
//! use icm20649::{AccelRange, GyroRange, Icm20649};
//!
//! let i2c = /* your embedded-hal 1.0 I2C peripheral */;
//! let mut imu = Icm20649::new(i2c);
//!
//! imu.init()?;
//! imu.set_accel_range(AccelRange::G16)?;
//! imu.set_gyro_range(GyroRange::Dps2000)?;
//!
//! let sample = imu.read_sample()?;
//! let ax_mps2 = sample.accel_mps2[0];
//! let gz_rads = sample.gyro_rads[2];
//! # Ok::<(), icm20649::Error<core::convert::Infallible>>(())
//! ```
//!
//! Scaled samples expose both engineering units and the intermediate units used
//! by the Adafruit driver:
//!
//! - acceleration in `g` and `m/s^2`
//! - angular rate in `deg/s` and `rad/s`
//! - temperature in `deg C`

#![no_std]
#![warn(missing_docs)]

mod driver;
mod registers;
mod sample;
#[cfg(test)]
mod tests;
mod types;

pub use driver::{Error, Icm20649};
pub use sample::{RawSample, Sample};
pub use types::{AccelDlpf, AccelRange, GyroDlpf, GyroRange};
