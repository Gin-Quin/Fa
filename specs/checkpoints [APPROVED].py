
# Fa checkpoints have testing and debugging utilities
# They are ignored by default, you need to activate them
# A checkpoint is a line which starts with '>'

# When you compile Fa code, you have the option to :
# 	- print checkpoints
#	- pause at a checkpoint
#	- pause and open a repl
#	- add the checkpoint to the checkpoints list object

# It is 'clean' debugging console log (it can be deactivated any time)

let x = 321
let y = 12

> Lets add x and y
let z = x + y
> Done

# Special commands?
# What about we can add special commands at the end of checkpoints:
> Lets add x and y [print]
let z = x + y
> Done [pause]

# I'm not 100% fan, but I can't explain why. To be thinked.
