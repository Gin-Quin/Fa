# Functions

## Defining Functions

Fa provides multiple ways to define functions, with two main syntax styles: regular functions and arrow functions.

- Regular functions are defined with the `function` keyword.
- Arrow functions are defined with the fat arrow `=>` operator.

Function declarations are very close to the Typescript syntax, with the only difference that the syntax `let myFunction = function() { ... }` is not allowed. The `function` keyword must be used at the top level of a scope.

The return type of a function is optional if the function's body is a single expression.

### Top-level functions

The `function` keyword can be used to define a function:

```fa
// Block body with explicit return type
function myFunction(a: A, b: B): C {
    // code here
    return someValue
}

// Expression body with implicit return type
function myFunction(a: A, b: B) => expression
```

Functions defined with the `function` keyword:

- cannot be used inside expressions
- are hoisted (can be used before their declaration)
- are pure

:::tip
A **pure function** is a function that:
- Produces the same output for the same input
- Has no side effects
- Does not modify global state
:::

Note: it's still possible to manipulate data outside of the function scope using Fa's **contexts**.

### Arrow functions (closures)

Functions can also be defined using arrow syntax with variable assignment:

```fa
// Block body with explicit return type
let myFunction = (a: A, b: B): C => {
    // code here
    return someValue
}

// Expression body with implicit return type
let myFunction = (a: A, b: B) => expression
```

Closures:

- are not pure, as they can capture variables from their surrounding environment and have side effects
- must be assigned to a variable before they can be used

### Functions in Modules

Inside a module, you can define regular functionswithout the `function` keyword:

```fa
let myModule = module {
    // Block body with explicit return type
    myFunction(): Result {
        // code here
        return someValue
    }
    
    // Expression body with implicit return type
    myFunction() => expression
}
```

### Functions in Objects

Inside objects, you can define different types of functions:

- Associated functions or methods
- Function pointers (using arrow syntax)

```fa
let myObject = {
    // Associated functions (no self parameter)
    someAssociatedFunction(): Result {
        // code here
        return someValue
    }
    someAssociatedFunction() => expression
    
    // Methods (with self parameter)
    someMethod(self): Result {
        // code here
        return someValue
    }
    someMethod(self) => expression
    
    // Mutable methods (with mutable self parameter)
    someMutation(mutable self): Result {
        // code here
        return someValue
    }
    someMutation(mutable self) => expression
    
    // Function pointers using arrow syntax
    someFunctionPointer = () => expression
    someFunctionPointer = (): Result => {
        // code here
        return someValue
    }

    // Function pointers methods
    someFunctionPointer = (self) => expression
    someFunctionPointer = (self): Result => {
        // code here
        return someValue
    }

    // Function pointers mutations
    someFunctionPointer = (mutable self): Result => {
        // code here
    }
}
```
