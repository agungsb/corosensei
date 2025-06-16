use std::arch::global_asm;

global_asm!(
    ".balign 8",
    ".globl test_stack_switch",
    ".type test_stack_switch, @function", 
    "test_stack_switch:",
    ".cfi_startproc",
    // Simple stack switching test
    // r2 = function to call, r3 = new stack pointer
    
    // Save current state
    "stg     %r13, -8(%r15)",    // Save frame pointer
    "stg     %r14, -16(%r15)",   // Save return address
    "aghi    %r15, -16",         // Allocate frame
    "lgr     %r13, %r15",        // Set frame pointer
    
    // Switch to new stack (r3)
    "lgr     %r15, %r3",
    
    // Call function (r2) - just call it directly
    "lghi    %r2, 42",           // Set argument  
    "lgr     %r14, %r2",         // Move function to r14
    "basr    %r14, %r14",        // Call function
    
    // Restore original stack
    "lgr     %r15, %r13",        // Restore SP from frame pointer
    "lg      %r14, 8(%r15)",     // Restore return address
    "lg      %r13, 0(%r15)",     // Restore frame pointer  
    "aghi    %r15, 16",          // Adjust stack pointer
    "br      %r14",              // Return
    ".cfi_endproc",
    ".size test_stack_switch, . - test_stack_switch",
);

extern "C" {
    fn test_stack_switch(func: extern "C" fn(u64) -> u64, new_stack: u64) -> u64;
}

extern "C" fn simple_func(arg: u64) -> u64 {
    println!("Simple function called with: {}", arg);
    arg * 2
}

fn main() {
    println!("Testing stack switch...");
    
    // Allocate a small stack
    let mut stack = vec![0u8; 4096];
    let stack_top = stack.as_mut_ptr() as u64 + 4096;
    
    let result = unsafe { test_stack_switch(simple_func, stack_top) };
    println!("Stack switch result: {}", result);
}
