use std::arch::{global_asm, asm};

// Test just the stack switching part
global_asm!(
    ".balign 8",
    ".globl debug_stack_switch_only", 
    ".type debug_stack_switch_only, @function",
    "debug_stack_switch_only:",
    ".cfi_startproc",
    // r2 = arg, r3 = stack, r4 = function
    
    // Save return address and frame pointer
    "stg     %r14,-8(%r15)",         // Save return address
    "stg     %r13,-16(%r15)",        // Save frame pointer
    "aghi    %r15,-16",              // Allocate 16-byte frame
    "lgr     %r13,%r15",             // Set frame pointer
    
    // Switch to new stack
    "lgr     %r15,%r3",              // Switch to new stack
    
    // DON'T call function - just switch back immediately
    
    // Switch back and restore
    "lgr     %r15,%r13",             // Restore SP from frame pointer
    "lg      %r14,8(%r15)",          // Restore return address
    "lg      %r13,0(%r15)",          // Restore frame pointer
    "aghi    %r15,16",               // Restore stack pointer
    
    "lghi    %r2, 888",              // Return 888 as success marker
    "br      %r14",                  // Return
    ".cfi_endproc",
    ".size debug_stack_switch_only, . - debug_stack_switch_only",
);

extern "C" fn test_func(_ptr: *mut u8) {
    println!("test_func called - THIS SHOULD NOT PRINT");
}

unsafe fn test_stack_switch_only(arg: *mut u8, stack_base: u64, f: extern "C" fn(*mut u8)) {
    asm!(
        "brasl   %r14, debug_stack_switch_only",
        "nop",
        in("r2") arg,
        in("r3") stack_base,
        in("r4") f,
        clobber_abi("C"),
    );
}

fn main() {
    println!("Testing stack switching only...");
    
    std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024)
        .spawn(|| {
            let mut stack = vec![0u8; 4096];
            let stack_top = stack.as_mut_ptr() as u64 + 4096 - 128;
            
            println!("About to test stack switching...");
            
            unsafe {
                test_stack_switch_only(
                    std::ptr::null_mut(),
                    stack_top,
                    test_func
                );
            }
            
            println!("Stack switching test completed!");
        })
        .unwrap()
        .join()
        .unwrap();
        
    println!("Test completed!");
}
