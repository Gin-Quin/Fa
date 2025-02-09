
# Not sure if adding exit is really helpful.. Need to rethingk about that
# No exit. Using return is enough

# In fa, you can use the two keywords return and exit
# Both make you quit immediately a function, but in a slightly different manner :
# - `return` needs a argument to be specified
# - `exit` quit the function with the current values of the return values

# exemples : an addition function with the `exit` and `return` ways :

add x: Integer, y: Integer -> x + y

add x: Integer, y: Integer -> Integer
	return x + y

add x: Integer, y: Integer -> z: Integer
	z = x + y

# In this case, the `return` function is more efficient, but it may not always be

# We could have use an 'empty return' to simulate 'exit', but I think it adds clarity about
# what is being done. When you see an exit you think : "Ok, I should see what the returned values are"
