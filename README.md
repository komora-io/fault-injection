# fault-injection

[docs](https://docs.rs/fault-injection)

Similar to the `try!` macro or `?` operator,
but externally controllable to inject faults
during testing. Unlike the `try!` macro or `?`
operator, this additionally annotates the
description of the error to include the crate,
file name, and line number where the error
originated from to facilitate quick debugging.
It is specialized to work with `io::Result`
types, and will return an `io::Error` for faults,
with `into()` called similar to the `try!` macro
or `?` operator.
Decrements the `FAULT_INJECT_COUNTER` by `1`
(it is set to `u64::MAX` by default), and if
it hits 0, returns an `io::Error` with a kind
of `Other`. If `SLEEPINESS` is set to
something other than 0, this macro will also
inject weakly pseudorandom delays for
facilitating a basic form of concurrency testing.

# Examples
```
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

// this will return an injected error
fn use_it() -> std::io::Result<()> {
    set_trigger_function(trigger_fn);
    FAULT_INJECT_COUNTER.store(1, std::sync::atomic::Ordering::Release);

    fallible!(do_io());

    Ok(())
}
```
