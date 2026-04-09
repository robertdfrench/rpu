; We start with 99 bottles of beer
put 99  gp0

; We take 1 bottle down each time
put 1   gp1

; This is the address at which we start writing numbers. It has
; no particular significance, other than being large enough that
; we won't accidentally overwrite the program code (which is
; also in memory, don't forget!)
put 100 gp2

; Store the address of the logic that decrements the number of
; bottles of beer.
put .decr gp3

; Store the address of the end of the program, for when we are
; out of beer.
put .end gp4

; At the start of each loop, see if we are out of beer. If we
; are out, jump to gp4 (the end of the program).
jump  gp4 gp0 .decr

; Take one down
sub   gp0 gp1

; Write the current number of bottles of beer into the address
; contained in gp2
write ans gp2

; Copy the current number of bottles of beer into gp0 so we can
; keep track of it
copy  ans gp0

; Increment the memory address by 1
add   gp2 gp1
copy  ans gp2

; Always loop
jump  gp3 gp7

halt .end
