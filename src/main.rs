#![no_std]
#![no_main]

mod filters;
mod transmitter;
mod utils;

use imxrt_log as logging;
use teensy4_bsp as bsp;
use teensy4_panic as _;

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
    let mut poller = logging::log::usbd(usb, logging::Interrupts::Disabled).unwrap();

    let mut receiver: board::Lpuart5 = board::lpuart(lpuart5, pins.p35, pins.p34, 100_000);
    receiver.disable(|uart| {
        uart.set_parity(Parity::EVEN);
        uart.set_inversion(Direction::Rx, true);
        uart.set_inversion(Direction::Tx, true);
    });

    for _ in 0..(STARTUP_GRACE_MS / USB_POLL_INTERVAL_MS) {
        poller.poll();
        delay.block_ms(USB_POLL_INTERVAL_MS);
    }

    log::info!("firmware-rs booted on Teensy 4.1");
    log::info!("ET16S receiver on LPUART5: p34=RX p35=TX, 100000 baud, 8E1, inverted");

    let mut parser = SbusParser::new();

    loop {
        poller.poll();

        match receiver.try_read() {
            Ok(Some(byte)) => {
                if let Some(frame) = parser.push(byte) {
                    led.toggle();
                    log::info!(
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
                log::warn!("uart receive error: {:?}", err);
                delay.block_ms(1);
            }
        }
    }
}
