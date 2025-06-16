use std::arch::{global_asm, asm};

// Test the function call part with stack switching
global_asm!(
    ".balign 8",
    ".globl debug_with_function_call", 
    ".type debug_with_function_call, @function",
    "debug_with_function_call:",
    ".cfi_startproc",
    // r2 = arg, r3 = stack, r4 = function
    
    // Save return address and frame pointer
    "stg     %r14,-8(%r15)",         // Save return address
    "stg     %r13,-16(%r15)",        // Save frame pointer
    "aghi    %r15,-16",              // Allocate 16-byte frame
    "lgr     %r13,%r15",             // Set frame pointer
    
    // Switch to new stack
    "lgr     %r15,%r3",              // Switch to new stack
    
    // Call function
    "basr    %r14,%r4",              // Call function
    
    // Switch back and restore
    "lgr     %r15,%r13",             // Restore SP from frame pointer
    "lg      %r14,8(%r15)",          // Restore return address
    "lg      %r13,0(%r15)",          // Restore frame pointer
    "aghi    %r15,16",               // Restore stack pointer
    "br      %r14",                  // Return
    ".cfi_endproc",
    ".size debug_with_function_call, . - debug_with_function_call",
);

extern "C" fn simple_test_func(_ptr: *mut u8) {
    println!("Simple test function called!");
}

unsafe fn test_with_function_call(arg: *mut u8, stack_base: u64, f: extern "C" fn(*mut u8)) {
    asm!(
        "brasl   %r14, debug_with_function_call",
        "nop",
        in("r2") arg,
        in("r3") stack_base,
        in("r4") f,
        clobber_abi("C"),
    );
}

fn main() {
    println!("Testing with function call...");
    
    std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024)
        .spawn(|| {
            let mut stack = vec![0u8; 4096];
            let stack_top = stack.as_mut_ptr() as u64 + 4096 - 128;
            
            println!("About to test with function call...");
            
            unsafe {
                test_with_function_call(
                    std::ptr::null_mut(),
                    stack_top,
                    simple_test_func
                );
            }
            
            println!("Function call test completed!");
        })
        .unwrap()
        .join()
        .unwrap();
        
    println!("Test completed!");
}
