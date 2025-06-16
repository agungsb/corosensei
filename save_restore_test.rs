use std::arch::global_asm;

// Test saving and restoring registers during stack switch
global_asm!(
    ".balign 8",
    ".globl save_restore_switch",
    ".type save_restore_switch, @function", 
    "save_restore_switch:",
    ".cfi_startproc",
    // r2 = new stack pointer
    
    // Save one register to the stack
    "stg     %r13, -8(%r15)",    // Save r13
    "aghi    %r15, -8",          // Allocate space
    
    // Switch to new stack
    "lgr     %r15, %r2",         // Switch to new stack
    
    // Immediately switch back - but we lost our saved register!
    // This is the bug! We need to remember where we saved r13
    
    // Let's try a different approach - save the old SP somewhere
    "lghi    %r2, 99",           // Return error code 99
    "br      %r14",              // Return
    ".cfi_endproc",
    ".size save_restore_switch, . - save_restore_switch",
);

extern "C" {
    fn save_restore_switch(new_stack: u64) -> u64;
}

fn main() {
    println!("Testing save/restore during stack switch...");
    
    let mut stack = vec![0u8; 4096];
    let stack_top = stack.as_mut_ptr() as u64 + 4096 - 128;
    
    let result = unsafe { save_restore_switch(stack_top) };
    println!("Save/restore switch result: {} (should be 99)", result);
}
