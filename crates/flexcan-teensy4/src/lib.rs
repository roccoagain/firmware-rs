//! FlexCAN driver for Teensy 4.x / i.MX RT1062.
//!
//! This crate provides a small, register-level wrapper around the three FlexCAN
//! peripherals available on the Teensy 4 family. It handles peripheral bring-up,
//! pad configuration, mailbox setup, transmit / receive operations, and basic
//! filtering.
//!
//! The API is centered around [`FlexCan`] together with the concrete aliases
//! [`Can1`], [`Can2`], and [`Can3`].
//!
//! Typical usage:
//! - construct a controller from an owned RAL instance and compatible pins,
//! - choose the CAN clock with [`FlexCan::set_clock`],
//! - program the baud rate with [`FlexCan::set_baud_rate`],
//! - configure mailboxes, then
//! - transmit with [`FlexCan::write`] or receive with [`FlexCan::read`].
//!
//! This crate is `#![no_std]` and is intended for bare-metal Teensy 4 targets.
#![no_std]
#![warn(missing_docs)]

#[cfg(test)]
extern crate std;

mod controller;
mod instance;
mod pins;
mod registers;
#[cfg(test)]
mod tests;
mod types;

pub use controller::{Can1, Can2, Can3, FlexCan};
pub use instance::Instance;
pub use types::{
    CanMessage, Clock, Error, FilterMode, IdType, Mailbox, MailboxMode, MailboxState, MessageFlags,
    PinSelection,
};
