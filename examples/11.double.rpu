; This program defines a function called DOUBLE, which is called
; with an argument X and returns 2*X. The DOUBLE function is
; called on each of 10, 9, 8, ... and the results are stored
; in the HEAP (defined at the end of the program).

; Start
    put 11 gp0
    put .HEAP gp7


; LOAD 
    put   1   gp1 .LOAD
    sub   gp0 gp1
    push  ans ; to retain the counter
    push  ans ; as an argument for DOUBLE
    put   .LOAD__DOUBLE_RETURN gp2
    push  gp2
    put   .DOUBLE gp2
    jump  gp2 zero


; LOAD__DOUBLE_RETURN
    pop   gp3 .LOAD__DOUBLE_RETURN
    pop   gp0 ; Recover the counter
    write gp3 gp7
    put   2   gp1
    add   gp1 gp7
    copy  ans gp7

    put .END gp2
    jump gp2 gp0
    put .LOAD gp2
    jump gp2 zero


; DOUBLE
    pop  gp2 .DOUBLE
    pop  gp0 ; Argument
    put  2 gp1
    mul  gp0 gp1
    push ans
    jump gp2 zero ; Return to caller


; END
    halt .END


; Define HEAP Address
    noop .HEAP
