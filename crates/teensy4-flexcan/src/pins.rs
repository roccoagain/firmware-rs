use core::ptr;

use imxrt_iomuxc::{
    self as iomuxc, ErasedPad, Iomuxc,
    consts::{U1, U2},
    flexcan::{Pin as FlexCanPin, Rx as FlexCanRx, Tx as FlexCanTx},
    imxrt1060::{
        gpio_ad_b0::{GPIO_AD_B0_11, GPIO_AD_B0_14, GPIO_AD_B0_15},
        gpio_emc::{GPIO_EMC_36, GPIO_EMC_37},
    },
};
use imxrt_ral as ral;

use crate::{controller::FlexCan, registers::CAN_PAD_CTL};

impl FlexCan<ral::can::CAN1> {
    /// Creates `CAN1` from an owned peripheral instance and compatible pads.
    pub fn from_instance<TX, RX>(instance: ral::can::CAN1, mut tx: TX, mut rx: RX) -> Self
    where
        TX: FlexCanPin<Signal = FlexCanTx, Module = U1>,
        RX: FlexCanPin<Signal = FlexCanRx, Module = U1>,
    {
        iomuxc::flexcan::prepare(&mut tx);
        iomuxc::flexcan::prepare(&mut rx);
        set_can_pad_config(&mut tx);
        set_can_pad_config(&mut rx);
        Self::bring_up(instance)
    }

    /// Creates `CAN1` from the singleton RAL instance and compatible pads.
    ///
    /// # Safety
    ///
    /// The caller must ensure the `CAN1` peripheral is not already in use and
    /// that taking the singleton register block is sound.
    pub unsafe fn new<TX, RX>(tx: TX, rx: RX) -> Self
    where
        TX: FlexCanPin<Signal = FlexCanTx, Module = U1>,
        RX: FlexCanPin<Signal = FlexCanRx, Module = U1>,
    {
        Self::from_instance(unsafe { ral::can::CAN1::instance() }, tx, rx)
    }
}

impl FlexCan<ral::can::CAN2> {
    /// Creates `CAN2` from an owned peripheral instance and compatible pads.
    pub fn from_instance<TX, RX>(instance: ral::can::CAN2, mut tx: TX, mut rx: RX) -> Self
    where
        TX: FlexCanPin<Signal = FlexCanTx, Module = U2>,
        RX: FlexCanPin<Signal = FlexCanRx, Module = U2>,
    {
        iomuxc::flexcan::prepare(&mut tx);
        iomuxc::flexcan::prepare(&mut rx);
        set_can_pad_config(&mut tx);
        set_can_pad_config(&mut rx);
        Self::bring_up(instance)
    }

    /// Creates `CAN2` from the singleton RAL instance and compatible pads.
    ///
    /// # Safety
    ///
    /// The caller must ensure the `CAN2` peripheral is not already in use and
    /// that taking the singleton register block is sound.
    pub unsafe fn new<TX, RX>(tx: TX, rx: RX) -> Self
    where
        TX: FlexCanPin<Signal = FlexCanTx, Module = U2>,
        RX: FlexCanPin<Signal = FlexCanRx, Module = U2>,
    {
        Self::from_instance(unsafe { ral::can::CAN2::instance() }, tx, rx)
    }
}

impl FlexCan<ral::can3::CAN3> {
    /// Creates `CAN3` from its default Teensy 4 pin pair.
    pub fn from_default_pins(
        instance: ral::can3::CAN3,
        mut tx: GPIO_EMC_36,
        mut rx: GPIO_EMC_37,
    ) -> Self {
        prepare_can3_tx(&mut tx, 9);
        prepare_can3_rx(&mut rx, 9, 0);
        Self::bring_up(instance)
    }

    /// Creates `CAN3` from the first alternate Teensy 4 pin pair.
    pub fn from_alt_pins(
        instance: ral::can3::CAN3,
        mut tx: GPIO_AD_B0_14,
        mut rx: GPIO_AD_B0_15,
    ) -> Self {
        prepare_can3_tx(&mut tx, 8);
        prepare_can3_rx(&mut rx, 8, 1);
        Self::bring_up(instance)
    }

    /// Creates `CAN3` from the second alternate Teensy 4 pin pair.
    pub fn from_alt2_pins(
        instance: ral::can3::CAN3,
        mut tx: GPIO_AD_B0_14,
        mut rx: GPIO_AD_B0_11,
    ) -> Self {
        prepare_can3_tx(&mut tx, 8);
        prepare_can3_rx(&mut rx, 8, 2);
        Self::bring_up(instance)
    }

    /// Creates `CAN3` from erased pads and explicit mux settings.
    ///
    /// This is the most flexible constructor and is useful when pin selection is
    /// determined dynamically.
    pub fn from_erased_pads(
        instance: ral::can3::CAN3,
        mut tx: ErasedPad,
        mut rx: ErasedPad,
        tx_alt: u32,
        rx_alt: u32,
        rx_daisy: u32,
    ) -> Self {
        iomuxc::alternate(&mut tx, tx_alt);
        iomuxc::set_sion(&mut tx);
        iomuxc::alternate(&mut rx, rx_alt);
        iomuxc::set_sion(&mut rx);
        set_can_pad_config(&mut tx);
        set_can_pad_config(&mut rx);
        let iomuxc = unsafe { ral::iomuxc::IOMUXC::instance() };
        ral::write_reg!(ral::iomuxc, iomuxc, CANFD_IPP_IND_CANRX_SELECT_INPUT, DAISY: rx_daisy);
        Self::bring_up(instance)
    }

    /// Creates `CAN3` from the singleton RAL instance and its default pin pair.
    ///
    /// # Safety
    ///
    /// The caller must ensure the `CAN3` peripheral is not already in use and
    /// that taking the singleton register block is sound.
    pub unsafe fn new_default(tx: GPIO_EMC_36, rx: GPIO_EMC_37) -> Self {
        Self::from_default_pins(unsafe { ral::can3::CAN3::instance() }, tx, rx)
    }
}

fn set_can_pad_config<P: Iomuxc>(pin: &mut P) {
    unsafe { ptr::write_volatile(pin.pad(), CAN_PAD_CTL) }
}

fn prepare_can3_tx<P: Iomuxc>(pin: &mut P, alt: u32) {
    iomuxc::alternate(pin, alt);
    iomuxc::set_sion(pin);
    set_can_pad_config(pin);
}

fn prepare_can3_rx<P: Iomuxc>(pin: &mut P, alt: u32, daisy: u32) {
    iomuxc::alternate(pin, alt);
    iomuxc::set_sion(pin);
    set_can_pad_config(pin);
    let iomuxc = unsafe { ral::iomuxc::IOMUXC::instance() };
    ral::write_reg!(ral::iomuxc, iomuxc, CANFD_IPP_IND_CANRX_SELECT_INPUT, DAISY: daisy);
}
