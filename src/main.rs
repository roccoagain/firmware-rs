#![no_std]
#![no_main]

mod filters;
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

const USB_POLL_INTERVAL_MS: u32 = 10;
const STARTUP_GRACE_MS: u32 = 2_000;
const DISCONNECT_FLAG_MASK: u8 = 0b0000_1100;

#[derive(Clone, Copy, Debug)]
struct Et16sFrame {
    channels: [u16; 16],
    flags: u8,
}

impl Et16sFrame {
    fn disconnected(&self) -> bool {
        self.flags & DISCONNECT_FLAG_MASK != 0
    }
}

struct SbusParser {
    buffer: [u8; 25],
    index: usize,
}

impl SbusParser {
    const fn new() -> Self {
        Self {
            buffer: [0; 25],
            index: 0,
        }
    }

    fn push(&mut self, byte: u8) -> Option<Et16sFrame> {
        if self.index == 0 {
            if byte != 0x0F {
                return None;
            }
            self.buffer[0] = byte;
            self.index = 1;
            return None;
        }

        self.buffer[self.index] = byte;
        self.index += 1;

        if self.index < self.buffer.len() {
            return None;
        }

        self.index = 0;

        // Legacy firmware aligns packets by looking for a 0x00 byte before the next 0x0F.
        // SBUS packets also conventionally end with 0x00.
        if self.buffer[24] != 0x00 {
            return None;
        }

        Some(parse_sbus_packet(&self.buffer))
    }
}

fn parse_sbus_packet(packet: &[u8; 25]) -> Et16sFrame {
    let mut channels = [0u16; 16];

    channels[0] = ((packet[1] as u16) | ((packet[2] as u16) << 8)) & 0x07FF;
    channels[1] = (((packet[2] as u16) >> 3) | ((packet[3] as u16) << 5)) & 0x07FF;
    channels[2] =
        (((packet[3] as u16) >> 6) | ((packet[4] as u16) << 2) | ((packet[5] as u16) << 10))
            & 0x07FF;
    channels[3] = (((packet[5] as u16) >> 1) | ((packet[6] as u16) << 7)) & 0x07FF;
    channels[4] = (((packet[6] as u16) >> 4) | ((packet[7] as u16) << 4)) & 0x07FF;
    channels[5] =
        (((packet[7] as u16) >> 7) | ((packet[8] as u16) << 1) | ((packet[9] as u16) << 9))
            & 0x07FF;
    channels[6] = (((packet[9] as u16) >> 2) | ((packet[10] as u16) << 6)) & 0x07FF;
    channels[7] = (((packet[10] as u16) >> 5) | ((packet[11] as u16) << 3)) & 0x07FF;
    channels[8] = ((packet[12] as u16) | ((packet[13] as u16) << 8)) & 0x07FF;
    channels[9] = (((packet[13] as u16) >> 3) | ((packet[14] as u16) << 5)) & 0x07FF;
    channels[10] =
        (((packet[14] as u16) >> 6) | ((packet[15] as u16) << 2) | ((packet[16] as u16) << 10))
            & 0x07FF;
    channels[11] = (((packet[16] as u16) >> 1) | ((packet[17] as u16) << 7)) & 0x07FF;
    channels[12] = (((packet[17] as u16) >> 4) | ((packet[18] as u16) << 4)) & 0x07FF;
    channels[13] =
        (((packet[18] as u16) >> 7) | ((packet[19] as u16) << 1) | ((packet[20] as u16) << 9))
            & 0x07FF;
    channels[14] = (((packet[20] as u16) >> 2) | ((packet[21] as u16) << 6)) & 0x07FF;
    channels[15] = (((packet[21] as u16) >> 5) | ((packet[22] as u16) << 3)) & 0x07FF;

    Et16sFrame {
        channels,
        flags: packet[23],
    }
}

fn scale_axis(raw: u16) -> f32 {
    const MIN_IN: f32 = 353.0;
    const MAX_IN: f32 = 1695.0;
    const MIN_OUT: f32 = -1.0;
    const MAX_OUT: f32 = 1.0;

    let value = raw as f32;
    let scaled = MIN_OUT + (value - MIN_IN) * (MAX_OUT - MIN_OUT) / (MAX_IN - MIN_IN);
    scaled.clamp(MIN_OUT, MAX_OUT)
}

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
