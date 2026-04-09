# This example adds 5 + 7 and writes the answer to LCD0

; We begin by loading the numbers 5 and 7 into general purpose
; registers. As these instructions execute, notice that the
; register values in the "General Purpose Registers" box will
; change.
put 5 gp0
put 7 gp1

; Now we can add those two numbers together with the 'add'
; instruction. The machine cannot add literal numbers together,
; it can only add the contents of registers. This is why we
; cannot say, for example, 'add 5 7'.
add gp0 gp1

; After any mathematical operation has completed, the answer is
; stored in the 'ans' register automatically. We merely need to
; copy this value to the 'out' register in order for the result
; (hopefully 12!) to appear in LCD0.
copy ans out

; We are done, so now we halt the cpu.
halt

; Bonus question: Why did we not have to 'put 0 dvc' in this
; case? How did the cpu know which LCD to use?
