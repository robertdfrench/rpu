# Setup
put 1   gp0
put 0   gp1
put 0   gp7

# Print inital values
put 0   dvc
cp  gp0 out
put 1   dvc
cp  gp1 out

# Main loop
add gp0 gp1 :LOOP

# Write previous to LCD 0
cp  gp1 gp0
put 0   dvc
cp  gp0 out

# Write current to LCD 1
cp  ans gp1
put 1   dvc
cp  gp1 out

# Jump to LOOP
add gp7 gp7
put :LOOP   gp6
jmp gp6 ans
