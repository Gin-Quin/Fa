I actually thought of this quite a long time ago: https://www.notion.so/Memory-f8fa4ef6997a419582f0f1575041069d

The only thing that I missed is that containers must be on the heap (since they don't have a fixed size). So we still need memory management, but we only apply it to containers.

---

I'm wondering if treating all objects from structures as copy could be a good idea.

Why?

Because we're already often doing this in Typescript:

```ts
const someFunction = ({ x, y, ...otherParameters }) => { /* ... */ }
```

When we destructure the parameters, we actually do a copy.

The idea is:

- all objects instantiated from `type MyObject = { ... }` are only stored in the stack, never in the heap,
- only containers like arrays, vectors, sets, maps and arenas use the heap.

More and more hints make me think this is actually the right direction to have in programming:

1. MultiArrayList from Zig. This is recommended for best performance in video games, yet it's not compatible with the 1 object = 1 heap allocation model.
2. Containers (arrays, vectors, arenas) as allocators, which is the recommended way to handle memory rather than creating a lot of small and sparse memory allocations.

The more I think about it, the more obvious it is! Simple and safe. No pointers. No memory errors (never!). No garbage collector. Unless I find out some defaults later on, this is actually incredible (sound too good to be true in some way).

## The issue of self-referencing objects

This becomes impossible:

```ts
type Node = {
  next: Node
}
```

Because then the size of `Node` is infinite.

Self-referencing makes no sense and is a bad practice. However, you can still:

- use an optional,
- use an union.

```ts
// this works
type Node = {
  next: Node?
}

// this works as well
type Node = union {
  Text {
    value: string
  }
  Parent {
    child: Node
  }
}
```

## Memory management of containers

Then, the memory management problem becomes the issue of containers. We can still have the issues of circular references and so on.

Let's have this example of circular references:

```ts
type MyObject = {
  values: Array(MyObject)
}

values = Array(MyObject) []
myObject = MyObject { values } // <-- We have a "move" here

values.add(myObject) // <-- Now we have a circular reference. values = [ { values: *self reference * }]

// if no reference counting, this will fail:
values.pop() // <-- will wrongly destroy itself

// if reference counting, when values will go out of scope, it will not be destroyed
```

Solutions:

1. ❌ Parent awareness. Bad because it's quite a performance overhead.
2. ❌ Mark and sweep. Bad because then we have a GC.
3. ✅ Move semantics for containers (the best I think, especially if I transpile to Rust, and if we take into consideration that we can use aliases).

Since strings use a rope-like system, we copy them by default.

If a user wants to have a growing string, he must use a `mutable String`, `Utf8Array`, `Utf16Array` or `Utf32Array`.

## Function parameters

Function parameters are always immutable.

## What about references

Sometimes, we still might want references. For example, when we want fast access to a previous node:

```ts
type Node = {
  next: Node?
  previous: *Node?
}
```

For this, we must use relations.

In Rust, a relation is treated as an unsafe weak reference.

## Function pointers

Function pointers have to be bound with the object they are assigned to. This makes this possible:

```ts
type MyType = {
  value: number

  getValue = () => value
}

printValue = (getValue: () => any) => {
  log.info(getValue())
}

myType = MyType {
  value = 121
}

// we separate the function from the type, but this still works
printValue(myType.getValue)
```
