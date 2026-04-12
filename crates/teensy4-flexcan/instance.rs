use core::ptr;

use imxrt_ral as ral;

/// A concrete FlexCAN peripheral instance.
///
/// This trait is implemented for the three Teensy 4 FlexCAN peripherals exposed
/// by `imxrt-ral`. It provides the register base address, bus number, and clock
/// gate enable routine required by [`crate::FlexCan`].
pub trait Instance: Sized {
    /// Logical CAN bus number used in received messages.
    const BUS_NUMBER: u8;

    /// Returns the peripheral register block base address.
    fn base() -> usize;
    /// Enables the peripheral clock gate.
    fn enable_clock();
}

impl Instance for ral::can::CAN1 {
    const BUS_NUMBER: u8 = 1;

    fn base() -> usize {
        ral::can::CAN1 as usize
    }

    fn enable_clock() {
        unsafe {
            let ccm = ral::ccm::CCM as *mut u32;
            let ccgr0 = ccm.add(0x68 / 4);
            ptr::write_volatile(ccgr0, ptr::read_volatile(ccgr0) | 0x3C000);
        }
    }
}

impl Instance for ral::can::CAN2 {
    const BUS_NUMBER: u8 = 2;

    fn base() -> usize {
        ral::can::CAN2 as usize
    }

    fn enable_clock() {
        unsafe {
            let ccm = ral::ccm::CCM as *mut u32;
            let ccgr0 = ccm.add(0x68 / 4);
            ptr::write_volatile(ccgr0, ptr::read_volatile(ccgr0) | 0x3C0000);
        }
    }
}

impl Instance for ral::can3::CAN3 {
    const BUS_NUMBER: u8 = 3;

    fn base() -> usize {
        ral::can3::CAN3 as usize
    }

    fn enable_clock() {
        let ccm = unsafe { ral::ccm::CCM::instance() };
        ral::modify_reg!(ral::ccm, ccm, CCGR7, CG3: 0b11, CG4: 0b11);
    }
}
