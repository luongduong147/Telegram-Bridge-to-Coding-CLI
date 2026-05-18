use std::sync::atomic::{AtomicBool, Ordering};

pub static INTERRUPT_FLAG: AtomicBool = AtomicBool::new(false);

pub fn set_interrupt() {
    INTERRUPT_FLAG.store(true, Ordering::SeqCst);
}

pub fn clear_interrupt() {
    INTERRUPT_FLAG.store(false, Ordering::SeqCst);
}

pub fn is_interrupted() -> bool {
    INTERRUPT_FLAG.load(Ordering::SeqCst)
}
