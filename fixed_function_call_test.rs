use std::arch::global_asm;

// Fixed version - save return address properly
global_asm!(
    ".balign 8",
    ".globl stack_switch_with_call_fixed",
    ".type stack_switch_with_call_fixed, @function", 
    "stack_switch_with_call_fixed:",
    ".cfi_startproc",
    // r2 = function, r3 = new stack, r4 = argument
    
    // Save frame pointer AND return address
    "stg     %r14, -8(%r15)",    // Save return address
    "stg     %r13, -16(%r15)",   // Save frame pointer
    "aghi    %r15, -16",         // Adjust old SP  
    "lgr     %r13, %r15",        // Set frame pointer
    
    // Prepare function call
    "lgr     %r1, %r2",          // Save function in r1
    "lgr     %r2, %r4",          // Move argument to r2
    
    // Switch to new stack
    "lgr     %r15, %r3",         // Switch to new stack
    
    // Call function
    "lgr     %r14, %r1",         // Move function to r14
    "basr    %r14, %r14",        // Call function (this corrupts r14!)
    
    // Switch back using frame pointer  
    "lgr     %r15, %r13",        // Restore SP from frame pointer
    "lg      %r14, 8(%r15)",     // Restore ORIGINAL return address
    "lg      %r13, 0(%r15)",     // Restore original r13
    "aghi    %r15, 16",          // Restore SP
    
    "br      %r14",              // Return with original return address
    ".cfi_endproc",
    ".size stack_switch_with_call_fixed, . - stack_switch_with_call_fixed",
);

extern "C" {
    fn stack_switch_with_call_fixed(func: extern "C" fn(u64) -> u64, new_stack: u64, arg: u64) -> u64;
}

extern "C" fn test_function(arg: u64) -> u64 {
    println!("Test function called with: {}", arg);
    arg * 3
}

fn main() {
    println!("Testing FIXED stack switch with function call...");
    
    // Test direct call first
    let direct_result = test_function(7);
    println!("Direct call result: {}", direct_result);
    
    // Test with stack switch
    let mut stack = vec![0u8; 4096];
    let stack_top = stack.as_mut_ptr() as u64 + 4096 - 128;
    
    let result = unsafe { stack_switch_with_call_fixed(test_function, stack_top, 7) };
    println!("Stack switch call result: {}", result);
    println!("Test completed successfully!");
}
