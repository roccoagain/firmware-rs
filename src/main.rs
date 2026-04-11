#![no_std]
#![no_main]

#[allow(dead_code)]
mod reset;

use imxrt_log as logging;
use teensy4_bsp as bsp;
use teensy4_panic as _;

use bsp::{board, hal::timer::Blocking};

const USB_POLL_INTERVAL_MS: u32 = 10;
const STARTUP_GRACE_MS: u32 = 2_000;
const BLINK_INTERVAL_MS: u32 = 500;
const RESET_LOG_DRAIN_MS: u32 = 250;
const RESET_AFTER_BLINKS: u32 = 20;

#[bsp::rt::entry]
fn main() -> ! {
    let board::Resources {
        pit,
        usb,
        pins,
        mut gpio2,
        ..
    } = board::t41(board::instances());

    let led = board::led(&mut gpio2, pins.p13);
    let mut delay = Blocking::<_, { board::PERCLK_FREQUENCY }>::from_pit(pit.0);
    let mut poller = logging::log::usbd(usb, logging::Interrupts::Disabled).unwrap();
    let mut blink_count = 0u32;

    for _ in 0..(STARTUP_GRACE_MS / USB_POLL_INTERVAL_MS) {
        poller.poll();
        delay.block_ms(USB_POLL_INTERVAL_MS);
    }

    log::info!("firmware-rs booted on Teensy 4.1");
    log::info!("starting blink loop after {} ms USB grace period", STARTUP_GRACE_MS);

    loop {
        blink_count = blink_count.wrapping_add(1);
        led.toggle();
        log::info!("blink {}", blink_count);

        for _ in 0..(BLINK_INTERVAL_MS / USB_POLL_INTERVAL_MS) {
            poller.poll();
            delay.block_ms(USB_POLL_INTERVAL_MS);
        }

        if blink_count >= RESET_AFTER_BLINKS {
            log::warn!("triggering software reset after {} blinks", blink_count);

            for _ in 0..(RESET_LOG_DRAIN_MS / USB_POLL_INTERVAL_MS) {
                poller.poll();
                delay.block_ms(USB_POLL_INTERVAL_MS);
            }

            reset::reset_teensy();
        }
    }
}
