# Memory management

Fa compiles to JavaScript, to native code and to WebAssembly. The last two are low-level targets with no garbage collector, so memory has to be managed — but in Fa the compiler does it for you, from rules simple enough to hold in your head.

Here is the whole model:

1. **Objects** are allocated on the **stack**, and only on the stack.
2. **Collections** (arrays, sets, maps, bags, columnars) are the only way to use the heap.
3. **Every assignment is a copy.** The compiler turns a copy into a move whenever the source is about to be dropped.
4. **`use` creates a reference** instead of copying. A reference cannot outlive what it points into.
5. **Function parameters are references**, so arguments are never copied when passed.
6. **Iterating a collection yields references**, so iteration never copies.
7. **Relations** are compiler-maintained weak references between fields, declared as part of the type.

You never ask *"when should I free this?"* or *"which allocator should I use?"*. The question in Fa is *"where should this object live?"*.

## Objects live on the stack

Objects are never heap-allocated. This has three consequences worth understanding:

- Heap allocation happens at collection level, not object level. Allocations are few and large rather than many and scattered.
- Objects stay small relative to collections, so copying them is cheap.
- Dangling pointers and reference cycles are impossible on the stack, so objects are safe by construction.

```fa
mutable object = {
  x = 12
  hello = "world"
  nested = {
    y = 21
  }
}

-- unlike TypeScript, this is a deep copy, not a shared pointer
mutable object2 = object

-- unlike Rust, there is no ownership transfer: the original is still usable
object.hello = "you"

console.log(object2.hello) -- "world"  (TypeScript would print "you")
```

### Isn't copying expensive?

Less than it looks, for two reasons.

Fa still uses references wherever it is safe to: function parameters, iteration, aliases and relations. You copy only where a copy is what you actually meant.

And because Fa is [structurally typed](./structural-typing), you usually take only the fields you need rather than the whole object:

```fa
function parametersAreReferences = ({ foo, bar }: { foo: String, bar: String }) => {
  -- `foo` and `bar` are references into the caller's object; nothing is copied
}
```

The same code in TypeScript, destructuring parameters, does copy every destructured field. The default copy behaviour costs close to nothing, and buys simplicity and safety.

### Recursive objects

An object cannot contain itself, because its size would be unbounded:

```fa
type Node = {
  next: Node -- error: `Node` has infinite size
}
```

There are two ways to build recursive structures:

- a [union](../types/unions), where only one member exists at a time,
- a collection plus references into it, which is what [relations](./relations) are for.

```fa
type Node = Union {
  Text = { value: String }
  Parent = { child: Node }
}
```

```fa
mutable nodes = Bag(Node) {}

type Node = {
  next: Node in nodes?
}
```

## Collections own the heap

Collections are the only heap allocation in Fa. Instead of `alloc` and `realloc`, you add elements to a collection, and each collection uses the allocation strategy that suits it.

A collection is destroyed when its scope ends, releasing everything it holds in one step.

### Iterating yields references

```fa
mutable array = [1, 2, 3]

for array >> element, index {
  element += 1 -- writes into the array
}

console.log(array) -- [2, 3, 4]
```

### Aliases

Reading an element out of a collection copies it. To work on the stored value itself, take a reference with `use`:

```fa
mutable people: Array(Human) = [
  Human { name = "John", age = 20 }
  Human { name = "Jane", age = 21 }
  Human { name = "Joe", age = 22 }
]

-- a copy: the array is untouched
let myFavouriteHuman = people[1]
myFavouriteHuman.age = 23
console.log(people[1].age) -- 21

-- a reference: the same value as `people[1]`
use myFavouriteHuman = people[1]
myFavouriteHuman.age = 23
console.log(people[1].age) -- 23
```

A reference into a collection is called an [owned reference](./owned-references), and the compiler tracks which collection owns it.

### Removing while iterating

Mutating a collection's structure while iterating it would invalidate the references iteration hands out, so the compiler rejects it:

```fa
mutable bag = Bag(Human) {}

for bag >> human {
  if human.name is "John" {
    bag.delete(human) -- error: `bag` is being iterated
  }
}
```

Use `retain` instead:

```fa
bag.retain((human: Human) => human.name is not "John")
```

## Lifetimes and destruction

Every value is destroyed at the end of the scope that declared it, in reverse order of declaration.

```fa
function example = () => {
  mutable a = Array(Int) {}
  mutable b = Array(Int) {}
} -- `b` is released, then `a`
```

Because references cannot outlive what they point into, and because nothing is shared implicitly, destruction never has to be deferred, counted or traced. There is no garbage collector, no reference counting, and no `free`.

Returning a reference out of a function is allowed only when it points into something the caller already owns. Otherwise, the value is copied out. See [owned references](./owned-references#returning-a-reference).

## Closures

A closure captures the values it uses **by copy**, exactly like an assignment. A closure can therefore be stored, returned, and sent to another thread without any lifetime question arising.

```fa
function makeAccumulator = () => {
  mutable total = 0
  return (value: Int): Int => {
    total += value
    return total
  }
}
```

Capturing a reference is allowed only when the compiler can prove the closure does not outlive it. See [closures](../functions/defining-functions#closures).

## References

Instead of "wild" references — a number pointing at an arbitrary memory location, as in C, C++ or Go — Fa has two kinds of safe reference.

The danger of a wild reference is that it can dangle. Languages solve this in different ways:

- JavaScript and Go use a garbage collector, at the cost of memory and throughput.
- Swift uses reference counting and weak references, which is still unsafe when misused.
- Rust uses ownership and borrowing, which is sound but has a steep learning curve.
- Zig, C and C++ leave it entirely to the programmer.

Fa takes a different route:

- an [**owned reference**](./owned-references) points into a collection, and the type system knows which one, so it can tell when the reference is no longer valid;
- a [**relation**](./relations) is a weak reference declared as part of a type, which the compiler keeps consistent on both sides.

Neither can dangle, and neither requires you to write a lifetime annotation.

## Summary

| You write | What happens |
| --- | --- |
| `let b = a` | copy (or move, if `a` is dead afterwards) |
| `use b = a` | reference, no copy |
| `f(a)` | reference, no copy |
| `for c >> item` | reference, no copy |
| `c.add(x)` | `x` is moved into the collection's heap storage |
| `c.take(i)` | the item is moved out of the collection |
| end of scope | every value declared in it is released |
