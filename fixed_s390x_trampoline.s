# Correct s390x stack_call_trampoline following x86_64 pattern
# r2 = arg, r3 = new_stack, r4 = function

# Save frame pointer to stack and set up new frame
stg     %r13, -8(%r15)     # Save current frame pointer
aghi    %r15, -8           # Allocate space (equivalent to push)
lgr     %r13, %r15         # Set frame pointer = current SP

# Switch to new stack
lgr     %r15, %r3          # Switch to new stack

# Call function (r2 already has arg)
basr    %r14, %r4          # Call function

# Restore original stack
lgr     %r15, %r13         # Restore SP from frame pointer
lg      %r13, 0(%r15)      # Restore original frame pointer (equivalent to pop)
aghi    %r15, 8            # Adjust SP
br      %r14               # Return
