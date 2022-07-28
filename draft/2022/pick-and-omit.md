Picking propertyies in an object is done in many languages, for exemple:

```ts
const { a, b } = object
```

Fa allows to pick-clone, ie doing something like this:

```coffee
const clone = { a, b } = object
# or: (to decide)
const clone << { a, b } << object
```

But what if I want to clone the _whole object_ except some keys?

In Typescript, this is just impossible. The best thing is to

1. clone the whole object
2. delete the unwanted keys

Like this:

```ts
const clone = { ...object }
delete clone.a, clone.b
```

This is very bad. It's not compatible with a true typed language and it's not optimized.

Fa needs a way to pick **every property but...*.

Which syntax to use?

```coffee
# Syntax 1: { not ...keys... }
const clone = { not a, b } = object
const clone << { not a, b } << object

# Syntax 2: { omit ...keys... }
const clone = { omit a, b } = object
const clone << { omit a, b } << object
const clone = 
  ...object omit { a, b }
const clone =
  ...object omit:
    a
    b

# Syntax 3: constant regular expression (too complex I think)
const clone = {/ a|b /} = object
```

Syntax #2 seems the best to me as "not" is not as explicit as "omit".
