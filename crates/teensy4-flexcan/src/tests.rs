use crate::registers::{
    MB_CODE_RX_EMPTY, MB_CODE_TX_INACTIVE, MB_CS_IDE, MB_CS_SRR, mb_code, mb_code_value,
    mb_id_ext, mb_id_std, mb_is_tx, pack_word, rx_empty_cs, unpack_word,
};
use crate::{CanMessage, Clock, Mailbox};

#[test]
fn can_message_default_matches_expected_defaults() {
    let message = CanMessage::default();

    assert_eq!(message.id, 0);
    assert_eq!(message.timestamp, 0);
    assert_eq!(message.idhit, 0);
    assert!(!message.flags.extended);
    assert!(!message.flags.remote);
    assert!(!message.flags.overrun);
    assert_eq!(message.len, 8);
    assert_eq!(message.buf, [0; 8]);
    assert_eq!(message.mb, -1);
    assert_eq!(message.bus, 0);
    assert!(!message.seq);
}

#[test]
fn clock_frequencies_match_documented_values() {
    assert_eq!(Clock::Off.hz(), 0);
    assert_eq!(Clock::Mhz8.hz(), 8_000_000);
    assert_eq!(Clock::Mhz16.hz(), 16_000_000);
    assert_eq!(Clock::Mhz20.hz(), 20_000_000);
    assert_eq!(Clock::Mhz24.hz(), 24_000_000);
    assert_eq!(Clock::Mhz30.hz(), 30_000_000);
    assert_eq!(Clock::Mhz40.hz(), 40_000_000);
    assert_eq!(Clock::Mhz60.hz(), 60_000_000);
    assert_eq!(Clock::Mhz80.hz(), 80_000_000);
}

#[test]
fn clock_cscmr2_encoding_matches_expected_register_values() {
    assert_eq!(Clock::Off.cscmr2(), (3, 0));
    assert_eq!(Clock::Mhz24.cscmr2(), (1, 0));
    assert_eq!(Clock::Mhz60.cscmr2(), (0, 0));
    assert_eq!(Clock::Mhz80.cscmr2(), (2, 0));
}

#[test]
fn mailbox_new_accepts_hardware_range_only() {
    assert_eq!(Mailbox::new(0), Some(Mailbox(0)));
    assert_eq!(Mailbox::new(63), Some(Mailbox(63)));
    assert_eq!(Mailbox::new(64), None);
    assert_eq!(Mailbox::FIFO.index(), 99);
}

#[test]
fn mailbox_code_helpers_round_trip_and_classify_tx() {
    let rx_empty = mb_code(MB_CODE_RX_EMPTY);
    let tx_inactive = mb_code(MB_CODE_TX_INACTIVE);

    assert_eq!(mb_code_value(rx_empty), MB_CODE_RX_EMPTY);
    assert_eq!(mb_code_value(tx_inactive), MB_CODE_TX_INACTIVE);
    assert!(!mb_is_tx(rx_empty));
    assert!(mb_is_tx(tx_inactive));
}

#[test]
fn rx_empty_cs_sets_extended_bits_only_for_extended_mailboxes() {
    assert_eq!(rx_empty_cs(false), mb_code(MB_CODE_RX_EMPTY));
    assert_eq!(
        rx_empty_cs(true),
        mb_code(MB_CODE_RX_EMPTY) | MB_CS_IDE | MB_CS_SRR
    );
}

#[test]
fn id_helpers_mask_and_shift_as_expected() {
    assert_eq!(mb_id_std(0x7FF), 0x7FF << 18);
    assert_eq!(mb_id_std(0xFFFF_FFFF), 0x7FF << 18);
    assert_eq!(mb_id_ext(0x1FFF_FFFF), 0x1FFF_FFFF);
    assert_eq!(mb_id_ext(0xFFFF_FFFF), 0x1FFF_FFFF);
}

#[test]
fn pack_and_unpack_word_preserve_byte_order() {
    let bytes = [0x12, 0x34, 0x56, 0x78];
    let word = pack_word(&bytes);
    let mut out = [0u8; 4];

    assert_eq!(word, 0x1234_5678);

    unpack_word(word, &mut out);
    assert_eq!(out, bytes);
}
