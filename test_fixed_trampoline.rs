use std::arch::global_asm;

global_asm!(
    ".balign 8",
    ".globl test_stack_switch",
    ".type test_stack_switch, @function", 
    "test_stack_switch:",
    ".cfi_startproc",
    // Test stack switching following x86_64 pattern
    // r2 = function to call, r3 = new stack pointer, r4 = argument
    
    // Save frame pointer and set up frame (like x86_64 push rbp; mov rbp,rsp)
    "stg     %r13, -8(%r15)",    // Save frame pointer
    "aghi    %r15, -8",          // Allocate space
    "lgr     %r13, %r15",        // Set frame pointer = current SP
    
    // Prepare for function call
    "lgr     %r1, %r2",          // Save function pointer in r1
    "lgr     %r2, %r4",          // Move argument to r2 (first arg register)
    
    // Switch to new stack
    "lgr     %r15, %r3",
    
    // Call function
    "lgr     %r14, %r1",         // Move function to r14 for call
    "basr    %r14, %r14",        // Call function
    
    // Restore original stack (like x86_64 mov rsp,rbp; pop rbp)
    "lgr     %r15, %r13",        // Restore SP from frame pointer
    "lg      %r13, 0(%r15)",     // Restore frame pointer
    "aghi    %r15, 8",           // Adjust SP
    "br      %r14",              // Return
    ".cfi_endproc",
    ".size test_stack_switch, . - test_stack_switch",
);

extern "C" {
    fn test_stack_switch(func: extern "C" fn(u64) -> u64, new_stack: u64, arg: u64) -> u64;
}

extern "C" fn simple_func(arg: u64) -> u64 {
    println!("Simple function called with: {}", arg);
    arg * 2
}

fn main() {
    println!("Testing fixed stack switch...");
    
    // Test without stack switching first
    let result1 = simple_func(21);
    println!("Direct call result: {}", result1);
    
    // Now test with stack switching
    let mut stack = vec![0u8; 4096];
    let stack_top = stack.as_mut_ptr() as u64 + 4096 - 128; // Leave some space
    
    println!("Function address: {:p}", simple_func as *const ());
    println!("Stack top: {:#x}", stack_top);
    
    let result = unsafe { test_stack_switch(simple_func, stack_top, 21) };
    println!("Stack switch result: {}", result);
}
