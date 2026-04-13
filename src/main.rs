#![no_std]
#![no_main]

mod transmitter;

use core::fmt::Write;
use teensy4_bsp as bsp;
use teensy4_panic as _;
use teensy4_usb::{UsbTeensy4, usb_writeln};

use bsp::{
    board,
    hal::{
        lpuart::{Direction, Parity},
        timer::Blocking,
    },
};
use transmitter::{SbusParser, scale_axis};

const USB_POLL_INTERVAL_MS: u32 = 10;
const STARTUP_GRACE_MS: u32 = 2_000;

#[bsp::rt::entry]
fn main() -> ! {
    let board::Resources {
        pit,
        usb,
        pins,
        mut gpio2,
        lpuart5,
        ..
    } = board::t41(board::instances());

    let led = board::led(&mut gpio2, pins.p13);
    let mut delay = Blocking::<_, { board::PERCLK_FREQUENCY }>::from_pit(pit.0);
    let mut usb_serial = UsbTeensy4::new_serial(usb);

    let mut receiver: board::Lpuart5 = board::lpuart(lpuart5, pins.p35, pins.p34, 100_000);
    receiver.disable(|uart| {
        uart.set_parity(Parity::EVEN);
        uart.set_inversion(Direction::Rx, true);
        uart.set_inversion(Direction::Tx, true);
    });

    // Wait for USB
    for _ in 0..(STARTUP_GRACE_MS / USB_POLL_INTERVAL_MS) {
        usb_serial.poll();
        delay.block_ms(USB_POLL_INTERVAL_MS);
    }

    usb_writeln!(usb_serial, "firmware-rs booted on Teensy 4.1");
    usb_writeln!(
        usb_serial,
        "ET16S receiver on LPUART5: p34=RX p35=TX, 100000 baud, 8E1, inverted"
    );

    let mut parser = SbusParser::new();
    let mut host_buf = [0u8; 64];

    loop {
        usb_serial.poll();

        if let Ok(read) = usb_serial.try_read(&mut host_buf) {
            if read > 0 {
                let _ = usb_serial.write(&host_buf[..read]);
            }
        }

        match receiver.try_read() {
            Ok(Some(byte)) => {
                if let Some(frame) = parser.push(byte) {
                    led.toggle();
                    usb_writeln!(
                        usb_serial,
                        "et16s raw ch=[{}, {}, {}, {}] scaled=[{:.2}, {:.2}, {:.2}, {:.2}] flags=0x{:02X} disconnected={}",
                        frame.channels[0],
                        frame.channels[1],
                        frame.channels[2],
                        frame.channels[3],
                        scale_axis(frame.channels[0]),
                        scale_axis(frame.channels[1]),
                        scale_axis(frame.channels[2]),
                        scale_axis(frame.channels[3]),
                        frame.flags,
                        frame.disconnected(),
                    );
                }
            }
            Ok(None) => {
                delay.block_ms(1);
            }
            Err(err) => {
                receiver.clear_status(bsp::hal::lpuart::Status::W1C);
                usb_writeln!(usb_serial, "uart receive error: {:?}", err);
                delay.block_ms(1);
            }
        }
    }
}
