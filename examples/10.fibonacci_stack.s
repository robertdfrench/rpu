; Setup
put 0 gp0
push gp0
put 1 gp1
push gp1

put .MAIN gp7

; Main loop
noop .MAIN
pop gp1
pop gp0

; Write lower value to LCD0
put 0 dvc
copy gp0 out

; Write higher value to LCD1
put 1 dvc
copy gp1 out

; Compute new value
add gp0 gp1

; Put higher value on stack
push gp1

; Put new value on stack
push ans

; Repeat
jump gp7 zero
