# Metaprogramming

## What is Metaprogramming?

Metaprogramming is a programming technique where code can manipulate other code during compilation or at runtime. In essence, it's "programming that programs" - code that treats other code as data. This powerful paradigm allows developers to write programs that generate, analyze, or transform other programs (or themselves).

In Fa, metaprogramming is primarily focused on compile-time evaluation, where certain expressions and functions are executed during compilation rather than at runtime.

## Why Metaprogramming Matters

Metaprogramming offers several significant advantages:

1. **Avoiding Code Generation**: Instead of using external tools to generate code, metaprogramming allows you to generate code within your program itself, making the process more integrated and maintainable.

2. **Type Validation and Safety**: Compile-time evaluation can catch type errors and other issues before runtime, enhancing program safety.

3. **Performance Optimization**: By moving computations from runtime to compile-time, you can eliminate runtime overhead for operations that can be predetermined.

4. **Reducing Boilerplate**: Metaprogramming can automate repetitive coding patterns, leading to cleaner, more concise code.

5. **Enhanced Expressiveness**: It enables more powerful abstractions and domain-specific language features within your code.

## Fa's Metaprogramming Approach

Fa implements metaprogramming using the `@` prefix operator. When you prefix a function call or value with `@`, you're instructing the compiler to evaluate it at compile-time rather than runtime.



## Static literals

A **static value** is a deeply immutable value that is available at compile-time.

### Static primitives

By default, all primitives are **static**.

```fa
let foo = 12 -- `foo` is a `static integer`
let bar = "hello" -- `bar` is a `static string`
```

### Static arrays

To make an array static, you can use the `@` prefix operator on the array literal:

```fa
let staticArray = @[1, 2, 3]
```

You cannot make a static array from a non-static value, even if the value is immutable:

```fa
function useStaticArray(a: Number): number[] {
	let staticArray = @[a, 12] -- error: `a` is immutable but not static
}
```

### Static objects

To enforce an object to be static, you can use the `@` prefix operator on the object literal:

```fa
let staticObject = @{ foo: "foo", bar: 12 }
```


## Static values

A **static value** is a value that only exists at compile-time. It is not available at runtime. You define it using the `@` prefix operator:

This code:
```
let @foo = 12
console.log(@foo)
```

Will produce the following code:
```
console.log(12)
```

All occurrences of a static value will be replaced at compile-time.

## Basic Syntax

```fa
-- Runtime call (normal)
let z = add(2, 4) -- Will be left as-is during transpilation

-- Compile-time call
let z = @add(2, 4) -- Will be transformed into "let z = 6"

-- Define a compile-time value
let @z = @add(2, 4) -- Will be completely stripped at runtime, `@z` occurrences will be replaced with `6`
```

## Key Features

1. **Compile-time Function Execution**: Pure functions with known inputs can be executed during compilation.

```fa
-- Define a function
function add = (x: Number, y: Number) => x + y

-- Call it at compile-time
let result = @add(2, 4)  -- Becomes "let result = 6"
```

2. **Type Reflection**: Examine types at compile-time.

```fa
let x = 12
console.log(@typeof(x))  -- Will print { type: "NumberLiteral", value: 12 }
```

3. **File System Integration**: Access the file system during compilation.

```fa
-- Read version from a file at compile-time
function @readVersion = () => @readFile("../version.txt")
let version = @readVersion() -- Transpiled to `version = "1.0.0"`

-- Access file information
let currentFile = @fileName
let directory = @directoryName
```

4. **Parsing**: Parse code at compile-time.

```fa
let json = @parseJson("{ \"foo\": \"bar\" }")

console.log(json.foo) -- type is parsed as well
```

5. **File System Routing**: Create routing structures based on the file system.

```fa
-- Read directory structure for routing
let routes = @readDirectory("../routes")
```

6. **Compile-time Collections**: You can have compile-time collections that are completely immutable.

```fa
let colors = @["red", "green", "blue"]

let @configuration = {
  port = 8080
  debug = false
  stage = "development"
}
```

7. **Compile-time Export**

If a filename starts with an `@`, then its exported value is available at compile-time.

```fa
-- file name: `@configuration.fa`
export = {
  port = 8080
  debug = false
  stage = "development"
}
```

## Implementation Details

To make metaprogramming possible, Fa includes an interpreter that can execute Fa code at compile-time. This interpreter supports a subset of the Fa standard library, including I/O operations, allowing for powerful compile-time capabilities.

Metaprogramming in Fa is designed to be intuitive and integrated with the language, rather than feeling like a separate feature or extension.

## Practical Applications

- **Configuration Management**: Read configuration from files at compile-time
- **API Client Generation**: Generate type-safe API clients from schema definitions
- **UI Component Libraries**: Create optimized component variants at compile-time
- **Database Queries**: Validate SQL queries during compilation
- **Custom DSLs**: Implement domain-specific languages within Fa

By leveraging metaprogramming, Fa enables developers to write more expressive, efficient, and safer code while reducing the need for external code generation tools and runtime overhead.
