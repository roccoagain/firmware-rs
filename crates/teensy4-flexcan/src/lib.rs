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
//! Typical usage looks like this:
//!
//! ```ignore
//! use teensy4_flexcan::{Can1, CanMessage, Clock, IdType, Mailbox, MailboxMode};
//! use teensy4_bsp as bsp;
//!
//! let instances = bsp::board::instances();
//! let mut pins = bsp::pins::t41::Pins::new(instances.iomuxc);
//!
//! let mut can = Can1::from_instance(instances.CAN1, pins.p22, pins.p23);
//! can.set_clock(Clock::Mhz24);
//! can.set_baud_rate(1_000_000, false)?;
//!
//! can.set_mailbox(Mailbox::new(0).unwrap(), MailboxMode::Tx, IdType::Standard)?;
//! can.set_mailbox(Mailbox::new(1).unwrap(), MailboxMode::Rx, IdType::Standard)?;
//!
//! let message = CanMessage {
//!     id: 0x123,
//!     len: 2,
//!     buf: [0xAB, 0xCD, 0, 0, 0, 0, 0, 0],
//!     ..CanMessage::default()
//! };
//!
//! can.write_mailbox(Mailbox::new(0).unwrap(), &message)?;
//!
//! if let Some(received) = can.read() {
//!     let _id = received.id;
//!     let _payload = &received.buf[..received.len as usize];
//! }
//! # Ok::<(), teensy4_flexcan::Error>(())
//! ```
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
