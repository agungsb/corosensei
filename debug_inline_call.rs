use std::arch::{global_asm, asm};

// Create a debugging version of the trampoline
global_asm!(
    ".balign 8",
    ".globl debug_stack_call_trampoline", 
    ".type debug_stack_call_trampoline, @function",
    "debug_stack_call_trampoline:",
    ".cfi_startproc",
    // Just return immediately without doing anything
    "lghi    %r2, 999",              // Return 999 as a marker
    "br      %r14",                  // Return immediately  
    ".cfi_endproc",
    ".size debug_stack_call_trampoline, . - debug_stack_call_trampoline",
);

// Test function
extern "C" fn test_func(_ptr: *mut u8) {
    println!("test_func called - THIS SHOULD NOT PRINT");
}

// Mimic corosensei's exact calling pattern
unsafe fn mimic_corosensei_call(arg: *mut u8, stack_base: u64, f: extern "C" fn(*mut u8)) {
    asm!(
        // This is exactly what corosensei does
        "brasl   %r14, debug_stack_call_trampoline",
        "nop",
        in("r2") arg,
        in("r3") stack_base,
        in("r4") f,
        clobber_abi("C"),
    );
}

fn main() {
    println!("Testing inline assembly call pattern...");
    
    // Test in a separate thread with large stack
    std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024)
        .spawn(|| {
            let mut stack = vec![0u8; 4096];
            let stack_top = stack.as_mut_ptr() as u64 + 4096 - 128;
            
            println!("About to call via inline assembly...");
            
            unsafe {
                mimic_corosensei_call(
                    std::ptr::null_mut(),
                    stack_top,
                    test_func
                );
            }
            
            println!("Inline assembly call completed!");
        })
        .unwrap()
        .join()
        .unwrap();
        
    println!("Test completed!");
}
