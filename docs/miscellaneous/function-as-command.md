# The command syntax

All of you have already noticed that the `print` function is called without parenthesis :

```coffee
print "Hey you"
```

There is a special rule in Fa about this syntax :

> All functions at the start of a new line can be called without parenthesis.

This is called the **command syntax**.

So what does that mean?

Let's say we define the following function that adds two numbers and print then return the result :

```coffee
let add(x: Integer, y = 0) -> sum = x + y
   print sum
```

Then we can use it this way :

```coffee
add 5, 17  # will print '22'
```

But we cannot use it this way :

```coffee
print add 5, 17
add 5, add 16, 1
```

First because the syntax becomes hard to read and understand, and second because this would raise an ambiguity : what would mean `print add 5, 17`?
- `print(add(5), 17)` or
- `print(add(5, 17))`?

The right way do do this is :

```coffee
print add(5, 17)  # will print twice '22'
```
