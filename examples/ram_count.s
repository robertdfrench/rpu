put 99  gp0 .main
put 1   gp1
put 100 gp2
put .decr gp3
put .end gp4
put 0   dvc

jump  gp4 gp0 .decr 
sub   gp0 gp1
write ans gp2
copy  ans gp0
add   gp2 gp1
copy  ans gp2
jump  gp3 dvc

halt .end
