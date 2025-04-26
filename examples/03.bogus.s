# This example shows what happens when your program has a bug.

; The 'put' instruction can only write to general-purpose
; registers. When we try to execute this instruction, the cpu
; will generate an error and refuse to keep running our program.
put 16 out
