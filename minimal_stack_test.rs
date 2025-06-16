use std::arch::global_asm;

// Simplest possible stack switch test - no function call
global_asm!(
    ".balign 8",
    ".globl simple_stack_switch",
    ".type simple_stack_switch, @function", 
    "simple_stack_switch:",
    ".cfi_startproc",
    // r2 = new stack pointer
    // Just switch to new stack and immediately return
    
    // Save current stack pointer 
    "lgr     %r1, %r15",         // Save current SP in r1
    
    // Switch to new stack
    "lgr     %r15, %r2",         // Switch to new stack
    
    // Immediately switch back
    "lgr     %r15, %r1",         // Restore original SP
    
    // Return success value
    "lghi    %r2, 42",           // Return 42
    "br      %r14",              // Return
    ".cfi_endproc",
    ".size simple_stack_switch, . - simple_stack_switch",
);

extern "C" {
    fn simple_stack_switch(new_stack: u64) -> u64;
}

fn main() {
    println!("Testing simplest stack switch...");
    
    // Allocate a small stack
    let mut stack = vec![0u8; 4096];
    let stack_top = stack.as_mut_ptr() as u64 + 4096 - 128;
    
    println!("Original stack area: around {:#x}", &stack as *const _ as u64);
    println!("New stack top: {:#x}", stack_top);
    
    let result = unsafe { simple_stack_switch(stack_top) };
    println!("Simple stack switch result: {}", result);
}
