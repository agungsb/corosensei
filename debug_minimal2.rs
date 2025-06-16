fn main() {
    println!("Testing function call through assembly...");
    
    extern "C" fn test_func(arg: *mut u8) {
        println!("Function called with arg: {:p}", arg);
    }
    
    let func_ptr = test_func as usize;
    let arg_ptr = 0x1234 as *mut u8;
    
    println!("Function ptr: {:x}", func_ptr);
    println!("Calling function...");
    
    unsafe {
        std::arch::asm!(
            "lgr    %r14, {func}",
            "basr   %r14, %r14",
            func = in(reg) func_ptr,
            in("r2") arg_ptr,
            clobber_abi("C"),
        );
    }
    
    println!("Function call completed");
}
