#![no_std]
#![no_main]

mod filters;
mod utils;

use cortex_m::peripheral::SCB;
use imxrt_log as logging;
use teensy4_bsp as bsp;
use teensy4_flexcan::{Can3, CanMessage, IdType, Mailbox, MailboxMode};
use teensy4_panic as _;

use bsp::{board, hal::timer::Blocking};

const USB_POLL_INTERVAL_MS: u32 = 10;
const STARTUP_GRACE_MS: u32 = 2_000;
const TX_INTERVAL_MS: u32 = 500;
const RESET_AFTER_LOOPS: u32 = 20;

#[bsp::rt::entry]
fn main() -> ! {
    let board::Resources {
        pit,
        usb,
        pins: board_pins,
        mut gpio2,
        ..
    } = board::t41(board::instances());

    let led_pin = board_pins.p13;
    let can3_rx = board_pins.p30;
    let can3_tx = board_pins.p31;

    let led = board::led(&mut gpio2, led_pin);
    let mut delay = Blocking::<_, { board::PERCLK_FREQUENCY }>::from_pit(pit.0);
    let mut poller = logging::log::usbd(usb, logging::Interrupts::Disabled).unwrap();

    let mut can = unsafe { Can3::new_default(can3_tx, can3_rx) };
    can.set_baud_rate(500_000, false).unwrap();
    can.set_mailbox(Mailbox::new(8).unwrap(), MailboxMode::Tx, IdType::Standard)
        .unwrap();

    for _ in 0..(STARTUP_GRACE_MS / USB_POLL_INTERVAL_MS) {
        poller.poll();
        delay.block_ms(USB_POLL_INTERVAL_MS);
    }

    log::info!("firmware-rs booted on Teensy 4.1");
    log::info!("CAN3 example on pins p31=TX / p30=RX at 500 kbit/s");

    let mut counter = 0u8;
    let mut loops = 0u32;

    loop {
        counter = counter.wrapping_add(1);
        loops += 1;
        led.toggle();

        let mut frame = CanMessage {
            id: 0x123,
            len: 2,
            ..CanMessage::default()
        };
        frame.buf[0] = counter;
        frame.buf[1] = 0xA5;

        match can.write_mailbox(Mailbox::new(8).unwrap(), &frame) {
            Ok(()) => log::info!("sent CAN frame id=0x123 data=[0x{:02X}, 0xA5]", counter),
            Err(err) => log::warn!("CAN transmit failed: {:?}", err),
        }

        for _ in 0..(TX_INTERVAL_MS / USB_POLL_INTERVAL_MS) {
            poller.poll();
            delay.block_ms(USB_POLL_INTERVAL_MS);
        }

        if loops >= RESET_AFTER_LOOPS {
            log::info!("resetting after {} loops", loops);
            delay.block_ms(USB_POLL_INTERVAL_MS);
            SCB::sys_reset();
        }
    }
}
