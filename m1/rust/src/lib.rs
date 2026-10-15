#![no_std]

extern "C" {
    fn puts(s: *const u8) -> i32;
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

/// Called from the C side: prints a greeting through DJGPP's libc and returns a floating-point result.
#[no_mangle]
pub extern "C" fn rust_hello() -> f64 {
    unsafe { puts(b"Hello from Rust in DOSBox\0".as_ptr()) };
    let x: f64 = 2.0;
    x * 1.5 + 0.25
}
