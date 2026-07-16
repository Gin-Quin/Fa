# Conclusion

The question is about should I use chevrons for generics, or regular arguments?

I think I like the chevron syntax better, for this reason:

- it can be ommitted when we want to use the default values, or when we want to infer the types from the parameters

It's just weird to "ignore" function parameters.

# Reflexion

We need generics in the following situations:

1. Generics (parameters: types, return: type)
2. Type functions (parameters: types, return: code that returns a type)
3. Type parsing (parameters: types, return: value)
4. Type generation (parameters: value, return: type)
5. Generics

- Use the same syntax as Typescript (chevrons)

Int<min = 12>
Map<String, Number>

type Map = <Key, Value> => { ... }
type Map = <Key, Value> => Key | Value
type Map<Key, Value> = Key | Value

2. Type functions

type TypeFunction = (ParameterA, ParameterB: String) => { ... }

or

type TypeFunction = <ParameterA, ParameterB: String> => { ... }

type MyClass = <>{
}

3. Type parsing

Regular functions, that accept a type as parameter

typeToString = (Type): String => { ... }

typeToString({ x: Number })

4. Type generation

Type(a)
Type<a>
Int<min = 12>

## Compile-time values vs types

Imagine this:

```
let Toto = 12
```

versus this:

```
type Toto = 12
```

Are these the same?? Is a compile-time value the same as a type?

For numbers, yes. But not for structs.

```
let Animal = {

}
```

```
type Animal = {

}
```

Although we could fix it with a struct keyword:

```
let Animal = struct {

}
```

In this case, we'd differentiate a value from a type.

Also we can have these types:

```
type Numbers = 12 | 14
```

But that's weird as a compile-time value:

```
let Numbers = 12 | 14
```

A **type** is a **value constraint**.

Now what about compile-time values in function parameters?

Option 1: use uppercases

type Int(Min: Number?, Max: Number?) =

Option 2: use @

type Int(@min: Number?, @max: Number?) =
type Int = (@min: Number?, @max: Number?) =>

Option 3: use chevrons

type Int<min: Number?, max: Number?> =
