put  5     gp0 .MAIN
put  1     gp1
put  .LOOP gp2
put  .END  gp3
put  0     gp4
put  0     dvc
copy gp0   out

sub  gp0 gp1   .LOOP
copy ans out
copy ans gp0
jump gp3 gp0
jump gp2 gp4

noop .END
halt
