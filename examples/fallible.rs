use std::io;

use fault_injection::{fallible, set_trigger_function, FAULT_INJECT_COUNTER};

fn trigger_fn(crate_name: &str, file_name: &str, line_number: u32) {
    println!(
        "fault injected at {} {} {}",
        crate_name, file_name, line_number
    );
}

fn do_io() -> io::Result<()> {
    Ok(())
}

fn main() -> io::Result<()> {
    set_trigger_function(trigger_fn);
    FAULT_INJECT_COUNTER.store(1, std::sync::atomic::Ordering::Release);

    fallible!(do_io());

    Ok(())
}
