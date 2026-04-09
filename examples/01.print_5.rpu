# This example writes the number 5 to LCD0

; We start by putting 5 into the register gp0. This is the data
; that we will write into the LCD
put  5   gp0

; Now we need to specify which device we'll be using. The first
; LCD panel is "Device 0", so we just put 0 into the dvc
; register.
put  0   dvc

; At this point, all we need to do is copy the contents of the
; gp0 register into the 'out' register. When you copy data to
; the 'out' register, it will be sent to the device specified in
; the 'dvc' register. In this case, that means LCD0.
copy gp0 out

; We are done, so now we halt the cpu.
halt
