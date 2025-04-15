put 0   gp0
put 1   gp1
put 0   gp7
put 20  gp6
cp  gp0 out
cp  gp1 out
add gp0 gp1
cp  ans out
cp  gp1 gp0
cp  ans gp1
add gp7 gp7
jmp gp6
