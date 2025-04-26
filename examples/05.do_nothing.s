# This program does nothing except increment the program
# counter (the special purpose 'pc' register). At the end, it
# displays the value of this register.

; The 'noop' instruction does nothing
noop
noop
noop
noop
noop

; Given that we have executed 5 instructions by this point, what
; will the value of 'pc' be? Does your guess match what you see
; on LCD 0?
copy pc out

; We are done, so halt the machine
halt
