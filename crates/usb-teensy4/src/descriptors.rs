use usb_device::device::UsbVidPid;

pub const VID_PID: UsbVidPid = UsbVidPid(0x16C0, 0x0483);
pub const MANUFACTURER: &str = "Teensyduino";
pub const PRODUCT: &str = "USB Serial";
pub const DEVICE_RELEASE_BCD: u16 = 0x0280;
pub const EP0_MAX_PACKET_SIZE: u8 = 64;
