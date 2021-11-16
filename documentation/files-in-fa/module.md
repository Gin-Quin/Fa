# Module

Modules, along scripts and data objects, are one the three file types in Fa. All files that have an export statement are modules.

Possible exports are:

- a value (any value),
- a type,
- a class,
- an enum,
- a namespace.

Modules that export a value must have their name beginning with a lowercase letter.

Modules that export a type, a class, an enum or a namespace must have their name beginning with an uppercase letter.

## Examples

### Export a value
```coffee
export 12
```

### Export a variable
```coffee
let x = 12
export x
```

### Export a function
```coffeefa
export (x: ) ->

```

### Export an object
```coffee
# myFile.fa
export
   pi: Math.pi
   user: "Johnny"
   url: "https://twitter.api/getMessages?user={Johnny}"
```

### Export a type
```coffee
# Dog.fa
export type
   const type = "Dog"
   name: String
   createdAt: Date
   speed = 42
```