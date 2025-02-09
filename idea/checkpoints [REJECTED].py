# The checkpoints idea look too much like comments
# Instead of implementing checkpoints, I prefer to implement *active comments*
# Active comments are comments that have a role when debugging
# They also can have interest for writing test files by defining a stage
# They are stripped in production mode

# Active comments are:

#[stage] *name of the stage* -- print the name of the stage
#[exit] -- exit the process
#[pause] -- pause the process and open a REPL



# -- (PREVIOUS SPECS)
# Fa checkpoints have testing, debugging and logging utilities
# They act like comments but they are not the same
# In production mode, they are cleanly removed
# A checkpoint is a line which starts with '>'

# Checkpoints is an awesome feature to have for testing

# When you compile Fa code, you have the option to :
# - print checkpoints
# - pause at a checkpoint
# - pause and open a repl
# - break the execution

let x = 321
let y = 12

> Lets add x and y
let z = x + y
> Done

# We can add special tags at the end of checkpoints:
> Lets add x and y [log]
let z = x + y
> Done [pause]
# Tags can be used to customize the behaviours of tags, or sort them
# Some useful tags could be : log, warn, info
# But a user can add the tags he wants

# We can also print values with the same syntax as string templates :
> Key is {key} [log]
