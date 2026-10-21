# Optionals

In Fa, every value can be optional, meaning it can have the value `none`.

```fa
-- create an optional undefined dog
let dog: Dog?

-- create an optional defined dog
let dog = Dog?()
```


`none` is not `false`: an optional value cannot be used directly as a condition. Compare it with `none` explicitly using `is none` or `is not none`.

```fa
mutable sum: Integer? = none

if sum is none {
  sum = 0
}
console.log(sum) -- will print '0'

if sum is 0 {
  sum = 12
}
console.log(sum) -- will print '12'
```

After a check, the type of the value is narrowed:

```fa
let dog: Dog? = findDog()

if dog is not none {
  dog.bark() -- `dog` is a `Dog` here
}

if dog { -- error: expected a `Boolean`, found `Dog?`
}
```
