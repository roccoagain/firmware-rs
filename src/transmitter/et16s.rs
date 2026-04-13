const DISCONNECT_FLAG_MASK: u8 = 0b0000_1100;

#[derive(Clone, Copy, Debug)]
pub struct Et16sFrame {
    pub channels: [u16; 16],
    pub flags: u8,
}

impl Et16sFrame {
    pub fn disconnected(&self) -> bool {
        self.flags & DISCONNECT_FLAG_MASK != 0
    }
}

pub struct SbusParser {
    buffer: [u8; 25],
    index: usize,
}

impl SbusParser {
    pub const fn new() -> Self {
        Self {
            buffer: [0; 25],
            index: 0,
        }
    }

    pub fn push(&mut self, byte: u8) -> Option<Et16sFrame> {
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

pub fn scale_axis(raw: u16) -> f32 {
    const MIN_IN: f32 = 353.0;
    const MAX_IN: f32 = 1695.0;
    const MIN_OUT: f32 = -1.0;
    const MAX_OUT: f32 = 1.0;

    let value = raw as f32;
    let scaled = MIN_OUT + (value - MIN_IN) * (MAX_OUT - MIN_OUT) / (MAX_IN - MIN_IN);
    scaled.clamp(MIN_OUT, MAX_OUT)
}
