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

- use an owned reference,
- use a weak reference.

```ts
let nodes = Bag(Node)

// this works
type Node = {
  next: Node in nodes?
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
