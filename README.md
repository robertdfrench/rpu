# Your Buddy Robert's Processing Unit
Are you having trouble with:

* Central Processing Units?
* Graphics Processing Units?
* *Tensor Processing Units??*

Who needs 'em?! Use your buddy Robert's processing unit instead.

![Your buddy Robert](profile.jpg)

## Easy
Those other processing units are hard. They have like, features,
and performance guarantees, all kinds of stuff. You don't need
all that. Your buddy Robert's processing unit is *easy peasy*.
It only has a few instructions, and none of them does anything
crazy:

* `add`: Adds two numbers together. *Easy!*
* `copy`: Copies stuff from one register to another. **Easy!**
* `halt`: Turns the computer off. No problem!
* `put`: Puts a number into a register. Easy!
* `jump`: Skip ahead a few instructions. Or back. Simple!
* `mul`: Multiple two numbers. Third grade stuff!
* `noop`: Literally DO NOTHING. Like falling off a log.
* `put`: Write a value to a register. Couldn't be simpler.
* `read`: Pull some data from memory
* `sub`: Subtract two numbers.
* `write`: Put some data *into* memory

## Get Started
1. Run `sudo make install` to install `rpu` and its man page.
2. Run `man rpu` to read the man page

*When you start `rpu`, you will need to press 'n' to get it to execute
the next instruction.  You can quit by pressing 'Esc'*

3. Run `rpu examples/print_5.s` to see how to write the number "5" to
   LCD0.
4. Run `rpu examples/add_5_6.s` to see how to add 5 + 7 and write "12"
   to LCD0.
5. Run `rpu examples/countdown.s` to see how to make LCD0 display the
   numbers 5,4,3,2,1 in order.
6. Run `rpu examples/fibonacci.s` to see how to compute the fibonacci
   sequence, showing the latest number on LCD0, and the previous number
   on LCD1.
