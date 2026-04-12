use core::ptr;

use imxrt_ral as ral;

use crate::{
    instance::Instance,
    registers::*,
    types::{
        CanMessage, Clock, Error, FilterMode, IdType, Mailbox, MailboxMode, MailboxState,
        MessageFlags,
    },
};

/// A FlexCAN controller bound to a specific hardware instance.
pub struct FlexCan<I> {
    _instance: I,
    clock: Clock,
}

/// FlexCAN controller for `CAN1`.
pub type Can1 = FlexCan<ral::can::CAN1>;
/// FlexCAN controller for `CAN2`.
pub type Can2 = FlexCan<ral::can::CAN2>;
/// FlexCAN controller for `CAN3`.
pub type Can3 = FlexCan<ral::can3::CAN3>;

impl<I> FlexCan<I>
where
    I: Instance,
{
    pub(crate) fn bring_up(instance: I) -> Self {
        I::enable_clock();

        let mut can = Self {
            _instance: instance,
            clock: Clock::Mhz24,
        };
        can.begin();
        can
    }

    /// Resets and initializes the peripheral into a known default state.
    pub fn begin(&mut self) {
        self.set_clock(Clock::Mhz24);
        self.modify_reg(0x00, |mcr| mcr & !MCR_MDIS);
        self.enter_freeze_mode();
        self.modify_reg(0x04, |ctrl1| ctrl1 | CTRL1_LOM);
        self.modify_reg(0x00, |mcr| mcr | MCR_FRZ);
        while self.reg(0x00) & MCR_LPMACK != 0 {}
        self.soft_reset();
        while self.reg(0x00) & MCR_FRZACK == 0 {}

        self.modify_reg(0x00, |mcr| {
            let mut mcr = mcr;
            mcr |= MCR_SRXDIS | MCR_IRMQ | MCR_AEN | MCR_LPRIOEN | MCR_SLFWAK | MCR_WAKSRC;
            mcr |= MCR_WRNEN | MCR_WAKMSK;
            mcr &= !(MCR_DMA | (1 << 11));
            mcr
        });
        self.modify_reg(0x34, |ctrl2| ctrl2 | CTRL2_RRS | CTRL2_EACEN | CTRL2_MRP);

        self.disable_fifo();
        self.exit_freeze_mode();
    }

    /// Selects the FlexCAN functional clock.
    ///
    /// Call this before [`Self::set_baud_rate`] when you need a non-default
    /// clock source.
    pub fn set_clock(&mut self, clock: Clock) {
        self.clock = clock;
        let ccm = unsafe { ral::ccm::CCM::instance() };
        let (sel, podf) = clock.cscmr2();
        ral::modify_reg!(ral::ccm, ccm, CSCMR2, CAN_CLK_SEL: sel, CAN_CLK_PODF: podf);
    }

    /// Returns the currently configured FlexCAN clock selection.
    pub fn clock(&self) -> Clock {
        self.clock
    }

    /// Programs the nominal CAN bitrate.
    ///
    /// The controller temporarily enters freeze mode while timings are updated.
    /// `listen_only` leaves the peripheral in bus-monitoring mode.
    pub fn set_baud_rate(&mut self, baud: u32, listen_only: bool) -> Result<(), Error> {
        if baud == 0 || self.clock.hz() == 0 {
            return Err(Error::InvalidBaudRate);
        }

        let clock_hz = self.clock.hz();
        let mut divisor = 0u32;
        let mut best_divisor = 0u32;
        let mut result = clock_hz / baud / (divisor + 1);
        let mut best_error = baud.abs_diff(clock_hz / (result.max(1) * (divisor + 1)));

        let leave_freeze = self.reg(0x00) & MCR_FRZACK == 0;
        self.enter_freeze_mode();

        while result > 5 {
            divisor = divisor.saturating_add(1);
            result = clock_hz / baud / (divisor + 1);
            if (5..=25).contains(&result) {
                let error = baud.abs_diff(clock_hz / (result * (divisor + 1)));
                if error < best_error || (error == best_error && (12..=18).contains(&result)) {
                    best_error = error;
                    best_divisor = divisor;
                }
            }
        }

        divisor = best_divisor;
        result = clock_hz / baud / (divisor + 1);
        if !(5..=25).contains(&result) || best_error > 300 {
            if leave_freeze {
                self.exit_freeze_mode();
            }
            return Err(Error::InvalidBaudRate);
        }

        let timings = [
            (0, 0, 1),
            (1, 0, 1),
            (1, 1, 1),
            (2, 1, 1),
            (2, 2, 1),
            (2, 3, 1),
            (2, 3, 2),
            (2, 4, 2),
            (2, 5, 2),
            (2, 5, 3),
            (2, 6, 3),
            (2, 7, 3),
            (2, 7, 4),
            (3, 7, 4),
            (3, 7, 5),
            (4, 7, 5),
            (4, 7, 6),
            (5, 7, 6),
            (6, 7, 6),
            (6, 7, 7),
            (7, 7, 7),
        ];
        let (propseg, pseg1, pseg2) = timings[(result - 5) as usize];
        let ctrl1 =
            propseg | (1 << 22) | (pseg1 << 19) | (pseg2 << 16) | CTRL1_ERRMSK | (divisor << 24);
        self.write_reg(0x04, ctrl1);
        if listen_only {
            self.modify_reg(0x04, |v| v | CTRL1_LOM);
        } else {
            self.modify_reg(0x04, |v| v & !CTRL1_LOM);
        }

        if leave_freeze {
            self.exit_freeze_mode();
        }
        Ok(())
    }

    /// Enables the hardware receive FIFO.
    ///
    /// This reinitializes mailbox state and reserves the low-numbered mailbox
    /// region for FIFO operation.
    pub fn enable_fifo(&mut self) {
        let leave_freeze = self.reg(0x00) & MCR_FRZACK == 0;
        self.enter_freeze_mode();

        self.modify_reg(0x00, |mcr| mcr & !MCR_FEN);
        self.write_imask(0);
        for index in 0..self.max_mailboxes() {
            self.clear_mailbox(index);
            self.write_rximr(index, 0);
        }
        self.write_reg(0x10, 0);
        self.write_reg(0x48, 0);
        let iflag = self.read_iflag();
        self.write_iflag(iflag);

        self.modify_reg(0x00, |mcr| mcr | MCR_FEN);
        for index in self.mailbox_offset()..self.max_mailboxes() {
            self.write_mailbox_cs(index, mb_code(MB_CODE_TX_INACTIVE));
            self.enable_mailbox_interrupt(Mailbox(index), true);
        }

        if leave_freeze {
            self.exit_freeze_mode();
        }
    }

    /// Disables the hardware receive FIFO and restores mailbox-based RX.
    pub fn disable_fifo(&mut self) {
        let leave_freeze = self.reg(0x00) & MCR_FRZACK == 0;
        self.enter_freeze_mode();

        self.modify_reg(0x00, |mcr| mcr & !MCR_FEN);
        self.write_imask(0);
        for index in 0..self.max_mailboxes() {
            self.clear_mailbox(index);
            if index < self.max_mailboxes() / 2 {
                let extended = index >= self.max_mailboxes() / 4;
                self.write_mailbox_cs(index, rx_empty_cs(extended));
                self.write_rximr(
                    index,
                    if self.reg(0x34) & CTRL2_EACEN != 0 {
                        1 << 30
                    } else {
                        0
                    },
                );
            } else {
                self.write_mailbox_cs(index, mb_code(MB_CODE_TX_INACTIVE));
                self.enable_mailbox_interrupt(Mailbox(index), true);
            }
        }
        let iflag = self.read_iflag();
        self.write_iflag(iflag);

        if leave_freeze {
            self.exit_freeze_mode();
        }
    }

    /// Enables or disables FIFO interrupts.
    ///
    /// This has no effect when FIFO mode is disabled.
    pub fn enable_fifo_interrupt(&mut self, enable: bool) {
        if self.reg(0x00) & MCR_FEN == 0 {
            return;
        }
        self.modify_reg(0x28, |imask1| {
            let mut imask1 = imask1 & !0xFF;
            if enable {
                imask1 |= 1 << 5;
            }
            imask1
        });
    }

    /// Enables or disables interrupts for every usable mailbox.
    pub fn enable_mailbox_interrupts(&mut self, enable: bool) {
        let leave_freeze = self.reg(0x00) & MCR_FRZACK == 0;
        self.enter_freeze_mode();
        for index in self.mailbox_offset()..self.max_mailboxes() {
            self.enable_mailbox_interrupt(Mailbox(index), enable);
        }
        if leave_freeze {
            self.exit_freeze_mode();
        }
    }

    /// Enables or disables interrupts for a single mailbox.
    pub fn enable_mailbox_interrupt(&mut self, mailbox: Mailbox, enable: bool) {
        let index = mailbox.index();
        if index < self.mailbox_offset() || index >= self.max_mailboxes() {
            return;
        }

        let keep_tx_interrupt = mb_is_tx(self.read_mailbox_cs(index));
        self.write_imask_bit(index, enable || keep_tx_interrupt);
    }

    /// Limits the controller to the first `last_mailbox` mailboxes.
    ///
    /// Values are clamped to the hardware range `1..=64`.
    pub fn set_max_mailboxes(&mut self, last_mailbox: u8) {
        let last_mailbox = last_mailbox.clamp(1, 64) - 1;
        let leave_freeze = self.reg(0x00) & MCR_FRZACK == 0;
        self.enter_freeze_mode();
        let fifo_enabled = self.reg(0x00) & MCR_FEN != 0;
        self.disable_fifo();
        self.write_iflag(self.read_iflag());
        self.modify_reg(0x00, |mcr| (mcr & !0x7F) | u32::from(last_mailbox));
        if fifo_enabled {
            self.enable_fifo();
        }
        if leave_freeze {
            self.exit_freeze_mode();
        }
    }

    /// Configures a mailbox for transmit, receive, or inactive use.
    pub fn set_mailbox(
        &mut self,
        mailbox: Mailbox,
        mode: MailboxMode,
        id_type: IdType,
    ) -> Result<(), Error> {
        let index = mailbox.index();
        if index < self.mailbox_offset() || index >= self.max_mailboxes() {
            return Err(Error::InvalidMailbox);
        }

        self.write_imask_bit(index, false);
        let _ = self.read_mailbox_cs(index);
        self.write_mailbox_id(index, 0);
        self.write_mailbox_word0(index, 0);
        self.write_mailbox_word1(index, 0);

        match (mode, id_type) {
            (_, IdType::Inactive) => self.write_mailbox_cs(index, mb_code(MB_CODE_RX_INACTIVE)),
            (MailboxMode::Rx, IdType::Standard) => self.write_mailbox_cs(index, rx_empty_cs(false)),
            (MailboxMode::Rx, IdType::Extended) => self.write_mailbox_cs(index, rx_empty_cs(true)),
            (MailboxMode::Tx, _) => {
                self.write_mailbox_cs(index, mb_code(MB_CODE_TX_INACTIVE));
                self.write_imask_bit(index, true);
            }
        }
        let _ = self.reg(0x08);
        self.write_iflag_bit(index);
        Ok(())
    }

    /// Sets the same accept / reject behavior for every mailbox filter.
    pub fn set_mailbox_filter_mode(&mut self, mode: FilterMode) {
        let mask = match mode {
            FilterMode::AcceptAll => 0,
            FilterMode::RejectAll => 0x3FFF_FFFF,
        };
        for index in self.mailbox_offset()..self.max_mailboxes() {
            self.write_rximr(index, mask);
        }
    }

    /// Sets the accept / reject behavior for one mailbox filter.
    pub fn set_mailbox_filter_state(
        &mut self,
        mailbox: Mailbox,
        mode: FilterMode,
    ) -> Result<(), Error> {
        let index = mailbox.index();
        if index < self.mailbox_offset() || index >= self.max_mailboxes() {
            return Err(Error::InvalidMailbox);
        }
        let mask = match mode {
            FilterMode::AcceptAll => 0,
            FilterMode::RejectAll => 0x3FFF_FFFF,
        };
        self.write_rximr(index, mask);
        Ok(())
    }

    /// Programs a mailbox filter from up to five identifier examples.
    pub fn set_mailbox_filter(&mut self, mailbox: Mailbox, ids: &[u32]) -> Result<(), Error> {
        self.set_mailbox_filter_inner(mailbox, ids, None)
    }

    /// Programs a mailbox filter with an explicit user mask.
    pub fn set_mailbox_user_filter(
        &mut self,
        mailbox: Mailbox,
        ids: &[u32],
        mask: u32,
    ) -> Result<(), Error> {
        self.set_mailbox_filter_inner(mailbox, ids, Some(mask))
    }

    /// Programs a mailbox filter to accept identifiers in the inclusive range.
    pub fn set_mailbox_filter_range(
        &mut self,
        mailbox: Mailbox,
        start: u32,
        end: u32,
    ) -> Result<(), Error> {
        let index = mailbox.index();
        if index < self.mailbox_offset()
            || index >= self.max_mailboxes()
            || mb_is_tx(self.read_mailbox_cs(index))
        {
            return Err(Error::InvalidMailbox);
        }
        let ide = self.read_mailbox_cs(index) & MB_CS_IDE != 0;
        let mask = if ide {
            mb_id_ext((!(start ^ end)) & 0x1FFF_FFFF)
        } else {
            mb_id_std((!(start ^ end)) & 0x7FF)
        };
        self.set_mailbox_filter_processing(index, start, mask);
        Ok(())
    }

    /// Transmits a frame using the first available TX mailbox.
    pub fn write(&mut self, message: &CanMessage) -> Result<(), Error> {
        if message.len > 8 {
            return Err(Error::InvalidMessageLength);
        }
        for index in self.mailbox_offset()..self.max_mailboxes() {
            if mb_is_tx(self.read_mailbox_cs(index)) {
                return self.write_mailbox(Mailbox(index), message);
            }
        }
        Err(Error::WouldBlock)
    }

    /// Transmits a frame through a specific TX mailbox.
    pub fn write_mailbox(&mut self, mailbox: Mailbox, message: &CanMessage) -> Result<(), Error> {
        let index = mailbox.index();
        if index >= self.max_mailboxes()
            || message.len > 8
            || !mb_is_tx(self.read_mailbox_cs(index))
        {
            return Err(Error::InvalidConfiguration);
        }

        self.write_iflag_bit(index);
        self.write_mailbox_cs(index, mb_code(MB_CODE_TX_INACTIVE));
        self.write_mailbox_id(
            index,
            if message.flags.extended {
                mb_id_ext(message.id)
            } else {
                mb_id_std(message.id)
            },
        );
        self.write_mailbox_word0(index, pack_word(&message.buf[0..4]));
        self.write_mailbox_word1(index, pack_word(&message.buf[4..8]));

        let mut code = (u32::from(message.len) & 0xF) << MB_CS_DLC_SHIFT;
        if message.flags.remote {
            code |= MB_CS_RTR;
        }
        if message.flags.extended {
            code |= MB_CS_IDE | MB_CS_SRR;
        }
        self.write_mailbox_cs(index, code | mb_code(MB_CODE_TX_ONCE));
        Ok(())
    }

    /// Reads the next completed RX mailbox frame, if any.
    pub fn read(&mut self) -> Option<CanMessage> {
        for index in self.mailbox_offset()..self.max_mailboxes() {
            let cs = self.read_mailbox_cs(index);
            let code = mb_code_value(cs);
            if code != MB_CODE_RX_FULL && code != MB_CODE_RX_OVERRUN {
                continue;
            }

            let id_reg = self.read_mailbox_id(index);
            let word0 = self.read_mailbox_word0(index);
            let word1 = self.read_mailbox_word1(index);
            let extended = cs & MB_CS_IDE != 0;
            let mut buf = [0u8; 8];
            unpack_word(word0, &mut buf[0..4]);
            unpack_word(word1, &mut buf[4..8]);

            let message = CanMessage {
                id: if extended {
                    id_reg & MB_ID_EXT_MASK
                } else {
                    (id_reg >> MB_ID_STD_SHIFT) & 0x7FF
                },
                timestamp: (cs & MB_CS_TIMESTAMP_MASK) as u16,
                idhit: 0,
                flags: MessageFlags {
                    extended,
                    remote: cs & MB_CS_RTR != 0,
                    overrun: code == MB_CODE_RX_OVERRUN,
                },
                len: ((cs >> MB_CS_DLC_SHIFT) & 0xF) as u8,
                buf,
                mb: index as i8,
                bus: I::BUS_NUMBER,
                seq: false,
            };

            self.write_mailbox_cs(index, rx_empty_cs(extended));
            let _ = self.reg(0x08);
            self.write_iflag_bit(index);
            return Some(message);
        }
        None
    }

    /// Returns pending controller events.
    ///
    /// This is currently a stub and always returns `0`.
    pub fn events(&mut self) -> u64 {
        0
    }

    /// Returns the current state of a mailbox.
    pub fn mailbox_state(&self, mailbox: Mailbox) -> Option<MailboxState> {
        let index = mailbox.index();
        if index >= self.max_mailboxes() {
            return None;
        }
        let cs = self.read_mailbox_cs(index);
        let extended = cs & MB_CS_IDE != 0;
        Some(match mb_code_value(cs) {
            MB_CODE_RX_INACTIVE => MailboxState::RxInactive,
            MB_CODE_RX_EMPTY => MailboxState::RxEmpty { extended },
            MB_CODE_RX_FULL => MailboxState::RxFull { extended },
            MB_CODE_RX_OVERRUN => MailboxState::RxOverrun { extended },
            MB_CODE_TX_INACTIVE => MailboxState::TxInactive,
            MB_CODE_TX_ABORT => MailboxState::TxAbort,
            MB_CODE_TX_ONCE => MailboxState::TxBusy { extended },
            other => MailboxState::Unknown(other),
        })
    }

    /// Returns `true` when the hardware receive FIFO is enabled.
    pub fn fifo_enabled(&self) -> bool {
        self.reg(0x00) & MCR_FEN != 0
    }

    /// Returns the number of active hardware mailboxes.
    pub fn max_mailboxes(&self) -> u8 {
        ((self.reg(0x00) & 0x7F) + 1) as u8
    }

    /// Returns the first mailbox index available for mailbox-based operations.
    ///
    /// When FIFO mode is enabled, the low mailbox region is reserved by the
    /// peripheral and this offset skips over it.
    pub fn mailbox_offset(&self) -> u8 {
        if !self.fifo_enabled() {
            return 0;
        }
        let rffn = ((self.reg(0x34) >> CTRL2_RFFN_SHIFT) & 0xF) + 1;
        let remaining = self
            .max_mailboxes()
            .saturating_sub(6)
            .saturating_sub((rffn * 2) as u8);
        self.max_mailboxes() - remaining
    }

    /// Reads the interrupt flag bitfield for all mailboxes.
    pub fn read_iflag(&self) -> u64 {
        ((self.reg(0x2C) as u64) << 32) | self.reg(0x30) as u64
    }

    /// Writes the interrupt flag bitfield for all mailboxes.
    pub fn write_iflag(&mut self, value: u64) {
        self.write_reg(0x2C, (value >> 32) as u32);
        self.write_reg(0x30, value as u32);
    }

    /// Reads the interrupt mask bitfield for all mailboxes.
    pub fn read_imask(&self) -> u64 {
        ((self.reg(0x24) as u64) << 32) | self.reg(0x28) as u64
    }

    /// Writes the interrupt mask bitfield for all mailboxes.
    pub fn write_imask(&mut self, value: u64) {
        self.write_reg(0x24, (value >> 32) as u32);
        self.write_reg(0x28, value as u32);
    }

    fn set_mailbox_filter_inner(
        &mut self,
        mailbox: Mailbox,
        ids: &[u32],
        user_mask: Option<u32>,
    ) -> Result<(), Error> {
        let index = mailbox.index();
        if index < self.mailbox_offset()
            || index >= self.max_mailboxes()
            || ids.is_empty()
            || ids.len() > 5
            || mb_is_tx(self.read_mailbox_cs(index))
        {
            return Err(Error::InvalidMailbox);
        }

        let extended = self.read_mailbox_cs(index) & MB_CS_IDE != 0;
        let or = ids.iter().fold(0u32, |acc, id| acc | *id);
        let and = ids
            .iter()
            .fold(if extended { 0x1FFF_FFFF } else { 0x7FF }, |acc, id| {
                acc & *id
            });
        let raw_mask = if extended {
            ((or ^ and) ^ 0x1FFF_FFFF) & user_mask.unwrap_or(0x1FFF_FFFF)
        } else {
            ((or ^ and) ^ 0x7FF) & user_mask.unwrap_or(0x7FF)
        };
        let mask = if extended {
            mb_id_ext(raw_mask)
        } else {
            mb_id_std(raw_mask)
        };
        self.set_mailbox_filter_processing(index, ids[0], mask);
        Ok(())
    }

    fn set_mailbox_filter_processing(&mut self, index: u8, filter_id: u32, mask: u32) {
        let extended = self.read_mailbox_cs(index) & MB_CS_IDE != 0;
        self.write_mailbox_id(
            index,
            if extended {
                mb_id_ext(filter_id)
            } else {
                mb_id_std(filter_id)
            },
        );
        self.write_rximr(index, mask);
        let _ = self.reg(0x08);
        self.write_iflag_bit(index);
    }

    fn clear_mailbox(&mut self, index: u8) {
        self.write_mailbox_cs(index, 0);
        self.write_mailbox_id(index, 0);
        self.write_mailbox_word0(index, 0);
        self.write_mailbox_word1(index, 0);
    }

    fn soft_reset(&mut self) {
        self.modify_reg(0x00, |mcr| mcr | MCR_SOFTRST);
        while self.reg(0x00) & MCR_SOFTRST != 0 {}
    }

    fn enter_freeze_mode(&mut self) {
        self.modify_reg(0x00, |mcr| mcr | MCR_FRZ | MCR_HALT);
        while self.reg(0x00) & MCR_FRZACK == 0 {}
    }

    fn exit_freeze_mode(&mut self) {
        self.modify_reg(0x00, |mcr| mcr & !MCR_HALT);
        while self.reg(0x00) & MCR_FRZACK != 0 {}
        while self.reg(0x00) & MCR_NOTRDY != 0 {}
    }

    fn read_mailbox_cs(&self, index: u8) -> u32 {
        self.mmio(mailbox_offset(index))
    }

    fn write_mailbox_cs(&mut self, index: u8, value: u32) {
        self.write_mmio(mailbox_offset(index), value);
    }

    fn read_mailbox_id(&self, index: u8) -> u32 {
        self.mmio(mailbox_offset(index) + 0x4)
    }

    fn write_mailbox_id(&mut self, index: u8, value: u32) {
        self.write_mmio(mailbox_offset(index) + 0x4, value);
    }

    fn read_mailbox_word0(&self, index: u8) -> u32 {
        self.mmio(mailbox_offset(index) + 0x8)
    }

    fn write_mailbox_word0(&mut self, index: u8, value: u32) {
        self.write_mmio(mailbox_offset(index) + 0x8, value);
    }

    fn read_mailbox_word1(&self, index: u8) -> u32 {
        self.mmio(mailbox_offset(index) + 0xC)
    }

    fn write_mailbox_word1(&mut self, index: u8, value: u32) {
        self.write_mmio(mailbox_offset(index) + 0xC, value);
    }

    fn write_rximr(&mut self, index: u8, value: u32) {
        self.write_mmio(0x880 + usize::from(index) * 4, value);
    }

    fn write_iflag_bit(&mut self, index: u8) {
        if index < 32 {
            self.write_reg(0x30, 1u32 << index);
        } else {
            self.write_reg(0x2C, 1u32 << (index - 32));
        }
    }

    fn write_imask_bit(&mut self, index: u8, set: bool) {
        let (offset, bit) = if index < 32 {
            (0x28, index)
        } else {
            (0x24, index - 32)
        };
        self.modify_reg(offset, |reg| {
            if set {
                reg | (1u32 << bit)
            } else {
                reg & !(1u32 << bit)
            }
        });
    }

    fn reg(&self, offset: usize) -> u32 {
        self.mmio(offset)
    }

    fn write_reg(&mut self, offset: usize, value: u32) {
        self.write_mmio(offset, value);
    }

    fn modify_reg(&mut self, offset: usize, f: impl FnOnce(u32) -> u32) {
        let current = self.reg(offset);
        self.write_reg(offset, f(current));
    }

    fn mmio(&self, offset: usize) -> u32 {
        unsafe { ptr::read_volatile((I::base() + offset) as *const u32) }
    }

    fn write_mmio(&mut self, offset: usize, value: u32) {
        unsafe { ptr::write_volatile((I::base() + offset) as *mut u32, value) }
    }
}
