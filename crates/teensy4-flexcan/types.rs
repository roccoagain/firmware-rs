pub(crate) const MAX_MAILBOXES: usize = 64;

/// Additional flags attached to a CAN frame.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MessageFlags {
    /// `true` for a 29-bit identifier, `false` for an 11-bit identifier.
    pub extended: bool,
    /// `true` when the frame is a remote transmission request.
    pub remote: bool,
    /// `true` when the controller reported an RX overrun for the mailbox.
    pub overrun: bool,
}

/// A single classic CAN frame and associated metadata.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CanMessage {
    /// CAN identifier.
    pub id: u32,
    /// Hardware timestamp captured by the controller.
    pub timestamp: u16,
    /// FIFO acceptance hit index.
    pub idhit: u8,
    /// Identifier / RTR / overrun flags.
    pub flags: MessageFlags,
    /// Number of valid data bytes in [`Self::buf`].
    pub len: u8,
    /// Payload bytes for the frame.
    pub buf: [u8; 8],
    /// Mailbox index that received the frame, or `-1` when unset.
    pub mb: i8,
    /// CAN bus number that received the frame.
    pub bus: u8,
    /// Sequence flag preserved from the original Teensy API surface.
    pub seq: bool,
}

impl Default for CanMessage {
    fn default() -> Self {
        Self {
            id: 0,
            timestamp: 0,
            idhit: 0,
            flags: MessageFlags {
                extended: false,
                remote: false,
                overrun: false,
            },
            len: 8,
            buf: [0; 8],
            mb: -1,
            bus: 0,
            seq: false,
        }
    }
}

/// Selects between the default and alternate pin routing for a peripheral.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PinSelection {
    /// Use the primary pin routing.
    Default,
    /// Use the alternate pin routing.
    Alternate,
}

/// Direction assigned to a mailbox.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MailboxMode {
    /// Transmit mailbox.
    Tx,
    /// Receive mailbox.
    Rx,
}

/// Identifier encoding assigned to a mailbox.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdType {
    /// 11-bit standard identifier.
    Standard,
    /// 29-bit extended identifier.
    Extended,
    /// Disable the mailbox.
    Inactive,
}

/// High-level mailbox filtering behavior.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FilterMode {
    /// Accept any identifier for the mailbox.
    AcceptAll,
    /// Reject every identifier for the mailbox.
    RejectAll,
}

/// FlexCAN functional clock selection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Clock {
    /// Disable the CAN clock.
    Off,
    /// 8 MHz CAN clock.
    Mhz8,
    /// 16 MHz CAN clock.
    Mhz16,
    /// 20 MHz CAN clock.
    Mhz20,
    /// 24 MHz CAN clock.
    Mhz24,
    /// 30 MHz CAN clock.
    Mhz30,
    /// 40 MHz CAN clock.
    Mhz40,
    /// 60 MHz CAN clock.
    Mhz60,
    /// 80 MHz CAN clock.
    Mhz80,
}

impl Clock {
    /// Returns the selected clock frequency in hertz.
    pub const fn hz(self) -> u32 {
        match self {
            Self::Off => 0,
            Self::Mhz8 => 8_000_000,
            Self::Mhz16 => 16_000_000,
            Self::Mhz20 => 20_000_000,
            Self::Mhz24 => 24_000_000,
            Self::Mhz30 => 30_000_000,
            Self::Mhz40 => 40_000_000,
            Self::Mhz60 => 60_000_000,
            Self::Mhz80 => 80_000_000,
        }
    }

    pub(crate) const fn cscmr2(self) -> (u32, u32) {
        match self {
            Self::Off => (3, 0),
            Self::Mhz8 => (2, 9),
            Self::Mhz16 => (2, 4),
            Self::Mhz20 => (2, 3),
            Self::Mhz24 => (1, 0),
            Self::Mhz30 => (0, 1),
            Self::Mhz40 => (2, 1),
            Self::Mhz60 => (0, 0),
            Self::Mhz80 => (2, 0),
        }
    }
}

/// Mailbox identifier.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Mailbox(pub u8);

impl Mailbox {
    /// Pseudo-mailbox constant used by FIFO-oriented code paths.
    pub const FIFO: Self = Self(99);

    /// Creates a mailbox handle for a valid mailbox index.
    pub const fn new(index: u8) -> Option<Self> {
        if index < MAX_MAILBOXES as u8 {
            Some(Self(index))
        } else {
            None
        }
    }

    /// Returns the raw mailbox index.
    pub const fn index(self) -> u8 {
        self.0
    }
}

/// Current controller state for a mailbox.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MailboxState {
    /// Receive mailbox is inactive.
    RxInactive,
    /// Receive mailbox is armed and empty.
    RxEmpty {
        /// `true` when the mailbox expects 29-bit identifiers.
        extended: bool,
    },
    /// Receive mailbox holds a frame.
    RxFull {
        /// `true` when the mailbox is configured for 29-bit identifiers.
        extended: bool,
    },
    /// Receive mailbox holds a frame and reported an overrun.
    RxOverrun {
        /// `true` when the mailbox is configured for 29-bit identifiers.
        extended: bool,
    },
    /// Transmit mailbox is idle.
    TxInactive,
    /// Transmit mailbox has been aborted.
    TxAbort,
    /// Transmit mailbox is actively sending.
    TxBusy {
        /// `true` when the queued frame uses a 29-bit identifier.
        extended: bool,
    },
    /// Raw mailbox code not modeled by this crate.
    Unknown(u32),
}

/// Errors returned by FlexCAN operations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    /// The requested baud rate cannot be represented with the current clock.
    InvalidBaudRate,
    /// The mailbox index or mailbox role is invalid for the operation.
    InvalidMailbox,
    /// The message length exceeded classic CAN's 8-byte payload limit.
    InvalidMessageLength,
    /// The peripheral state does not allow the requested operation.
    InvalidConfiguration,
    /// A non-blocking transmit operation found no free TX mailbox.
    WouldBlock,
}
