fn main() {
    println!("Testing inline assembly patterns...");
    
    // Test 1: Basic register operations
    let result: u64;
    unsafe {
        std::arch::asm!(
            "lghi   {out}, 42",
            out = out(reg) result,
        );
    }
    println!("Basic register load: {}", result);
    
    // Test 2: Memory operations (simplified)
    let mut value = 123u64;
    let temp: u64;
    unsafe {
        std::arch::asm!(
            "stg    {input}, -8(%r15)",
            "lg     {output}, -8(%r15)",
            input = in(reg) value,
            output = out(reg) temp,
        );
    }
    println!("Memory store/load: {}", temp);
    
    // Test 3: Stack manipulation (the problematic area)
    let original_sp: u64;
    let modified_sp: u64;
    unsafe {
        std::arch::asm!(
            "lgr    {orig}, %r15",      // Save original SP
            "aghi   %r15, -16",         // Allocate frame
            "lgr    {mod}, %r15",       // Get modified SP
            "aghi   %r15, 16",          // Restore frame
            orig = out(reg) original_sp,
            mod = out(reg) modified_sp,
        );
    }
    println!("Stack manipulation - Original: {:#x}, Modified: {:#x}, Diff: {}", 
             original_sp, modified_sp, original_sp - modified_sp);
    
    // Test 4: Function pointer call (most complex)
    extern "C" fn test_func(arg: u64) -> u64 {
        println!("Test function called with: {}", arg);
        arg * 2
    }
    
    let func_ptr = test_func as usize;
    let input = 21u64;
    let output: u64;
    
    unsafe {
        std::arch::asm!(
            "lgr    %r14, {func}",
            "basr   %r14, %r14",
            func = in(reg) func_ptr,
            inlateout("r2") input => output,
        );
    }
    println!("Function call result: {}", output);
}
