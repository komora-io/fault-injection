/// Facilitates fault injection. Every time any IO operation
/// is performed, this is decremented. If it hits 0, an
/// io::Error is returned from that IO operation. Use this
/// to ensure that error handling is being performed, by
/// running some test workload, checking the counter, and
/// then setting this to an incrementally-lower number while
/// asserting that your application properly handles the
/// error that will propagate up.
pub static FAULT_INJECT_COUNTER: core::sync::atomic::AtomicU64 =
    core::sync::atomic::AtomicU64::new(u64::MAX);

pub static SLEEPINESS: core::sync::atomic::AtomicU8 = core::sync::atomic::AtomicU8::new(0);

#[macro_export]
macro_rules! fallible {
    ($e:expr) => {{
        const CRATE_NAME: &str = if let Some(name) = core::option_env!("CARGO_CRATE_NAME") {
            name
        } else {
            ""
        };

        if fault_injection::FAULT_INJECT_COUNTER.fetch_sub(1, core::sync::atomic::Ordering::AcqRel)
            == 1
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("injected fault at {}:{}:{}", CRATE_NAME, file!(), line!()),
            ));
        }

        let sleepiness = fault_injection::SLEEPINESS.load(core::sync::atomic::Ordering::Acquire);
        if sleepiness > 0 {
            #[cfg(target_arch = "x86")]
            let rdtsc = unsafe { core::arch::x86::_rdtsc() as u16 };

            #[cfg(target_arch = "x86_64")]
            let rdtsc = unsafe { core::arch::x86_64::_rdtsc() as u16 };

            #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
            let rdtsc = 0;

            let random_sleeps = rdtsc.trailing_zeros() * sleepiness;

            for _ in 0..random_sleeps {
                std::thread::yield_now();
            }
        }

        // annotates io::Error to include the source of the error
        match $e {
            Ok(ok) => ok,
            Err(e) => {
                return Err(std::io::Error::new(
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
    }};
}
