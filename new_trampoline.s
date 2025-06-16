global_asm!(
    ".balign 8",
    asm_function_begin!("stack_call_trampoline"),
    ".cfi_startproc",
    cfi_signal_frame!(),
    // Simple s390x stack switching implementation
    // r2 = arg, r3 = new stack top, r4 = function
    // Save return address and frame pointer
    "stg     %r14,-8(%r15)",         // Save return address
    "stg     %r13,-16(%r15)",        // Save frame pointer
    "lgr     %r13,%r15",             // Current stack -> frame pointer
    "aghi    %r15,-16",              // Allocate minimal frame
    ".cfi_def_cfa r15, 16",
    ".cfi_offset r14, -8",
    ".cfi_offset r13, -16",
    // Switch to new stack
    "lgr     %r15,%r3",              // Switch to new stack
    // Call function (r2 already has arg, r4 has function)
    "lgr     %r14,%r4",              // Move function to r14
    "basr    %r14,%r14",             // Call function via r14
    // Restore original stack
    "lgr     %r15,%r13",             // Restore stack from frame pointer
    "lg      %r14,8(%r15)",          // Restore return address
    "lg      %r13,0(%r15)",          // Restore original frame pointer
    "aghi    %r15,16",               // Restore stack pointer
    "br      %r14",                  // Return
    ".cfi_endproc",
    asm_function_end!("stack_call_trampoline"),
);
