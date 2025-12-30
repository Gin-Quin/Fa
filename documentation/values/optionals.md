# Nullables

In Fa, every value can be optional, meaning it can have the value `null`.

```fa
# create an optional undefined dog
dog: Dog?

# create an optional defined dog
dog = Dog?()
```

## Nullable primitives

Strings, booleans, and numbers can also be optionals.

- An optional integer will be falsy when equal to null or zero.
- An optional string will be falsy when equal to null or empty.
- An optional boolean will be falsy when equal to null or false.


```fa
sum: Integer?
console.log(sum)  // will print 'null'

if no sum {
  sum = 0
}
console.log(sum)  // will print '0'

if no sum {
  sum = 12
}
console.log(sum)  // will print '12'
```

If you want to check if an optional integer is strictly equal to null, you must use the equality operator:

```fa
sum: Integer?
if sum is 0 {
  console.log("This will not be printed because sum is undefined")
}
if sum is null {
  console.log("This will be printed")
}
```

Same goes for strings and booleans.
