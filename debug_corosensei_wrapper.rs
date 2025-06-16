use std::arch::{global_asm, asm};
use std::mem::ManuallyDrop;

// Our working trampoline
global_asm!(
    ".balign 8",
    ".globl working_stack_call_trampoline", 
    ".type working_stack_call_trampoline, @function",
    "working_stack_call_trampoline:",
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
    ".size working_stack_call_trampoline, . - working_stack_call_trampoline",
);

// Simplified version of corosensei's wrapper pattern
union FuncOrResult<F, R> {
    func: ManuallyDrop<F>,
    result: ManuallyDrop<Result<R, String>>,
}

unsafe extern "C" fn corosensei_wrapper(ptr: *mut u8) {
    println!("Corosensei wrapper called with ptr: {:p}", ptr);
    
    // Exact same pattern as corosensei but simplified types
    type ClosureType = Box<dyn FnOnce() -> String>;
    
    let data = &mut *(ptr as *mut FuncOrResult<ClosureType, String>);
    let func = ManuallyDrop::take(&mut data.func);
    
    println!("About to call closure...");
    let result = func();  // This might be where infinite recursion happens!
    println!("Closure returned: {}", result);
    
    data.result = ManuallyDrop::new(Ok(result));
    println!("Wrapper completed successfully");
}

unsafe fn call_working_trampoline(arg: *mut u8, stack_base: u64, f: unsafe extern "C" fn(*mut u8)) {
    asm!(
        "brasl   %r14, working_stack_call_trampoline",
        "nop",
        in("r2") arg,
        in("r3") stack_base,
        in("r4") f,
        clobber_abi("C"),
    );
}

fn main() {
    println!("Testing corosensei wrapper pattern...");
    
    std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024)
        .spawn(|| {
            // Create the same data structure as corosensei
            let closure: Box<dyn FnOnce() -> String> = Box::new(|| {
                println!("Inside the actual closure!");
                "hello from closure".to_string()
            });
            
            let mut data = FuncOrResult {
                func: ManuallyDrop::new(closure),
            };
            
            let mut stack = vec![0u8; 4096];
            let stack_top = stack.as_mut_ptr() as u64 + 4096 - 128;
            
            println!("About to call corosensei wrapper via trampoline...");
            
            unsafe {
                call_working_trampoline(
                    &mut data as *mut _ as *mut u8,
                    stack_top,
                    corosensei_wrapper
                );
            }
            
            println!("Trampoline call completed, checking result...");
            
            let result: Result<String, String> = unsafe { ManuallyDrop::take(&mut data.result) };
            match result {
                Ok(s) => println!("Success: {}", s),
                Err(e) => println!("Error: {}", e),
            }
            
            println!("Test completed!");
        })
        .unwrap()
        .join()
        .unwrap();
        
    println!("All done!");
}
