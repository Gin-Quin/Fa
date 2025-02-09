# Strings and bytes

For maximum compatibility with Javascript, strings are encoded in UTF-16.

Raw UTF-16 strings are declared with the double quotes `"`.

```ts
utf16String = "Hello, world!"
utf16String: String = "Hello, world!"
```

Bytes are declared with the `Bytes` type. They can be initialized with a string inside single quotes `'` or an array of numbers.

```ts
bytes = 'Hello, world!'
bytes: Bytes = 'Hello, world!'

bytes = [72, 101, 108, 108, 111, 44, 32, 119, 111, 114, 108, 100, 33]
bytes: Bytes = [72, 101, 108, 108, 111, 44, 32, 119, 111, 114, 108, 100, 33]
```
