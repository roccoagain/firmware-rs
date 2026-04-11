use teensy4_bsp::ral;

const RESET_COOKIE: u32 = 0x0BAD_00F1;
const SCB_AIRCR: *mut u32 = 0xE000_ED0C as *mut u32;
const AIRCR_VECTKEY_SYSRESETREQ: u32 = 0x05FA_0004;

/// Request a system reset via SCB AIRCR.
///
/// Writes a reboot marker to `SRC.GPR5` first so post-reset code can
/// distinguish this reset path from other `SYSRESETREQ` / lockup resets.
pub fn reset_teensy() -> ! {
    unsafe {
        let src = ral::src::SRC::instance();
        ral::write_reg!(ral::src, src, GPR5, RESET_COOKIE);
        core::ptr::write_volatile(SCB_AIRCR, AIRCR_VECTKEY_SYSRESETREQ);
    }

    loop {
        core::hint::spin_loop();
    }
}
