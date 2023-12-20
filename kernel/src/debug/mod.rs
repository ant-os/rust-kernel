use core::{sync::atomic::{AtomicBool, Ordering::SeqCst}, arch};

use crate::{serial::Port, device::character::UnsafeCharacterDevice};

static DEBUG_ENABLED: AtomicBool = AtomicBool::new(false);

pub fn toggle_debug(boolean: bool) {
    DEBUG_ENABLED.store(boolean, SeqCst);
}

pub fn is_debug_enabled() -> bool {
    DEBUG_ENABLED.load(SeqCst)
}
