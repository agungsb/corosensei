// Simplified s390x stack call trampoline
// Arguments: r2=arg, r3=stack_base, r4=function
// Save minimal state
stg     %r14,-8(%r15)        // Save return address
stg     %r13,-16(%r15)       // Save frame pointer  
stg     %r15,-24(%r15)       // Save current stack pointer
aghi    %r15,-24             // Allocate 24-byte frame

// Switch to new stack
lgr     %r15,%r3             // Switch to new stack

// Call function
basr    %r14,%r4             // Call function (r2 already has arg)

// Restore original stack
lg      %r15,0(%r15)         // Wrong! Need to get saved stack pointer
lg      %r14,16(%r15)        // Restore return address
lg      %r13,8(%r15)         // Restore frame pointer
aghi    %r15,24              // Restore stack pointer
br      %r14                 // Return
