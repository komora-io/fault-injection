/// Facilitates fault injection. Every time any IO operation
/// is performed, this is decremented. If it hits 0, an
/// io::Error is returned from that IO operation. Use this
/// to ensure that error handling is being performed, by
/// running some test workload, checking the counter, and
/// then setting this to an incrementally-lower number while
/// asserting that your application properly handles the
/// error that will propagate up. Defaults to `u64::MAX`,
/// so it won't be hit normally unless you do something 6 billion
/// times per second for 100 years. If you're building something
/// like that, maybe consider re-setting this to `u64::MAX` every
/// few decades for safety.
pub static FAULT_INJECT_COUNTER: core::sync::atomic::AtomicU64 =
    core::sync::atomic::AtomicU64::new(u64::MAX);

/// Facilitates delay injection. If you set this to something other
/// than 0, the `fallible!` macro will randomly call `std::thread::yield_now()`,
/// with the nubmer of times being multiplied by this value. You should not
/// need to set it very high to get a lot of delays, but you'll need
/// to play with the number sometimes for specific concurrent systems under test.
pub static SLEEPINESS: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);

#[doc(hidden)]
pub type Trigger = fn(&'static str, &'static str, u32);

/// This function will be called any time the [`FAULT_INJECT_COUNTER`] reaches 0
/// and an error is injected. You can use this to re-set the counter for deep
/// fault tree enumeration, test auditing, etc...
///
/// The function accepts the crate name, file name, and line number as arguments.
///
/// [`FAULT_INJECT_COUNTER`]: FAULT_INJECT_COUNTER
pub fn set_trigger_function(
    f: fn(crate_name: &'static str, file_name: &'static str, line_number: u32),
) {
    TRIGGER_FN.store(f as usize as _, core::sync::atomic::Ordering::Release);
}

#[doc(hidden)]
pub static TRIGGER_FN: core::sync::atomic::AtomicPtr<Trigger> =
    core::sync::atomic::AtomicPtr::new(0 as usize as _);

/// Similar to the `try!` macro or `?` operator,
/// but externally controllable to inject faults
/// during testing. Unlike the `try!` macro or `?`
/// operator, this additionally annotates the
/// description of the error to include the crate,
/// file name, and line number where the error
/// originated from to facilitate quick debugging.
/// It is specialized to work with `io::Result`
/// types, and will return an `io::Error` for faults,
/// with `into()` called similar to the `try!` macro
/// or `?` operator.
/// Decrements the [`FAULT_INJECT_COUNTER`] by `1`
/// (it is set to `u64::MAX` by default), and if
/// it hits 0, returns an `io::Error` with a kind
/// of `Other`. If [`SLEEPINESS`] is set to
/// something other than 0, this macro will also
/// inject weakly pseudorandom delays for
/// facilitating a basic form of concurrency testing.
///
/// # Examples
/// ```
/// use std::io;
///
/// use fault_injection::{fallible, set_trigger_function, FAULT_INJECT_COUNTER};
///
/// fn trigger_fn(crate_name: &str, file_name: &str, line_number: u32) {
///     println!(
///         "fault injected at {} {} {}",
///         crate_name, file_name, line_number
///     );
/// }
///
/// fn do_io() -> io::Result<()> {
///     Ok(())
/// }
///
/// // this will return an injected error
/// fn use_it() -> std::io::Result<()> {
///     set_trigger_function(trigger_fn);
///     FAULT_INJECT_COUNTER.store(1, std::sync::atomic::Ordering::Release);
///
///     fallible!(do_io());
///
///     Ok(())
/// }
///
/// assert!(use_it().is_err());
/// ```
///
///
/// [`FAULT_INJECT_COUNTER`]: FAULT_INJECT_COUNTER
/// [`SLEEPINESS`]: SLEEPINESS
#[macro_export]
macro_rules! fallible {
    ($e:expr) => {{
        fault_injection::maybe!($e)?
    }};
}

/// Performs the same fault injection as [`fallible`] but does not
/// early-return, and does not try to convert the injected
/// `io::Error` using the `?` operator.
///
/// [`fallible`]: fallible
#[macro_export]
macro_rules! maybe {
    ($e:expr) => {{
        let sleepiness = fault_injection::SLEEPINESS.load(core::sync::atomic::Ordering::Acquire);
        if sleepiness > 0 {
            #[cfg(target_arch = "x86")]
            let rdtsc = unsafe { core::arch::x86::_rdtsc() as u16 };

            #[cfg(target_arch = "x86_64")]
            let rdtsc = unsafe { core::arch::x86_64::_rdtsc() as u16 };

            #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
            let rdtsc = 0b10;

            let random_sleeps = rdtsc.trailing_zeros() as u32 * sleepiness;

            for _ in 0..random_sleeps {
                std::thread::yield_now();
            }
        }

        const CRATE_NAME: &str = if let Some(name) = core::option_env!("CARGO_CRATE_NAME") {
            name
        } else {
            ""
        };

        if fault_injection::FAULT_INJECT_COUNTER.fetch_sub(1, core::sync::atomic::Ordering::AcqRel)
            == 1
        {
            let trigger_fn = fault_injection::TRIGGER_FN.load(core::sync::atomic::Ordering::Acquire);
            if !trigger_fn.is_null() {
                unsafe {
                    let f: fault_injection::Trigger = std::mem::transmute(trigger_fn);
                    (f)(CRATE_NAME, file!(), line!());
                }
            }

            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("injected fault at {}:{}:{}", CRATE_NAME, file!(), line!()),
            ))
        } else {
            // annotates io::Error to include the source of the error
            match $e {
                Ok(ok) => Ok(ok),
                Err(e) => {
                    Err(std::io::Error::new(
                        e.kind(),
                        format!(
                            "{}:{}:{} -> {}",
                            CRATE_NAME,
                            file!(),
                            line!(),
                            e.to_string()
                        ),
                    ))
                }
            }
        }
    }};
}
