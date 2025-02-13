# Nullables

In Fa, every variable can be optional - it means it can have the value null.

```coffee
# create an optional undefined Dog
let dog = Dog?

# create an optional well-defined Dog
let dog = Dog?()
```

## Nullable primitives

Strings, booleans and numbers can also be optionals.

- An optional integer will be falsy when equal to null or zero.
- An optional string will be falsy when equal to null or empty.
- An optional boolean will be falsy when equal to null or false.


```coffee
sum = Integer?
print sum  # will print 'null'

if no sum {
  sum = 0
}
print sum  # will print '0'

if no sum {
  sum = 12
}
print sum  # will print '12'
```

If you want to check if an optional integer is strictly equal to null, you have to use the equality operator:

```coffee
sum = Integer?
if sum is 0 {
  print "This will not be printed because sum is undefined"
}
if sum is null {
  print "This will be printed"
}
```

Same goes for strings and booleans.
