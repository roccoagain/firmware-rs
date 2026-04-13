use embedded_hal::i2c::I2c;

use crate::registers::{
    ACCEL_CONFIG_1, ACCEL_SMPLRT_DIV_1, ACCEL_XOUT_H, CHIP_ID, DEFAULT_ADDRESS, GYRO_CONFIG_1,
    GYRO_SMPLRT_DIV, I2C_BYPASS_EN_BIT, I2C_MST_EN_BIT, INT_PIN_CFG, PWR_MGMT_1, REG_BANK_SEL,
    RESET_BIT, SLEEP_BIT, USER_CTRL, WHO_AM_I,
};
use crate::{
    sample::{RawSample, Sample},
    types::{AccelDlpf, AccelRange, GyroDlpf, GyroRange},
};

/// Driver error returned by [`Icm20649`].
#[derive(Debug, PartialEq, Eq)]
pub enum Error<E> {
    /// Underlying I2C transport error.
    I2c(E),
    /// WHO_AM_I returned an unexpected value.
    InvalidChipId(u8),
}

/// ICM-20649 I2C driver.
///
/// The driver caches the currently selected accel and gyro ranges locally so
/// [`read_sample`](Self::read_sample) can scale raw register values without
/// rereading the configuration registers on every sample.
pub struct Icm20649<I2C> {
    i2c: I2C,
    address: u8,
    bank: u8,
    accel_range: AccelRange,
    gyro_range: GyroRange,
}

impl<I2C, E> Icm20649<I2C>
where
    I2C: I2c<Error = E>,
{
    /// Create a driver with the default I2C address (`0x68`).
    pub fn new(i2c: I2C) -> Self {
        Self::new_with_address(i2c, DEFAULT_ADDRESS)
    }

    /// Create a driver with an explicit 7-bit I2C address.
    ///
    /// This only constructs the driver. Call [`init`](Self::init) before using
    /// the sensor.
    pub fn new_with_address(i2c: I2C, address: u8) -> Self {
        Self {
            i2c,
            address,
            bank: 0,
            accel_range: AccelRange::G30,
            gyro_range: GyroRange::Dps4000,
        }
    }

    /// Initialize the sensor using the same defaults as Adafruit's driver.
    ///
    /// This validates the chip ID, performs a device reset, exits sleep mode,
    /// and programs default ranges and sample-rate divisors.
    pub fn init(&mut self) -> Result<(), Error<E>> {
        self.set_bank(0)?;
        let chip_id = self.read_reg(0, WHO_AM_I)?;
        if chip_id != CHIP_ID {
            return Err(Error::InvalidChipId(chip_id));
        }

        self.reset()?;
        self.update_reg(0, PWR_MGMT_1, |value| value & !SLEEP_BIT)?;
        self.set_gyro_range(GyroRange::Dps4000)?;
        self.set_accel_range(AccelRange::G30)?;
        self.set_gyro_rate_divisor(10)?;
        self.set_accel_rate_divisor(20)?;
        Ok(())
    }

    /// Reset the device and wait until the reset bit clears.
    ///
    /// After reset, the cached bank is set back to bank 0.
    pub fn reset(&mut self) -> Result<(), Error<E>> {
        self.write_reg(0, PWR_MGMT_1, RESET_BIT)?;
        while self.read_reg(0, PWR_MGMT_1)? & RESET_BIT != 0 {}
        self.bank = 0;
        Ok(())
    }

    /// Return the cached accelerometer range.
    ///
    /// This does not touch the bus. Use [`read_accel_range`](Self::read_accel_range)
    /// to fetch the value from hardware.
    pub fn accel_range(&self) -> AccelRange {
        self.accel_range
    }

    /// Return the cached gyroscope range.
    ///
    /// This does not touch the bus. Use [`read_gyro_range`](Self::read_gyro_range)
    /// to fetch the value from hardware.
    pub fn gyro_range(&self) -> GyroRange {
        self.gyro_range
    }

    /// Read the configured accelerometer range from hardware.
    ///
    /// The cached range used by [`read_sample`](Self::read_sample) is updated to
    /// match the returned value.
    pub fn read_accel_range(&mut self) -> Result<AccelRange, Error<E>> {
        let value = self.read_reg(2, ACCEL_CONFIG_1)?;
        let range = AccelRange::from_bits((value >> 1) & 0b11);
        self.accel_range = range;
        Ok(range)
    }

    /// Set the accelerometer full-scale range.
    ///
    /// The cached range used for scaling is updated immediately.
    pub fn set_accel_range(&mut self, range: AccelRange) -> Result<(), Error<E>> {
        self.update_reg(2, ACCEL_CONFIG_1, |value| {
            (value & !(0b11 << 1)) | ((range as u8) << 1)
        })?;
        self.accel_range = range;
        Ok(())
    }

    /// Read the configured gyroscope range from hardware.
    ///
    /// The cached range used by [`read_sample`](Self::read_sample) is updated to
    /// match the returned value.
    pub fn read_gyro_range(&mut self) -> Result<GyroRange, Error<E>> {
        let value = self.read_reg(2, GYRO_CONFIG_1)?;
        let range = GyroRange::from_bits((value >> 1) & 0b11);
        self.gyro_range = range;
        Ok(range)
    }

    /// Set the gyroscope full-scale range.
    ///
    /// The cached range used for scaling is updated immediately.
    pub fn set_gyro_range(&mut self, range: GyroRange) -> Result<(), Error<E>> {
        self.update_reg(2, GYRO_CONFIG_1, |value| {
            (value & !(0b11 << 1)) | ((range as u8) << 1)
        })?;
        self.gyro_range = range;
        Ok(())
    }

    /// Read the accelerometer sample-rate divisor.
    ///
    /// The raw register value is returned as a 16-bit big-endian integer. Only
    /// the lower 12 bits are meaningful on the device.
    pub fn accel_rate_divisor(&mut self) -> Result<u16, Error<E>> {
        let mut bytes = [0; 2];
        self.read_many(2, ACCEL_SMPLRT_DIV_1, &mut bytes)?;
        Ok(u16::from_be_bytes(bytes))
    }

    /// Set the accelerometer sample-rate divisor.
    ///
    /// Only the lower 12 bits are written to the device.
    pub fn set_accel_rate_divisor(&mut self, divisor: u16) -> Result<(), Error<E>> {
        let bytes = (divisor & 0x0FFF).to_be_bytes();
        self.write_many(2, ACCEL_SMPLRT_DIV_1, &bytes)
    }

    /// Read the gyroscope sample-rate divisor.
    pub fn gyro_rate_divisor(&mut self) -> Result<u8, Error<E>> {
        self.read_reg(2, GYRO_SMPLRT_DIV)
    }

    /// Set the gyroscope sample-rate divisor.
    pub fn set_gyro_rate_divisor(&mut self, divisor: u8) -> Result<(), Error<E>> {
        self.write_reg(2, GYRO_SMPLRT_DIV, divisor)
    }

    /// Enable or disable accelerometer DLPF.
    ///
    /// When `enabled` is `true`, `cutoff` is written into the filter selection
    /// bits. When `enabled` is `false`, only the enable bit is cleared.
    pub fn enable_accel_dlpf(
        &mut self,
        enabled: bool,
        cutoff: AccelDlpf,
    ) -> Result<(), Error<E>> {
        self.configure_dlpf(2, ACCEL_CONFIG_1, enabled, cutoff as u8)
    }

    /// Enable or disable gyroscope DLPF.
    ///
    /// When `enabled` is `true`, `cutoff` is written into the filter selection
    /// bits. When `enabled` is `false`, only the enable bit is cleared.
    pub fn enable_gyro_dlpf(
        &mut self,
        enabled: bool,
        cutoff: GyroDlpf,
    ) -> Result<(), Error<E>> {
        self.configure_dlpf(2, GYRO_CONFIG_1, enabled, cutoff as u8)
    }

    /// Enable or disable the auxiliary I2C master.
    ///
    /// This controls the internal auxiliary bus master used for external
    /// sensors connected through the IMU.
    pub fn enable_i2c_master(&mut self, enabled: bool) -> Result<(), Error<E>> {
        self.update_reg(0, USER_CTRL, |value| {
            if enabled {
                value | I2C_MST_EN_BIT
            } else {
                value & !I2C_MST_EN_BIT
            }
        })
    }

    /// Enable or disable I2C bypass mode.
    ///
    /// Bypass mode connects the external pins directly to the primary I2C bus
    /// instead of routing them through the IMU's auxiliary master.
    pub fn set_i2c_bypass(&mut self, enabled: bool) -> Result<(), Error<E>> {
        self.update_reg(0, INT_PIN_CFG, |value| {
            if enabled {
                value | I2C_BYPASS_EN_BIT
            } else {
                value & !I2C_BYPASS_EN_BIT
            }
        })
    }

    /// Read one raw accel / gyro / temperature sample.
    ///
    /// This performs a single 14-byte burst read starting at `ACCEL_XOUT_H`.
    pub fn read_raw_sample(&mut self) -> Result<RawSample, Error<E>> {
        let mut buffer = [0; 14];
        self.read_many(0, ACCEL_XOUT_H, &mut buffer)?;

        Ok(RawSample {
            accel_x: i16::from_be_bytes([buffer[0], buffer[1]]),
            accel_y: i16::from_be_bytes([buffer[2], buffer[3]]),
            accel_z: i16::from_be_bytes([buffer[4], buffer[5]]),
            gyro_x: i16::from_be_bytes([buffer[6], buffer[7]]),
            gyro_y: i16::from_be_bytes([buffer[8], buffer[9]]),
            gyro_z: i16::from_be_bytes([buffer[10], buffer[11]]),
            temperature: i16::from_be_bytes([buffer[12], buffer[13]]),
        })
    }

    /// Read and scale one sample using the current cached ranges.
    ///
    /// If the sensor range may have been changed externally, call
    /// [`read_accel_range`](Self::read_accel_range) and
    /// [`read_gyro_range`](Self::read_gyro_range) first to refresh the cache.
    pub fn read_sample(&mut self) -> Result<Sample, Error<E>> {
        let raw = self.read_raw_sample()?;
        Ok(Sample::from_raw(raw, self.accel_range, self.gyro_range))
    }

    /// Read a single register from the selected bank.
    ///
    /// This is intended as an escape hatch for device features not wrapped by
    /// the typed API.
    pub fn read_register(&mut self, bank: u8, register: u8) -> Result<u8, Error<E>> {
        self.read_reg(bank, register)
    }

    /// Write a single register in the selected bank.
    ///
    /// This is intended as an escape hatch for device features not wrapped by
    /// the typed API.
    pub fn write_register(&mut self, bank: u8, register: u8, value: u8) -> Result<(), Error<E>> {
        self.write_reg(bank, register, value)
    }

    /// Return ownership of the underlying I2C peripheral.
    pub fn free(self) -> I2C {
        self.i2c
    }

    fn configure_dlpf(
        &mut self,
        bank: u8,
        register: u8,
        enabled: bool,
        cutoff_bits: u8,
    ) -> Result<(), Error<E>> {
        self.update_reg(bank, register, |value| {
            let with_enable = if enabled { value | 0x01 } else { value & !0x01 };
            if enabled {
                (with_enable & !(0b111 << 3)) | ((cutoff_bits & 0b111) << 3)
            } else {
                with_enable
            }
        })
    }

    fn read_reg(&mut self, bank: u8, register: u8) -> Result<u8, Error<E>> {
        let mut value = [0];
        self.read_many(bank, register, &mut value)?;
        Ok(value[0])
    }

    fn write_reg(&mut self, bank: u8, register: u8, value: u8) -> Result<(), Error<E>> {
        self.set_bank(bank)?;
        self.i2c
            .write(self.address, &[register, value])
            .map_err(Error::I2c)
    }

    fn read_many(&mut self, bank: u8, register: u8, buffer: &mut [u8]) -> Result<(), Error<E>> {
        self.set_bank(bank)?;
        self.i2c
            .write_read(self.address, &[register], buffer)
            .map_err(Error::I2c)
    }

    fn write_many(&mut self, bank: u8, register: u8, values: &[u8]) -> Result<(), Error<E>> {
        self.set_bank(bank)?;
        let mut buffer = [0u8; 3];
        buffer[0] = register;
        buffer[1..=values.len()].copy_from_slice(values);
        self.i2c.write(self.address, &buffer[..=values.len()]).map_err(Error::I2c)
    }

    fn update_reg<F>(&mut self, bank: u8, register: u8, update: F) -> Result<(), Error<E>>
    where
        F: FnOnce(u8) -> u8,
    {
        let value = self.read_reg(bank, register)?;
        self.write_reg(bank, register, update(value))
    }

    fn set_bank(&mut self, bank: u8) -> Result<(), Error<E>> {
        let bank = bank & 0b11;
        if self.bank == bank {
            return Ok(());
        }

        self.i2c
            .write(self.address, &[REG_BANK_SEL, bank << 4])
            .map_err(Error::I2c)?;
        self.bank = bank;
        Ok(())
    }
}
