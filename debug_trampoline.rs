use std::arch::global_asm;

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

// Simple test function
extern "C" fn test_func(_ptr: *mut u8) {
    println!("test_func called - THIS SHOULD NOT PRINT");
}

extern "C" {
    fn debug_stack_call_trampoline(arg: *mut u8, stack: u64, func: extern "C" fn(*mut u8)) -> u64;
}

// Mimic corosensei's on_stack call pattern
fn test_call() {
    let mut stack = vec![0u8; 4096];
    let stack_top = stack.as_mut_ptr() as u64 + 4096 - 128;
    
    println!("About to call debug trampoline...");
    
    let result = unsafe {
        debug_stack_call_trampoline(
            std::ptr::null_mut(),
            stack_top,
            test_func
        )
    };
    
    println!("Trampoline returned: {}", result);
}

fn main() {
    println!("Testing debug trampoline...");
    
    // Test in a separate thread with large stack
    std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024)
        .spawn(|| {
            test_call();
        })
        .unwrap()
        .join()
        .unwrap();
        
    println!("Test completed!");
}
