use std::mem::ManuallyDrop;

// Mimic the exact pattern used by corosensei::on_stack
union FuncOrResult<F, R> {
    func: ManuallyDrop<F>,
    result: ManuallyDrop<Result<R, String>>,
}

unsafe extern "C" fn wrapper_string(ptr: *mut u8) {
    println!("Wrapper called with ptr: {:p}", ptr);
    
    type ClosureType = dyn FnOnce() -> String;
    
    // Read the function out of the union.
    let data = &mut *(ptr as *mut FuncOrResult<Box<ClosureType>, String>);
    let func = ManuallyDrop::take(&mut data.func);
    
    println!("About to call the actual function...");
    
    // Call it.
    let result = func();
    
    println!("Function completed, storing result...");
    
    // Store the result.
    data.result = ManuallyDrop::new(Ok(result));
    
    println!("Wrapper completed successfully");
}

// Use our working stack switch
use std::arch::global_asm;

global_asm!(
    ".balign 8",
    ".globl test_stack_call",
    ".type test_stack_call, @function", 
    "test_stack_call:",
    ".cfi_startproc",
    // r2 = arg, r3 = stack, r4 = function (same as corosensei)
    
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
    ".size test_stack_call, . - test_stack_call",
);

extern "C" {
    fn test_stack_call(arg: *mut u8, stack: u64, func: unsafe extern "C" fn(*mut u8));
}

fn main() {
    println!("Testing corosensei pattern...");
    
    // Set up the exact same pattern as corosensei
    let closure: Box<dyn FnOnce() -> String> = Box::new(|| {
        println!("Inside closure!");
        "hello".to_string()
    });
    
    let mut data = FuncOrResult {
        func: ManuallyDrop::new(closure),
    };
    
    // Allocate stack
    let mut stack = vec![0u8; 4096];
    let stack_top = stack.as_mut_ptr() as u64 + 4096 - 128;
    
    println!("Calling wrapper via stack switch...");
    
    unsafe {
        test_stack_call(
            &mut data as *mut _ as *mut u8,
            stack_top,
            wrapper_string
        );
    }
    
    println!("Stack call completed, checking result...");
    
    // Get the result
    let result: Result<String, String> = unsafe { ManuallyDrop::take(&mut data.result) };
    match result {
        Ok(s) => println!("Success: {}", s),
        Err(e) => println!("Error: {}", e),
    }
    
    println!("Test completed successfully!");
}
