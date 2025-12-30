# Functions

## Defining Functions

In Fa, you define a function with the fat arrow `=>` operator.

The syntax is very similar to TypeScript's arrow functions, except that you use the `function` keyword:

```
// in TypeScript
const myFunction = (): ReturnType => {}

// in Fa
function myFunction = (): ReturnType => {}
```

When you declare a function with the `function` keyword:

- the function value is immutable,
- the function is hoisted, which means you can use it before it's declared.

It's also possible to define a function using a `let` or a `mutable` keyword:

```fa
let myImmutableFunction = (): ReturnType => {}

mutable myMutableFunction = (): ReturnType => {}
```

In that case, the function is not hoisted and follows the same rules as any value declared with the `let` or `mutable` keyword.

The return type of a function can be opted out if the function's body is a single expression. Otherwise, it's mandatory.

```fa
-- valid: type of a single expression is inferred
function addOne = (x: Number) => x + 1

 -- invalid: missing explicit return type
function addOne = (x: Number) => {
	return x + 1
}

-- valid: explicit return type
function addOne = (x: Number): Number => {
	return x + 1
}
```

### Methods and function pointers

Inside objects, you can define different types of functions:

- Methods
- Function pointers (using arrow syntax)

```fa
let myObject = {
    // Associated functions (no self parameter)
    someAssociatedFunction(): Result => {
        // code here
        return someValue
    }
    
    // Methods (with self parameter)
    someMethod(self): Result => {
        // code here
        return someValue
    }
    
    // Mutable methods (with mutable self parameter)
    someMutation(mutable self): Result => {
        // code here
        return someValue
    }
    
    // Function pointers (using arrow syntax)
    someFunctionPointer = () => expression
    someFunctionPointer = (): Result => {
        // code here
        return someValue
    }

    // Function pointer methods
    someFunctionPointer = (self) => expression
    someFunctionPointer = (self): Result => {
        // code here
        return someValue
    }

    // Function pointer mutations
    someFunctionPointer = (mutable self): Result => {
        // code here
    }
}
```

### Function aliases

You can also define function aliases:

```fa
function myFunctionClone = myFunction
```

It's especially useful when composing functions together:

```fa
function addAndMultiply = add ||> multiply
```

You can also use the `with` keyword to bind parameters:

```fa
function xyz = (x: Number, y: Number, z: Number) => x + y + z

function xz = xyz with (y = 0)
```

### Function factories

A factory is a function that returns another function. You can write it very naturally:

```fa
function makeAdd = (y: Number) => {
	return (x: Number) => x + y
}

// or, in one line:
function makeAdd = (y: Number) => (x: Number) => x + y

let add3 = makeAdd(3)
let add5 = makeAdd(5)
```

When making a new function from a factory, you will most of the time use `let`, unless the parameters of the function factory are known at build time (otherwise it will not be possible to hoist the function).
