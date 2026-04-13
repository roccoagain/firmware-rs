pub(crate) const CAN_PAD_CTL: u32 = 0x10B0;

pub(crate) const MCR_MDIS: u32 = 1 << 31;
pub(crate) const MCR_FRZ: u32 = 1 << 30;
pub(crate) const MCR_FEN: u32 = 1 << 29;
pub(crate) const MCR_HALT: u32 = 1 << 28;
pub(crate) const MCR_NOTRDY: u32 = 1 << 27;
pub(crate) const MCR_WAKMSK: u32 = 1 << 26;
pub(crate) const MCR_SOFTRST: u32 = 1 << 25;
pub(crate) const MCR_FRZACK: u32 = 1 << 24;
pub(crate) const MCR_SLFWAK: u32 = 1 << 22;
pub(crate) const MCR_WRNEN: u32 = 1 << 21;
pub(crate) const MCR_LPMACK: u32 = 1 << 20;
pub(crate) const MCR_WAKSRC: u32 = 1 << 19;
pub(crate) const MCR_SRXDIS: u32 = 1 << 17;
pub(crate) const MCR_IRMQ: u32 = 1 << 16;
pub(crate) const MCR_DMA: u32 = 1 << 15;
pub(crate) const MCR_LPRIOEN: u32 = 1 << 13;
pub(crate) const MCR_AEN: u32 = 1 << 12;

pub(crate) const CTRL1_LOM: u32 = 1 << 3;
pub(crate) const CTRL1_ERRMSK: u32 = 1 << 14;

pub(crate) const CTRL2_EACEN: u32 = 1 << 16;
pub(crate) const CTRL2_RRS: u32 = 1 << 17;
pub(crate) const CTRL2_MRP: u32 = 1 << 18;
pub(crate) const CTRL2_RFFN_SHIFT: u32 = 24;

pub(crate) const MB_CS_CODE_SHIFT: u32 = 24;
pub(crate) const MB_CS_CODE_MASK: u32 = 0x0F << MB_CS_CODE_SHIFT;
pub(crate) const MB_CS_SRR: u32 = 1 << 22;
pub(crate) const MB_CS_IDE: u32 = 1 << 21;
pub(crate) const MB_CS_RTR: u32 = 1 << 20;
pub(crate) const MB_CS_DLC_SHIFT: u32 = 16;
pub(crate) const MB_CS_TIMESTAMP_MASK: u32 = 0xFFFF;

pub(crate) const MB_CODE_RX_INACTIVE: u32 = 0x0;
pub(crate) const MB_CODE_RX_EMPTY: u32 = 0x4;
pub(crate) const MB_CODE_RX_FULL: u32 = 0x2;
pub(crate) const MB_CODE_RX_OVERRUN: u32 = 0x6;
pub(crate) const MB_CODE_TX_INACTIVE: u32 = 0x8;
pub(crate) const MB_CODE_TX_ABORT: u32 = 0x9;
pub(crate) const MB_CODE_TX_ONCE: u32 = 0xC;

pub(crate) const MB_ID_EXT_MASK: u32 = 0x1FFF_FFFF;
pub(crate) const MB_ID_STD_SHIFT: u32 = 18;

pub(crate) const fn mailbox_offset(index: u8) -> usize {
    0x80 + index as usize * 0x10
}

pub(crate) const fn mb_code(code: u32) -> u32 {
    code << MB_CS_CODE_SHIFT
}

pub(crate) const fn mb_code_value(cs: u32) -> u32 {
    (cs & MB_CS_CODE_MASK) >> MB_CS_CODE_SHIFT
}

pub(crate) const fn mb_id_std(id: u32) -> u32 {
    (id & 0x7FF) << MB_ID_STD_SHIFT
}

pub(crate) const fn mb_id_ext(id: u32) -> u32 {
    id & MB_ID_EXT_MASK
}

pub(crate) const fn rx_empty_cs(extended: bool) -> u32 {
    mb_code(MB_CODE_RX_EMPTY) | if extended { MB_CS_IDE | MB_CS_SRR } else { 0 }
}

pub(crate) const fn mb_is_tx(cs: u32) -> bool {
    mb_code_value(cs) >> 3 != 0
}

pub(crate) fn pack_word(bytes: &[u8]) -> u32 {
    (u32::from(bytes[0]) << 24)
        | (u32::from(bytes[1]) << 16)
        | (u32::from(bytes[2]) << 8)
        | u32::from(bytes[3])
}

pub(crate) fn unpack_word(word: u32, out: &mut [u8]) {
    out[0] = (word >> 24) as u8;
    out[1] = (word >> 16) as u8;
    out[2] = (word >> 8) as u8;
    out[3] = word as u8;
}
