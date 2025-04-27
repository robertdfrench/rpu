# This program counts down: 5,4,3,2,1 blastoff!

; We write 5 into gp0. This is the start of our countdown.
put  5     gp0

; We will decrement by 1 each time.
put  1     gp1

; Store the address of the loop logic, so that we can keep
; looping through the countdown
put  .LOOP gp2

; Store the address of the ending logic, so that we can exit the
; program once the countdown has completed
put  .END  gp3

; Make sure that gp4 contains the value zero, so that we can use
; it when we want to jump *no matter what*. This isn't strictly
; necessary, since gp4 will be zero anyways, but it makes the
; code clearer when we are explicit.
put  0     gp4

; Select LCD0 as our default output device
put  0     dvc

; Copy the first value (5) to the LCD
copy gp0   out

; Subtract 1 from gp0 each time
sub  gp0 gp1   .LOOP

; Any math operation leaves its answer in the 'ans' register
copy ans out
copy ans gp0

; Go to .END if gp0 has reached 0
jump gp3 gp0

; Go to .LOOP otherwise
jump gp2 gp4

noop .END
halt
