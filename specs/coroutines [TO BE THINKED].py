
# Fa will support coroutines when C++20 is ready (Fa will transpile to C++20 only)
# https://en.cppreference.com/w/cpp/language/coroutines
# In the meantime, coroutines will only work when transpiling to javascript

# -- In Fa, an asynchronous funtion is a function that calls another asynchronous function in its body

# A direct implication is : if there are no asynchronous functions in the Fa standard library,
# then it is not possible to create asynchronous functions.

# -- There is no await keyword in Fa. All asynchronous functions are automatically 'awaited'.

# Most of the time, the user wont have to know if its function is synchronous or not.

# In case one wants to start an asynchronous and continue without waiting for it,
# one should use the keyword 'run' - which will return a promise
# (using run does not guarantee the called function will not be terminated immediately)


let content = readFile 'foo.txt'
print content
# or
run readFile 'foo.txt'
	receive content -> print content


# the JS equivalent would be :
let content = await readFile('foo.txt')
console.log(content)
# or
readFile('foo.txt')
.then( content => console.log(content) )


# See also generators, wich are parts of coroutines
