# Setup
put 1   gp0
put 0   gp1
put 0   gp7

# Print inital values
put 0   dvc
copy  gp0 out
put 1   dvc
copy  gp1 out

# Main loop
add gp0 gp1 .LOOP

# Write previous to LCD 0
copy  gp1 gp0
put 0   dvc
copy  gp0 out

# Write current to LCD 1
copy  ans gp1
put 1   dvc
copy  gp1 out

# Jump to LOOP
add gp7 gp7
put .LOOP   gp6
jump gp6 ans
