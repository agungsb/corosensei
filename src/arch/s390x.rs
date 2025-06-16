//! Low-level s390x (IBM System Z) support.
//!
//! This file is heavily based on the AArch64 implementation, adapted for
//! s390x assembly language, calling conventions, and register usage.
//!
//! ## s390x Calling Convention
//!
//! - Stack grows downward (higher to lower addresses)
//! - 8-byte stack alignment
//! - Parameters passed in r2-r6, then stack
//! - Return value in r2
//! - Callee must save r6-r13, r14 (return address), r15 (stack pointer)
//! - r15 = Stack Pointer, r14 = Return Address, r13 = Frame Pointer
//!
//! ## Stack layout
//!
//! Here is what the layout of the stack looks like when a coroutine is
//! suspended:
//!
//! ```text
//! +--------------+  <- Stack base
//! | Initial func |
//! +--------------+
//! | Parent link  |
//! +--------------+
//! |              |
//! ~     ...      ~
//! |              |
//! +--------------+
//! | Saved r14    |
//! +--------------+
//! | Saved r13    |
//! +--------------+
//! | Saved r6     |
//! +--------------+
//! ```
//!
//! And this is the layout of the parent stack when a coroutine is running:
//!
//! ```text
//! |           |
//! ~    ...    ~
//! |           |
//! +-----------+
//! | Saved r6  |
//! +-----------+
//! | Saved r14 |
//! +-----------+
//! | Saved r13 |
//! +-----------+
//! ```
//!
//! And finally, this is the stack layout of a coroutine that has just been
//! initialized:
//!
//! ```text
//! +--------------+  <- Stack base
//! | Initial func |
//! +--------------+
//! | Parent link  |
//! +--------------+
//! |              |
//! ~ Initial obj  ~
//! |              |
//! +--------------+
//! | Initial r14  |  <- Points to stack_init_trampoline
//! +--------------+  <- Initial stack pointer
//! ```

use core::arch::{asm, global_asm};

use super::{allocate_obj_on_stack, push};
use crate::stack::{Stack, StackPointer};
use crate::unwind::{
    asm_may_unwind_root, asm_may_unwind_yield, cfi_reset_args_size_root, cfi_reset_args_size_yield,
    InitialFunc, StackCallFunc, TrapHandler,
};
use crate::util::EncodedValue;

pub const STACK_ALIGNMENT: usize = 8;
pub const PARENT_LINK_OFFSET: usize = 0;
pub type StackWord = u64;

global_asm!(
    ".balign 8",
    asm_function_begin!("stack_init_trampoline"),
    ".cfi_startproc",
    cfi_signal_frame!(),
    // At this point our register state contains the following:
    // - r15 points to the top of the parent stack.
    // - r14 contains the return address in the parent context.
    // - r13 and r6 contain their values from the parent context.
    // - r4 points to the top of the coroutine stack.
    // - r3 points to the base of our stack.
    // - r2 contains the argument passed from switch_and_link.
    //
    // Save the r13, r6 and r14 values of the parent context onto the parent
    // stack.
    "stmg    %r6,%r14,48(%r15)",     // Save r6-r14 to parent stack
    "aghi    %r15,-64",              // Allocate stack frame
    // Write the parent stack pointer to the parent link and adjust r3 to point
    // to the parent link.
    "la      %r1,64(%r15)",          // Get parent stack pointer
    "stg     %r1,-16(%r3)",          // Store to parent link
    "aghi    %r3,-16",               // Adjust r3 to point to parent link
    // Switch to the coroutine stack.
    "lgr     %r15,%r4",              // Switch to coroutine stack
    "aghi    %r15,8",                // Skip initial r14 value
    // Set up the frame pointer to point at the parent link.
    "lgr     %r13,%r3",              // r13 = parent link pointer
    // Define CFA for unwinding
    // 0x0f: DW_CFA_def_cfa_expression
    // 5: byte length of the following DWARF expression
    // 0x7d 0x00: DW_OP_breg13 (r13 + 0)
    // 0x06: DW_OP_deref
    // 0x23, 0x40: DW_OP_plus_uconst 64
    ".cfi_escape 0x0f, 5, 0x7d, 0x00, 0x06, 0x23, 0x40",
    // Tell the unwinder how to restore the registers that were saved on the
    // parent stack.
    ".cfi_offset r6, -16",
    ".cfi_offset r14, -8",
    ".cfi_offset r13, -24",
    // Set up the 3rd argument to the initial function to point to the object
    // that init_stack() set up on the stack.
    "lgr     %r4,%r15",              // r4 = stack pointer (3rd arg)
    // Call the initial function. r2 is already the first argument.
    "lg      %r1,8(%r3)",            // Load initial function address
    "basr    %r14,%r1",             // Call initial function
    // This point should never be reached in normal execution
    asm_function_alt_entry!("stack_init_trampoline_return"),
    ".short  0x0000",             // Trigger program check (should not reach here)
    ".cfi_endproc",
    asm_function_end!("stack_init_trampoline"),
);

global_asm!(
    ".balign 8",
    asm_function_begin!("stack_call_trampoline"),
    ".cfi_startproc",
    cfi_signal_frame!(),
    // At this point our register state contains the following:
    // - r15 points to the top of the parent stack.
    // - r13 holds its value from the parent context.
    // - r4 is the function that should be called.
    // - r3 points to the top of our stack.
    // - r2 contains the argument to be passed to the function.
    //
    // Create a stack frame and save registers.
    "stmg    %r13,%r14,104(%r15)",   // Save r13-r14 to stack
    "aghi    %r15,-160",             // Allocate stack frame
    "lgr     %r13,%r15",             // Set up frame pointer
    ".cfi_def_cfa r13, 160",
    ".cfi_offset r14, -8",
    ".cfi_offset r13, -16",
    // Switch to the new stack.
    "lgr     %r15,%r3",              // Switch to new stack
    // Call the function pointer. The argument is already in r2.
    "basr    %r14,%r4",             // Call function
    // Switch back to the original stack and restore registers.
    "lgr     %r15,%r13",             // Restore original stack
    "lmg     %r13,%r14,104(%r15)",   // Restore r13-r14
    "aghi    %r15,160",              // Restore stack pointer
    "br      %r14",                  // Return
    ".cfi_endproc",
    asm_function_end!("stack_call_trampoline"),
);

// These trampolines use a custom calling convention and should only be called
// with inline assembly.
extern "C" {
    fn stack_init_trampoline(arg: EncodedValue, stack_base: StackPointer, stack_ptr: StackPointer);
    static stack_init_trampoline_return: [u8; 0];
    #[allow(dead_code)]
    fn stack_call_trampoline(arg: *mut u8, sp: StackPointer, f: StackCallFunc);
}

#[inline]
pub unsafe fn init_stack<T>(stack: &impl Stack, func: InitialFunc<T>, obj: T) -> StackPointer {
    let mut sp = stack.base().get();

    // Initial function.
    push(&mut sp, Some(func as StackWord));

    // Placeholder for parent link.
    push(&mut sp, None);

    // Allocate space on the stack for the initial object, rounding to
    // STACK_ALIGNMENT.
    allocate_obj_on_stack(&mut sp, 8, obj);

    // The stack is aligned to STACK_ALIGNMENT at this point.
    debug_assert_eq!(sp % STACK_ALIGNMENT, 0);

    // Entry point called by switch_and_link().
    push(&mut sp, Some(stack_init_trampoline as StackWord));

    StackPointer::new_unchecked(sp)
}

#[inline]
pub unsafe fn switch_and_link(
    arg: EncodedValue,
    sp: StackPointer,
    stack_base: StackPointer,
) -> (EncodedValue, Option<StackPointer>) {
    let (ret_val, ret_sp);

    asm_may_unwind_root!(
        // DW_CFA_GNU_args_size 0
        cfi_reset_args_size_root!(),

        // Read the saved r14 from the coroutine stack and call it.
        "lg      %r1,0(%r4)",           // Load target address
        "basr    %r14,%r1",            // Call target

        // Upon returning, our register state contains the following:
        // - r4: Our stack pointer.
        // - r3: The top of the coroutine stack, or 0 if coming from
        //       switch_and_reset.
        // - r2: The argument passed from the coroutine.

        // Switch back to our stack.
        "lgr     %r15,%r4",            // Restore our stack pointer
        "aghi    %r15,24",             // Adjust for saved registers

        // Outputs
        inlateout("r2") arg => ret_val,
        lateout("r3") ret_sp,

        // Inputs
        in("r3") stack_base.get() as u64,
        in("r4") sp.get() as u64,

        // Clobbered registers
        lateout("r0") _, lateout("r1") _, lateout("r5") _,
        lateout("r7") _, lateout("r8") _, lateout("r9") _,
        lateout("r10") _,
        clobber_abi("C"),
    );

    (ret_val, StackPointer::new(ret_sp))
}

#[inline(always)]
pub unsafe fn switch_yield(arg: EncodedValue, parent_link: *mut StackPointer) -> EncodedValue {
    let ret_val;

    asm_may_unwind_yield!(
        // Save callee-saved registers while reserving space for return address.
        "stmg    %r6,%r14,48(%r15)",    // Save r6-r14
        "aghi    %r15,-64",             // Allocate stack frame

        // Store our return address at the expected position.
        "larl    %r1,0f",               // Get return address
        "stg     %r1,56(%r15)",         // Store return address

        // Get the parent stack pointer from the parent link.
        "lg      %r4,0(%r4)",           // Load parent stack pointer

        // Save our stack pointer to r3.
        "lgr     %r3,%r15",             // r3 = our stack pointer

        // Restore callee-saved registers from the parent stack.
        "lmg     %r6,%r14,48(%r4)",     // Restore r6-r14 from parent

        // DW_CFA_GNU_args_size 0
        cfi_reset_args_size_yield!(),

        // Return into the parent context
        "br      %r14",                 // Return to parent

        // This gets called by switch_and_link(). At this point our register
        // state contains the following:
        // - r15 points to the top of the parent stack.
        // - r14 contains the return address in the parent context.
        // - r13 and r6 contain their values from the parent context.
        // - r4 points to the top of the coroutine stack.
        // - r3 points to the base of our stack.
        // - r2 contains the argument passed from switch_and_link.
        "0:",

        // Save the parent context registers onto the parent stack.
        "stmg    %r6,%r14,48(%r15)",    // Save r6-r14 to parent stack
        "aghi    %r15,-64",             // Allocate parent stack frame

        // Write the parent stack pointer to the parent link.
        "la      %r1,64(%r15)",         // Get parent stack pointer
        "stg     %r1,-16(%r3)",         // Store to parent link

        // Load our callee-saved registers from the coroutine stack.
        "lmg     %r6,%r14,48(%r4)",     // Restore our r6-r14

        // Switch to the coroutine stack while popping the saved registers.
        "lgr     %r15,%r4",             // Switch to coroutine stack
        "aghi    %r15,64",              // Adjust for frame

        // Outputs
        inlateout("r2") arg => ret_val,

        // Inputs
        in("r4") parent_link as u64,

        // Clobbered registers
        lateout("r0") _, lateout("r1") _, lateout("r5") _,
        lateout("r7") _, lateout("r8") _, lateout("r9") _,
        lateout("r10") _,
        clobber_abi("C"),
    );

    ret_val
}

#[inline(always)]
pub unsafe fn switch_and_reset(arg: EncodedValue, parent_link: *mut StackPointer) -> ! {
    // Most of this code is identical to switch_yield(), refer to the
    // comments there. Only the differences are commented.
    asm!(
        // Load the parent context's stack pointer.
        "lg      %r4,0({parent_link})", // Load parent stack pointer

        // Restore callee-saved registers from the parent stack.
        "lmg     %r6,%r14,48(%r4)",     // Restore r6-r14 from parent

        // Return into the parent context
        "br      %r14",                 // Return to parent

        parent_link = in(reg) parent_link as u64,

        in("r2") arg,

        // Hard-code the returned stack pointer value to 0 to indicate that this
        // coroutine is done.
        in("r3") 0,

        options(noreturn),
    );
}

#[inline]
#[cfg(feature = "asm-unwind")]
pub unsafe fn switch_and_throw(
    forced_unwind: crate::unwind::ForcedUnwind,
    sp: StackPointer,
    stack_base: StackPointer,
) -> (EncodedValue, Option<StackPointer>) {
    extern "C-unwind" fn throw(forced_unwind: crate::unwind::ForcedUnwind) -> ! {
        extern crate std;
        use std::boxed::Box;
        std::panic::resume_unwind(Box::new(forced_unwind));
    }

    let (ret_val, ret_sp);

    asm_may_unwind_root!(
        // Set up a return address.
        "larl    %r14,0f",              // Get return address

        // Save the registers of the parent context.
        "stmg    %r6,%r14,48(%r15)",    // Save r6-r14
        "aghi    %r15,-64",             // Allocate stack frame

        // Update the parent link near the base of the coroutine stack.
        "la      %r1,64(%r15)",         // Get parent stack pointer
        "stg     %r1,-16(%r3)",         // Store to parent link

        // Load the coroutine registers, with the saved r14.
        "lg      %r14,56(%r4)",         // Load saved return address
        "lmg     %r6,%r13,48(%r4)",     // Load saved r6-r13

        // Switch to the coroutine stack while popping the saved registers.
        "lgr     %r15,%r4",             // Switch to coroutine stack
        "aghi    %r15,64",              // Adjust for frame

        // DW_CFA_GNU_args_size 0
        cfi_reset_args_size_root!(),

        // Simulate a call to the throw function.
        "brasl   %r14,{throw}",         // Call throw function

        // Upon returning, our register state is just like a normal return into
        // switch_and_link().
        "0:",

        // Switch back to our stack.
        "lgr     %r15,%r4",             // Restore our stack
        "aghi    %r15,24",              // Adjust for saved registers

        // Helper function to trigger stack unwinding.
        throw = sym throw,

        // Argument to pass to the throw function.
        in("r2") forced_unwind.0.get(),

        // Same output registers as switch_and_link().
        lateout("r2") ret_val,
        lateout("r3") ret_sp,

        // Stack pointer and stack base inputs for stack switching.
        in("r3") stack_base.get() as u64,
        in("r4") sp.get() as u64,

        // Clobbered registers
        lateout("r0") _, lateout("r1") _, lateout("r5") _,
        lateout("r7") _, lateout("r8") _, lateout("r9") _,
        lateout("r10") _,
        clobber_abi("C"),
    );

    (ret_val, StackPointer::new(ret_sp))
}

#[inline]
pub unsafe fn drop_initial_obj(
    _stack_base: StackPointer,
    stack_ptr: StackPointer,
    drop_fn: unsafe fn(ptr: *mut u8),
) {
    let ptr = (stack_ptr.get() as *mut u8).add(8);
    drop_fn(ptr);
}

/// Registers which must be updated upon return from a trap handler.
///
/// The exact set of registers that need to be updated varies depending on the
/// target. Note that *all* registers must be updated to the specified values,
/// otherwise behavior is undefined.
///
/// To catch any issues at compilation time, it is recommended to use Rust's
/// pattern matching syntax to extract the individual registers from this
/// struct.
///
/// ```
/// # use corosensei::trap::TrapHandlerRegs;
/// # let regs = TrapHandlerRegs { pc: 0, sp: 0, r2: 0, r3: 0, r13: 0, r14: 0 };
/// let TrapHandlerRegs { pc, sp, r2, r3, r13, r14 } = regs;
/// ```
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug)]
pub struct TrapHandlerRegs {
    pub pc: u64,
    pub sp: u64,
    pub r2: u64,
    pub r3: u64,
    pub r13: u64,
    pub r14: u64,
}

pub unsafe fn setup_trap_trampoline<T>(
    stack_base: StackPointer,
    val: T,
    handler: TrapHandler<T>,
) -> TrapHandlerRegs {
    // Preserve the top 16 bytes of the stack since they contain the parent
    // link.
    let parent_link = stack_base.get() - 16;

    // Everything below this can be overwritten. Write the object to the stack.
    let mut sp = parent_link;
    allocate_obj_on_stack(&mut sp, 8, val);
    let val_ptr = sp;

    // Set up registers for entry into the function.
    TrapHandlerRegs {
        pc: handler as u64,
        sp: sp as u64,
        r2: val_ptr as u64,
        r3: parent_link as u64,
        r13: parent_link as u64,
        r14: stack_init_trampoline_return.as_ptr() as u64,
    }
}

/// This function executes a function on the given stack. The argument is passed
/// through to the called function.
#[inline]
pub unsafe fn on_stack(arg: *mut u8, stack: impl Stack, f: StackCallFunc) {
    // Similar to AArch64, we need to be careful about unwinding information
    // when using .cfi_signal_frame.
    asm_may_unwind_root!(
        // DW_CFA_GNU_args_size 0
        cfi_reset_args_size_root!(),
        concat!("brasl   %r14,", asm_mangle!("stack_call_trampoline")),
        "nop",                          // NOP for unwinding safety
        in("r2") arg,
        in("r3") stack.base().get(),
        in("r4") f,
        clobber_abi("C"),
    );
}
