use std::arch::global_asm;

// Test proper save/restore pattern
global_asm!(
    ".balign 8",
    ".globl proper_save_restore",
    ".type proper_save_restore, @function", 
    "proper_save_restore:",
    ".cfi_startproc",
    // r2 = new stack pointer
    
    // Method 1: Use a different register to remember old stack
    "lgr     %r1, %r15",         // Save old SP in r1 (caller-saved)
    
    // Save caller's r13 to old stack
    "stg     %r13, -8(%r15)",    // Save r13 to old stack
    "aghi    %r15, -8",          // Adjust old SP
    "lgr     %r13, %r15",        // Set frame pointer to adjusted old SP
    
    // Switch to new stack
    "lgr     %r15, %r2",         // Switch to new stack
    
    // Now switch back using frame pointer
    "lgr     %r15, %r13",        // Restore SP from frame pointer
    "lg      %r13, 0(%r15)",     // Restore original r13
    "aghi    %r15, 8",           // Restore SP
    
    "lghi    %r2, 42",           // Return success
    "br      %r14",              // Return
    ".cfi_endproc",
    ".size proper_save_restore, . - proper_save_restore",
);

extern "C" {
    fn proper_save_restore(new_stack: u64) -> u64;
}

fn main() {
    println!("Testing proper save/restore pattern...");
    
    let mut stack = vec![0u8; 4096];
    let stack_top = stack.as_mut_ptr() as u64 + 4096 - 128;
    
    let result = unsafe { proper_save_restore(stack_top) };
    println!("Proper save/restore result: {}", result);
    println!("Function completed successfully!");
}
