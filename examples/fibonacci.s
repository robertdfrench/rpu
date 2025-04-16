# Setup
put 0   gp0
put 1   gp1
put 0   gp7

# Print inital values
cp  gp0 out
cp  gp1 out

# Main loop
add gp0 gp1 :LOOP
cp  ans out
cp  gp1 gp0
cp  ans gp1
add gp7 gp7
put :LOOP   gp6
jmp gp6 ans
