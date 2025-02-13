# Flags

Flags are like enums, but they can be combined with the `+` and `-` operators.

```ts
type MyFlag = Flag { foo, bar, baz }

myFlag = MyFlag.foo + MyFlag.bar
```

Under the hood, flags are stored as integers with size depending on the number of flags.

Number of bits of a flag structure = number of flags.

For example, a flag with 12 flags will be stored as a 12-bit integer, i.e. two bytes.

