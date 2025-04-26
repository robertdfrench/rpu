# This example shows the basics of the 'jump' instruction.  Read
# through this code to determine whether the number 11 or 22
# will be written to LCD 1, and pay attention to the value of
# the Special Purpose 'pc' register as the program executes.

; Select LCD 1 as our output device
put 1 dvc

; Store the number 11 in general purpose register 0
put 11 gp0

; Store the address (the location in memory) of the instruction
; associated with the ".UPDATE_LCD" label into gp2. We will use
; this address with the jump instruction to tell the computer
; which instruction it should execute next.
put .UPDATE_LCD gp1

; Jump to the address in register gp1 if and only if the value
; of gp2 is zero. Notice that we have not yet written anything
; into gp2 -- what does that mean about its value?
jump gp1 gp2

; Store the number 22 in general purpose register 0. Does this
; instruction even get executed?
put 22 gp0

; Update LCD 1 with the contents of gp0
copy gp0 out .UPDATE_LCD

; Halt the machine
halt
