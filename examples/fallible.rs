use std::io;

use fault_injection::{fallible, FAULT_INJECT_COUNTER};

fn do_io() -> io::Result<()> {
    Ok(())
}

fn main() -> io::Result<()> {
    FAULT_INJECT_COUNTER.store(1, std::sync::atomic::Ordering::Release);

    fallible!(do_io());

    Ok(())
}
