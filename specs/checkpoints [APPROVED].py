
# Fa checkpoints have testing, debugging and logging utilities
# They act like comments but they are not the same
# In production mode, they are cleanly removed
# A checkpoint is a line which starts with '>'

# Checkpoints is an awesome feature to have for testing

# When you compile Fa code, you have the option to :
# 	- print checkpoints
#	- pause at a checkpoint
#	- pause and open a repl
#  - break the execution

let x = 321
let y = 12

> Lets add x and y
let z = x + y
> Done

# We can add special commands at the end of checkpoints:
> Lets add x and y [log]
let z = x + y
> Done [pause]

# We can also print values with the same syntax as string templates :
> Key is {key} [log]