#![no_std]
#![allow(static_mut_refs)]

mod descriptors;

use core::{
    arch::asm,
    fmt,
    mem::MaybeUninit,
    str,
    sync::atomic::{AtomicBool, AtomicU8, AtomicU32, Ordering},
};
use teensy4_bsp::hal::usbd::{BusAdapter, EndpointMemory, EndpointState};
use teensy4_bsp::ral;
use usb_device::{
    bus::UsbBusAllocator,
    device::{UsbDevice, UsbDeviceBuilder, UsbDeviceState},
    UsbError,
};
use usbd_serial::SerialPort;

type UsbClass = SerialPort<'static, BusAdapter, [u8; 512], [u8; 512]>;
type UsbDev = UsbDevice<'static, BusAdapter>;

static TAKEN: AtomicBool = AtomicBool::new(false);
static EP_MEMORY: EndpointMemory<2048> = EndpointMemory::new();
static EP_STATE: EndpointState = EndpointState::max_endpoints();
static mut BUS: MaybeUninit<UsbBusAllocator<BusAdapter>> = MaybeUninit::uninit();
static mut CLASS: MaybeUninit<UsbClass> = MaybeUninit::uninit();
static mut DEVICE: MaybeUninit<UsbDev> = MaybeUninit::uninit();
static CONFIGURED: AtomicBool = AtomicBool::new(false);
static LAST_LINE_CODING: AtomicU32 = AtomicU32::new(0);
static REBOOT_POLLS_REMAINING: AtomicU8 = AtomicU8::new(0);
static mut SERIAL_NUMBER_BYTES: [u8; 10] = [0; 10];
static SERIAL_NUMBER_LEN: AtomicU8 = AtomicU8::new(0);

const TEENSY_REBOOT_BAUD: u32 = 134;
const TEENSY_REBOOT_POLLS: u8 = 80;

#[macro_export]
macro_rules! usb_write {
    ($dst:expr, $($arg:tt)*) => {{
        let _ = core::write!($dst, $($arg)*);
    }};
}

#[macro_export]
macro_rules! usb_writeln {
    ($dst:expr) => {{
        let _ = core::writeln!($dst);
    }};
    ($dst:expr, $($arg:tt)*) => {{
        let _ = core::writeln!($dst, $($arg)*);
    }};
}

pub struct UsbTeensy4 {
    _private: (),
}

impl UsbTeensy4 {
    pub fn new_serial<P>(peripherals: P) -> Self
    where
        P: teensy4_bsp::hal::usbd::Peripherals,
    {
        if TAKEN.swap(true, Ordering::SeqCst) {
            panic!("UsbTeensy4 already initialized");
        }

        let bus_adapter = BusAdapter::new(peripherals, &EP_MEMORY, &EP_STATE);
        bus_adapter.set_interrupts(false);

        // Safety: TAKEN ensures there is only one live stack.
        let bus = unsafe {
            BUS.write(UsbBusAllocator::new(bus_adapter));
            BUS.assume_init_ref()
        };

        let class = SerialPort::new_with_store(bus, [0; 512], [0; 512]);
        let device = UsbDeviceBuilder::new(bus, descriptors::VID_PID)
            .composite_with_iads()
            .device_release(descriptors::DEVICE_RELEASE_BCD)
            .manufacturer(descriptors::MANUFACTURER)
            .product(descriptors::PRODUCT)
            .serial_number(serial_number())
            .max_packet_size_0(descriptors::EP0_MAX_PACKET_SIZE)
            .build();

        // Safety: TAKEN ensures there is only one live stack.
        unsafe {
            CLASS.write(class);
            DEVICE.write(device);
        }

        CONFIGURED.store(false, Ordering::SeqCst);
        LAST_LINE_CODING.store(0, Ordering::SeqCst);
        REBOOT_POLLS_REMAINING.store(0, Ordering::SeqCst);

        Self { _private: () }
    }

    pub fn poll(&mut self) {
        let (device, class) = unsafe { (DEVICE.assume_init_mut(), CLASS.assume_init_mut()) };

        device.poll(&mut [class]);

        let configured = device.state() == UsbDeviceState::Configured;
        let was_configured = CONFIGURED.swap(configured, Ordering::SeqCst);
        if configured && !was_configured {
            device.bus().configure();
        }

        if configured {
            let _ = class.flush();
        }

        // Teensy tools request a 134 baud CDC line coding change to ask the
        // running firmware to jump into HalfKay, which enables buttonless flashing.
        let line_coding = class.line_coding().data_rate();
        let last_line_coding = LAST_LINE_CODING.swap(line_coding, Ordering::SeqCst);
        if line_coding == TEENSY_REBOOT_BAUD && last_line_coding != TEENSY_REBOOT_BAUD {
            REBOOT_POLLS_REMAINING.store(TEENSY_REBOOT_POLLS, Ordering::SeqCst);
        }

        let reboot_polls_remaining = REBOOT_POLLS_REMAINING.load(Ordering::SeqCst);
        if reboot_polls_remaining > 0 {
            REBOOT_POLLS_REMAINING.store(reboot_polls_remaining - 1, Ordering::SeqCst);
            if reboot_polls_remaining == 1 {
                reboot_teensyduino();
            }
        }
    }

    pub fn is_configured(&self) -> bool {
        CONFIGURED.load(Ordering::SeqCst)
    }

    pub fn write(&mut self, bytes: &[u8]) -> Result<usize, UsbError> {
        self.poll();
        if !self.is_configured() {
            return Err(UsbError::WouldBlock);
        }

        unsafe { CLASS.assume_init_mut().write(bytes) }
    }

    pub fn try_read(&mut self, bytes: &mut [u8]) -> Result<usize, UsbError> {
        self.poll();
        if !self.is_configured() {
            return Err(UsbError::WouldBlock);
        }

        unsafe { CLASS.assume_init_mut().read(bytes) }
    }
}

fn reboot_teensyduino() -> ! {
    unsafe {
        let ocotp = ral::ocotp::OCOTP::instance();
        if ral::read_reg!(ral::ocotp, ocotp, CFG5) & 0x02 == 0 {
            asm!("bkpt #251", options(noreturn));
        }

        let usb1 = ral::usb::USB1::instance();
        let iomuxc_gpr = ral::iomuxc_gpr::IOMUXC_GPR::instance();

        asm!("cpsid i");
        ral::write_reg!(ral::usb, usb1, USBCMD, 0);
        ral::write_reg!(ral::iomuxc_gpr, iomuxc_gpr, GPR16, 0x0020_0003);

        asm!("mov sp, {stack}", stack = in(reg) 0x2020_1000u32);
        asm!("dsb sy", options(nostack, preserves_flags));

        let mailbox = 0x2020_8000 as *mut u32;
        mailbox.write_volatile(0xEB12_0000);

        let rom_table = (0x0020_001C as *const u32).read_volatile();
        let boot_entry = (rom_table as *const u32).add(2).read_volatile();
        let boot: extern "C" fn(*mut u32) -> ! = core::mem::transmute(boot_entry as usize);
        boot(mailbox)
    }
}

fn serial_number() -> &'static str {
    let mut num = unsafe {
        let ocotp = ral::ocotp::OCOTP::instance();
        ral::read_reg!(ral::ocotp, ocotp, MAC0) & 0x00ff_ffff
    };

    // Match the PJRC core's workaround for macOS CDC-ACM serial handling.
    if num < 10_000_000 {
        num *= 10;
    }

    let mut digits = [0u8; 10];
    let mut len = 0usize;

    loop {
        digits[len] = b'0' + (num % 10) as u8;
        len += 1;
        num /= 10;
        if num == 0 {
            break;
        }
    }

    unsafe {
        for i in 0..len {
            SERIAL_NUMBER_BYTES[i] = digits[len - 1 - i];
        }
    }
    SERIAL_NUMBER_LEN.store(len as u8, Ordering::SeqCst);

    unsafe {
        str::from_utf8_unchecked(&SERIAL_NUMBER_BYTES[..SERIAL_NUMBER_LEN.load(Ordering::SeqCst) as usize])
    }
}

impl fmt::Write for UsbTeensy4 {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let mut bytes = s.as_bytes();
        while !bytes.is_empty() {
            match self.write(bytes) {
                Ok(written) => bytes = &bytes[written..],
                Err(UsbError::WouldBlock) => self.poll(),
                Err(_) => return Err(fmt::Error),
            }
        }
        Ok(())
    }
}
