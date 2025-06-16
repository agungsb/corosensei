fn main() {
    println!("Testing basic assembly call...");
    
    let result: i32;
    unsafe {
        std::arch::asm!(
            "lghi   {out}, 42",
            out = out(reg) result,
        );
    }
    println!("Basic assembly works: {}", result);

    // Test function pointer call
    unsafe fn test_func(_arg: *mut u8) {
        println!("Function called successfully!");
    }
    
    let func_ptr = test_func as *const ();
    unsafe {
        std::arch::asm!(
            "basr   %r14, {func}",
            func = in(reg) func_ptr,
            in("r2") std::ptr::null_mut::<u8>(),
            clobber_abi("C"),
        );
    }
}
