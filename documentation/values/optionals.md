# Optionals

In Fa, every value can be optional, meaning it can have the value `none`.

```fa
-- create an optional undefined dog
let dog: Dog?

-- create an optional defined dog
let dog = Dog?()
```


```fa
mutable sum: Integer? = none

if no sum { -- `none` is falsy
  sum = 0
}
console.log(sum) -- will print '0'

if sum is 0 {
  sum = 12
}
console.log(sum) -- will print '12'
```

If you want to check if an optional integer is strictly equal to none, you must use the equality operator:

```fa
let sum: Integer? = 0

assert(sum is 0) -- yes, sum is 0
assert(sum?) -- yes, sum is not none
assert(no sum) -- yes, sum is falsy
```
