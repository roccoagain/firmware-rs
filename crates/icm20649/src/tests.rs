use core::cell::RefCell;
use core::convert::Infallible;

use embedded_hal::i2c::{ErrorType, I2c, Operation};

use crate::registers::{
    ACCEL_CONFIG_1, ACCEL_SMPLRT_DIV_1, CHIP_ID, DEFAULT_ADDRESS, GYRO_CONFIG_1, GYRO_SMPLRT_DIV,
    PWR_MGMT_1, REG_BANK_SEL, RESET_BIT, SLEEP_BIT, WHO_AM_I,
};
use crate::{AccelRange, GyroDlpf, GyroRange, Icm20649, RawSample, Sample};

struct MockI2c {
    ops: RefCell<&'static [Expectation]>,
}

#[derive(Clone, Copy)]
enum Expectation {
    Write(&'static [u8]),
    WriteRead(&'static [u8], &'static [u8]),
}

impl MockI2c {
    fn new(ops: &'static [Expectation]) -> Self {
        Self {
            ops: RefCell::new(ops),
        }
    }

    fn next(&self) -> Expectation {
        let mut ops = self.ops.borrow_mut();
        let (next, rest) = ops.split_first().expect("unexpected I2C operation");
        *ops = rest;
        *next
    }

    fn done(&self) {
        assert!(self.ops.borrow().is_empty(), "unused I2C expectations remain");
    }
}

impl ErrorType for MockI2c {
    type Error = Infallible;
}

impl I2c for MockI2c {
    fn read(&mut self, _address: u8, _read: &mut [u8]) -> Result<(), Self::Error> {
        unreachable!("driver only uses write_read")
    }

    fn write(&mut self, address: u8, write: &[u8]) -> Result<(), Self::Error> {
        assert_eq!(address, DEFAULT_ADDRESS);
        match self.next() {
            Expectation::Write(expected) => assert_eq!(write, expected),
            Expectation::WriteRead(_, _) => panic!("expected write_read, got write"),
        }
        Ok(())
    }

    fn write_read(
        &mut self,
        address: u8,
        write: &[u8],
        read: &mut [u8],
    ) -> Result<(), Self::Error> {
        assert_eq!(address, DEFAULT_ADDRESS);
        match self.next() {
            Expectation::WriteRead(expected_write, expected_read) => {
                assert_eq!(write, expected_write);
                read.copy_from_slice(expected_read);
            }
            Expectation::Write(_) => panic!("expected write, got write_read"),
        }
        Ok(())
    }

    fn transaction(
        &mut self,
        _address: u8,
        _operations: &mut [Operation<'_>],
    ) -> Result<(), Self::Error> {
        unreachable!("driver does not use transactions")
    }
}

#[test]
fn sample_scaling_matches_adafruit_constants() {
    let sample = Sample::from_raw(
        RawSample {
            accel_x: 1024,
            accel_y: -1024,
            accel_z: 2048,
            gyro_x: 82,
            gyro_y: -164,
            gyro_z: 328,
            temperature: 334,
        },
        AccelRange::G30,
        GyroRange::Dps4000,
    );

    assert_eq!(sample.accel_g, [1.0, -1.0, 2.0]);
    assert_eq!(sample.gyro_dps, [10.0, -20.0, 40.0]);
    assert!((sample.temperature_c - 22.00039).abs() < 0.001);
    assert!((sample.gyro_rads[0] - 0.17453292).abs() < 0.000001);
}

#[test]
fn init_verifies_id_and_programs_defaults() {
    let expectations = &[
        Expectation::WriteRead(&[WHO_AM_I], &[CHIP_ID]),
        Expectation::Write(&[PWR_MGMT_1, RESET_BIT]),
        Expectation::WriteRead(&[PWR_MGMT_1], &[0]),
        Expectation::WriteRead(&[PWR_MGMT_1], &[SLEEP_BIT]),
        Expectation::Write(&[PWR_MGMT_1, 0]),
        Expectation::Write(&[REG_BANK_SEL, 0x20]),
        Expectation::WriteRead(&[GYRO_CONFIG_1], &[0]),
        Expectation::Write(&[GYRO_CONFIG_1, 0b110]),
        Expectation::WriteRead(&[ACCEL_CONFIG_1], &[0]),
        Expectation::Write(&[ACCEL_CONFIG_1, 0b110]),
        Expectation::Write(&[GYRO_SMPLRT_DIV, 10]),
        Expectation::Write(&[ACCEL_SMPLRT_DIV_1, 0, 20]),
    ];

    let mut sensor = Icm20649::new(MockI2c::new(expectations));
    sensor.init().unwrap();
    sensor.free().done();
}

#[test]
fn gyro_dlpf_uses_gyro_config_register() {
    let expectations = &[
        Expectation::Write(&[REG_BANK_SEL, 0x20]),
        Expectation::WriteRead(&[GYRO_CONFIG_1], &[0]),
        Expectation::Write(&[GYRO_CONFIG_1, 0b1 | (GyroDlpf::Hz51_2 as u8) << 3]),
    ];

    let mut sensor = Icm20649::new(MockI2c::new(expectations));
    sensor.enable_gyro_dlpf(true, GyroDlpf::Hz51_2).unwrap();
    sensor.free().done();
}
