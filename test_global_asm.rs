use std::arch::global_asm;

global_asm!(
    ".balign 8",
    ".globl test_global_func",
    ".type test_global_func, @function",
    "test_global_func:",
    ".cfi_startproc",
    "lghi    %r2, 123",     // Return 123
    "br      %r14",         // Return
    ".cfi_endproc",
    ".size test_global_func, . - test_global_func",
);

extern "C" {
    fn test_global_func() -> u64;
}

fn main() {
    println!("Testing global_asm...");
    let result = unsafe { test_global_func() };
    println!("Global assembly function returned: {}", result);
}
