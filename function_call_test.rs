use std::arch::global_asm;

// Test with actual function call using working pattern
global_asm!(
    ".balign 8",
    ".globl stack_switch_with_call",
    ".type stack_switch_with_call, @function", 
    "stack_switch_with_call:",
    ".cfi_startproc",
    // r2 = function, r3 = new stack, r4 = argument
    
    // Save frame pointer properly
    "stg     %r13, -8(%r15)",    // Save r13 to old stack
    "aghi    %r15, -8",          // Adjust old SP  
    "lgr     %r13, %r15",        // Set frame pointer to adjusted old SP
    
    // Prepare function call
    "lgr     %r1, %r2",          // Save function in r1
    "lgr     %r2, %r4",          // Move argument to r2
    
    // Switch to new stack
    "lgr     %r15, %r3",         // Switch to new stack
    
    // Call function
    "lgr     %r14, %r1",         // Move function to r14
    "basr    %r14, %r14",        // Call function
    
    // Switch back using frame pointer  
    "lgr     %r15, %r13",        // Restore SP from frame pointer
    "lg      %r13, 0(%r15)",     // Restore original r13
    "aghi    %r15, 8",           // Restore SP
    
    "br      %r14",              // Return (r2 has return value)
    ".cfi_endproc",
    ".size stack_switch_with_call, . - stack_switch_with_call",
);

extern "C" {
    fn stack_switch_with_call(func: extern "C" fn(u64) -> u64, new_stack: u64, arg: u64) -> u64;
}

extern "C" fn test_function(arg: u64) -> u64 {
    println!("Test function called with: {}", arg);
    arg * 3
}

fn main() {
    println!("Testing stack switch with function call...");
    
    // Test direct call first
    let direct_result = test_function(7);
    println!("Direct call result: {}", direct_result);
    
    // Test with stack switch
    let mut stack = vec![0u8; 4096];
    let stack_top = stack.as_mut_ptr() as u64 + 4096 - 128;
    
    let result = unsafe { stack_switch_with_call(test_function, stack_top, 7) };
    println!("Stack switch call result: {}", result);
    println!("Test completed successfully!");
}
