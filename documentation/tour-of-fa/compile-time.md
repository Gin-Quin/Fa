## Compile-time

A value can be declared as compile-time only by using the `@` prefix.

```plaintext
@myCompileTimeValue = 12

@compileTimeAdd = (a: Number, b: Number) =&gt; a + b

thirteen = @compileTimeAdd(10, 3)
```

Compile-time values can **only** be used at compile-time. Run-time values can be used at compile-time as well if:

1.  they are constants,
2.  in case of functions, they do not rely on external run-time values.

Example of a run-time function that can be used at compile-time:

```typescript
add = (a: Number, b: Number) => a + b

one = 1

@addedValue = add(one, 2) // OK, because `add` is a run-time function and 1 and 2 are constants
```

```plaintext
add = (a: Number, b: Number) =&gt; a + b

one = 1

@addedValue = add(one, 2) // OK, because `add` is a run-time function and 1 and 2 are constants
```

In a run-time context, you can use the `@` prefix to force a function to be computed at compile-time:

```plaintext
add = (a: Number, b: Number) =&gt; a + b

thirteen = @add(10, 3) // this will generate `thirteen = 13`
```

\> Everything that starts with `@` is stripped from the generated code.