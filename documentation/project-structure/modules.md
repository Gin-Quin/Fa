# Modules

Fa uses a file system-based approach to code organization that differs significantly from traditional module systems. This document explains how code is imported and exported in Fa.

## Core Principles

1. **Single Export Rule**: Each file exports exactly one value (which can be a function, type, namespace, etc.)
2. **Implicit Namespaces**: Folders automatically create namespaces
3. **Implicit Imports**: No explicit import statements are needed for sibling files or root files
4. **Grouping Without Namespacing**: Folders can be parenthesized `(myFolderName)` to group files without creating a namespace

## Importing Code

In Fa, importing code is much simpler than in languages like JavaScript. The system automatically makes code available based on its location in the file structure.

### Example File Structure

```
source/
  ├── myRootFile.fa
  └── myFolder/
      ├── myFile.fa        <-- we are here
      ├── myOtherFile.fa
      └── mySubFolder/
          └── mySubFile.fa
```

#### Comparison with JavaScript

**JavaScript approach:**

```js
// In JavaScript, explicit imports are required
import myOtherFile from "./myOtherFile.fa"
import mySubFile from "./mySubFolder/mySubFile.fa"
import myRootFile from "../myRootFile.fa"

console.log(myOtherFile, mySubFile, myRootFile)
```

**Fa approach:**

```fa
// In Fa, no import statements are needed
console.log(
  myOtherFile,             // Sibling file - automatically available
  mySubFolder.mySubFile,   // Accessed through namespace
  myRootFile               // Root file - automatically available
)
```

### Import Rules

- **Sibling files**: Directly accessible by filename (without extension)
- **Nested files**: Accessed through namespace paths (folder.file)
- **Root files**: Directly accessible from anywhere in the project
- **Parent directory files**: Not directly accessible (must be accessed through root)

> **IDE Support**: Ideally, the IDE highlights imported values with a distinct color to improve readability.

### Import Aliases with the Extract Operator

Fa provides a convenient way to extract specific values from namespaces using the `>>` extract operator. This is particularly useful when you want to use specific parts of a namespace without referencing the full path each time.

#### Basic Extraction

```fa
// Extract Component from the react namespace
use react.Component

// Now Component can be used directly
class MyComponent extends Component {
  // ...
}
```

#### Multiple Extractions

You can extract multiple values at once:

```fa
// Extract multiple items from a namespace
use utils >> { formatDate, validateEmail, parseJSON }

// Use them directly
formatDate(new Date())
validateEmail("user@example.com")
```

#### Nested Extraction

You can extract from nested namespaces:

```fa
// Extract from nested namespaces
use ui.components >> { Button, Card, Modal }

// Use components directly
Button({ label: "Click me" })
```

#### Combining with Regular Imports

The extraction operator can be combined with regular namespace access:

```fa
// Extract some components
use ui >> { Button, Card }

// Still access other components through the namespace
ui.Modal.show("Hello World")
```

## Exporting Code

Every file in Fa exports exactly one entity. The exported entity is automatically named after the file itself.

### Exporting a Value

To export a simple value (number, string, function, etc.):

```fa
export = 42
```

When imported, this value will be available using the filename:

```fa
// If the above code is in "answer.fa"
console.log(answer) // Outputs: 42
```

### Exporting a Type

To export a type definition:

```fa
export type = {
  name: string
  age: number
  isActive: boolean
}
```

This can be used in type annotations:

```fa
// If the above type is in "User.fa"
function createUser(data: User): User {
  // ...
}
```

### Exporting a Namespace

To export multiple related values as a namespace:

```fa
export namespace = {
  defaultValue = 100
  
  increment(x: number) = x + 1
  
  decrement(x: number) = x - 1
  
  namespace constants = {
    #pi = 3.14159
    #maxUsers = 1000
  }
}
```

When imported, all properties are accessible through the filename:

```fa
// If the above namespace is in "Math.fa"
console.log(Math.defaultValue)      // 100
console.log(Math.increment(5))      // 6
console.log(Math.Constants.PI)      // 3.14159
```

## Advanced Features

### Parenthesized Folders

Folders with parentheses `(utils)` group files without creating a namespace:

```
source/
  └── (utils)/
      ├── format.fa
      └── validate.fa
```

Files in parenthesized folders can be accessed directly:

```fa
// No need to use utils.format
console.log(format("Hello"))
console.log(validate("email@example.com"))
```

### Re-exporting

To create an aggregated export, you can re-export values from other files:

```fa
// In "index.fa"
export namespace = {
  // Re-export other files
  User = User           // from User.fa
  createUser = createUser  // from createUser.fa
  
  // Add new exports
  version = "1.0.0"
}
```
