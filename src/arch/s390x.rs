//! Low-level S390x support.
//!
//! This file is based on the aarch64 and riscv implementations.
//!
//! ## Stack layout
//!
//! Here is what the layout of the stack looks like when a coroutine is
//! suspended.
//!
//! ```text
//! +------------------+  <- Stack base
//! | Initial func     |
//! +------------------+
//! | Parent link      |
//! +------------------+
//! |                  |
//! ~       ...        ~
//! |                  |
//! +------------------+
//! | Padding          |
//! +------------------+
//! | Saved PC (r14)   |
//! +------------------+
//! | Saved FP (r11)   |
//! +------------------+
//! | Saved r6-r13, r15|  (Callee-saved registers)
//! +------------------+
//! ```
//!
//! And this is the layout of the parent stack when a coroutine is running:
//!
//! ```text
//! |           |
//! ~    ...    ~
//! |           |
//! +-----------+
//! | Padding   |
//! +-----------+
//! | Saved r6  |
//! +-----------+
//! | Saved PC  |
//! +-----------+
//! | Saved FP  |
//! +-----------+
//! ```
//!
//! And finally, this is the stack layout of a coroutine that has just been
//! initialized:
//!
//! ```text
//! +------------------+  <- Stack base
//! | Initial func     |
//! +------------------+
//! | Parent link      |
//! +------------------+
//! |                  |
//! ~ Initial obj      ~
//! |                  |
//! +------------------+
//! | Padding          |
//! +------------------+
//! | Initial PC       |
//! +------------------+
//! | Padding          |
//! +------------------+
//! | Padding          |
//! +------------------+  <- Initial stack pointer
//! ```

use core::arch::global_asm;

use super::{allocate_obj_on_stack, push};
use crate::stack::{Stack, StackPointer};
use crate::unwind::{
    asm_may_unwind_root, cfi_reset_args_size_root, InitialFunc, StackCallFunc, TrapHandler,
};
use crate::util::EncodedValue;

pub const STACK_ALIGNMENT: usize = 16;
pub const PARENT_LINK_OFFSET: usize = 16;
pub type StackWord = u64;

// The S390x processor specific ABI documentation can be found here:
// https://github.com/IBM/s390x-abi/releases/download/v1.4/abi-s390x-psabi.pdf
//
// Registers r6-r13 are callee-saved. We must also save r14 (link register)
// and r15 (stack pointer). This makes for 10 registers total.
// Each register is 8 bytes.
#[allow(dead_code)]
const SAVED_REGS_SIZE: usize = 10 * 8;

global_asm!(
    ".balign 4",
    asm_function_begin!("stack_init_trampoline"),
    ".cfi_startproc",
    cfi_signal_frame!(),
    // At this point our register state contains the following:
    // - sp (%r15) points to the top of the parent stack.
    // - lr (%r14) contains the return address in the parent context.
    // - r6-r13 contain their values from the parent context.
    // - r2 points to the top of the coroutine stack.
    // - r3 points to the base of our stack.
    // - r4 contains the argument passed from switch_and_link.

    // Save parent context's registers onto the parent stack.
    // We need to save r6-r13, r14 (lr), and r15 (sp).
    "stmg %r6, %r15, 48(%r15)", // stmg saves r6-r15 (10 regs)
    
    // Write the parent stack pointer to the parent link.
    // The parent link is at offset -16 from the stack base (%r3).
    "stg %r15, -16(%r3)",

    // Switch to the coroutine stack.
    "lgr %r15, %r2",

    // Set up the frame pointer to point at the stack base. This is needed for
    // the unwinding code below.
    "lgr %r11, %r3",

    // Adjust r3 to point to the parent link.
    "aghi %r3, -16",

    // Pop the initial PC from the coroutine stack into the link register.
    "lg %r14, 0(%r15)",
    "aghi %r15, 16", // Advance stack pointer

    // Set up the argument for the initial function.
    "lgr %r2, %r4", // move arg from r4 to r2
    // The address of the parent link is the second argument.
    "lgr %r3, %r11",
    "aghi %r3, -16",
    
    // Get the actual function pointer from the stack base and call it.
    "lg %r1, 8(%r11)",
    "basr %r14, %r1",

    asm_function_alt_entry!("stack_init_trampoline_return"),
    // This trap is necessary because of our use of .cfi_signal_frame earlier.
    ".long 0x00000000", // "unimp" equivalent is trapping with 0
    ".cfi_endproc",
    asm_function_end!("stack_init_trampoline"),
);


extern "C" {
    fn stack_init_trampoline(arg: EncodedValue, stack_base: StackPointer, stack_ptr: StackPointer);
    static stack_init_trampoline_return: [u8; 0];
}

#[inline]
pub unsafe fn init_stack<T>(stack: &impl Stack, func: InitialFunc<T>, obj: T) -> StackPointer {
    let mut sp = stack.base().get();

    // Initial function.
    push(&mut sp, Some(func as StackWord));

    // Placeholder for parent link.
    push(&mut sp, None);

    // Allocate space on the stack for the initial object.
    allocate_obj_on_stack(&mut sp, PARENT_LINK_OFFSET, obj);

    debug_assert_eq!(sp % STACK_ALIGNMENT, 0);

    // Padding.
    push(&mut sp, None);

    // Entry point called by switch_and_link().
    push(&mut sp, Some(stack_init_trampoline as StackWord));

    // Add padding to align the stack as needed by the trampoline.
    push(&mut sp, None);

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
        cfi_reset_args_size_root!(),

        // Load the target PC from the coroutine stack and call it.
        // The ABI expects the new stack pointer in r2.
        "lg %r1, 0(%r4)", // Use r4 for sp instead of r2 to avoid conflict
        "basr %r14, %r1",

        // Upon returning from the coroutine, our register state is:
        // - %r2: The argument passed from the coroutine.
        // - %r3: The coroutine's stack pointer, or 0 if it's finished.
        
        // Restore parent registers from the stack.
        // The stack pointer is already correct.
        "lmg %r6, %r15, 48(%r15)",

        // The return value from the coroutine is in %r2.
        inlateout("r2") arg => ret_val,
        // The new stack pointer for the coroutine is returned in %r3.
        lateout("r3") ret_sp,
        
        // Pass the target stack pointer in a non-conflicting register, r4.
        in("r4") sp.get() as u64,
        // Pass the stack base in r3.
        in("r3") stack_base.get() as u64,

        // Mark all other registers as clobbered.
        clobber_abi("C"),
    );
    
    (ret_val, StackPointer::new(ret_sp))
}

#[inline(always)]
pub unsafe fn switch_yield(_arg: EncodedValue, _parent_link: *mut StackPointer) -> EncodedValue {
    // This function needs to be implemented for full context switching.
    // For now, we will panic as it is a complex piece of assembly.
    // A proper implementation would save the current state and jump to the parent.
    panic!("switch_yield is not yet implemented for s390x");
}

#[inline(always)]
pub unsafe fn switch_and_reset(_arg: EncodedValue, _parent_link: *mut StackPointer) -> ! {
    // This function needs to be implemented for full context switching.
    // For now, we will panic as it is a complex piece of assembly.
    // A proper implementation would switch to the parent context and not return.
    panic!("switch_and_reset is not yet implemented for s390x");
}

#[inline]
#[cfg(feature = "asm-unwind")]
pub unsafe fn switch_and_throw(
    _forced_unwind: crate::unwind::ForcedUnwind,
    _sp: StackPointer,
    _stack_base: StackPointer,
) -> (EncodedValue, Option<StackPointer>) {
    panic!("asm-unwind is not yet supported on s390x");
}


#[inline]
pub unsafe fn drop_initial_obj(
    _stack_base: StackPointer,
    stack_ptr: StackPointer,
    drop_fn: unsafe fn(ptr: *mut u8),
) {
    // The initial object is located after the saved registers and padding.
    // This offset needs to be precise.
    let ptr = (stack_ptr.get() as *mut u8).add(16); // Placeholder offset
    drop_fn(ptr);
}

/// Registers which must be updated upon return from a trap handler.
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(C)]
pub struct TrapHandlerRegs {
    pub pc: u64,
    pub sp: u64,
    pub r2: u64,
    pub r3: u64,
    pub r11: u64,
    pub r14: u64,
}

pub unsafe fn setup_trap_trampoline<T>(
    stack_base: StackPointer,
    val: T,
    handler: TrapHandler<T>,
) -> TrapHandlerRegs {
    // Preserve the top 16 bytes of the stack since they contain the parent link.
    let parent_link = stack_base.get() - PARENT_LINK_OFFSET;

    // Write the object to the stack.
    let mut sp = parent_link;
    allocate_obj_on_stack(&mut sp, PARENT_LINK_OFFSET, val);
    let val_ptr = sp;
    
    debug_assert_eq!(sp % STACK_ALIGNMENT, 0);

    // Set up registers for entry into the trap handler function.
    TrapHandlerRegs {
        pc: handler as u64,
        sp: sp as u64,
        r2: val_ptr as u64,      // 1st arg: pointer to the value
        r3: parent_link as u64,  // 2nd arg: pointer to the parent link
        r11: stack_base.get() as u64, // Frame pointer
        r14: stack_init_trampoline_return.as_ptr() as u64, // Dummy return address
    }
}


/// This function executes a function on the given stack.
#[inline]
pub unsafe fn on_stack(_arg: *mut u8, _stack: impl Stack, _f: StackCallFunc) {
     // This function needs to be implemented for full context switching.
    // For now, we will panic as it is a complex piece of assembly.
    panic!("on_stack is not yet implemented for s390x");
}

