# Errors 

:::warning
This is work in progress
:::

In Fa, there are no `try / throw / catch` expressions. Instead, functions must return an explicit type using a union.

Throws are error-prone because:

- the thrown value cannot be safely typed and retrieved with its type,
- it's possible to forget to catch a throw.

Using a union type, error-checking safety is enforced at compilation time.

Example use case:

```fa
-- let's define a function that returns a number or an error
function loadScore = (): Number | Error => {
  score = await api.loadScore()
  if score is Error >> { message } {
    return Error("Could not load score: {message}.")
  }
  return score
}

-- then let's use it this way:
function printScore = () => {
  score = loadScore()
  if score is Error {
  	return score
  }
  console.log("Your score is: {score.value}")
}
```

## The '!' operator

Always having to check if an error is returned can be cumbersome, especially if it is to return the error to a caller function that will itself return the error to its caller function, etc.

That's why logic has been added to the exclamation mark operator `!`.

It must be used on a **value that has a union type**. If the value has the first type in the union, this value will be narrowed. Otherwise, the function will return the value itself.

Example:

```ts
printScore = () => {
  score = loadScore()!
  console.log("Your score is: {score}")
}
```

This is the exact equivalent of the following function:

```ts
function printScore = () => {
  let score = loadScore()
  if score.error {
    return /* score.error */ // <- Since the function returns "Void", the exclamation mark operator will return void and not the error
  }
  log("Your score is: {score.value}")
}
```

In other words, the `!` operator unwraps the value if it is successful, otherwise it returns the value itself, narrowed as the union without its first type.

### Unions with more than two values

It's possible to use the `!` operator on a union with more than two values. It can be useful when you have many possible errors.

```ts
type Error(Context = Never) = {
  error = Context
}

function someFunction = (): String | Error(Integer) | Error(String) => {}

let someString: String = someFunction()! // if it fails, it will return a value of type `Error(Integer) | Error(String)`
```

## The `?` operator

Like the `!` operator, the `?` operator must be used on a union type.

It tries to unwrap the value as the first type of the union. If it fails, it unwraps as the other type(s) of the union and stops the current operation.

The `?` operator can be used in several places:

1. When accessing the field of an object: `object?.field`.
2. When calling a function: `myFunction?()`.
3. When accessing an element of a collection that may be in error: `array?[index]`.

> Using the `?` operator without chaining with an operation is a no-op and will trigger a warning.
