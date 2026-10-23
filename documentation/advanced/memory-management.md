# Memory management

Fa compiles to JavaScript, to native code and to WebAssembly. The last two are low-level targets with no garbage collector, so memory has to be managed. To allow maximum performance and lean WebAssembly output, Fa does not come with a garbage collector either: in Fa, the compiler manages memory for you, from rules simple enough to hold in your head.

Here is the whole model:

1. **Objects** are allocated on the **stack**.
2. **Collections** (arrays, sets, maps, bags, columnars) own the **heap**. The only other heap allocation is made by the compiler for [recursive types](#recursive-types).
3. **Every assignment is a copy.** The compiler turns a copy into a move whenever the source is about to be destroyed.
4. **`use` creates an alias** instead of copying. An alias cannot outlive the value it refers to.
5. **Function parameters are immutable references**, so arguments are never copied when passed. A function that modifies a parameter declares it with the `mutates` effect.
6. **Iterating over a collection yields references**, so iteration never copies.
7. **Every value is destroyed at the end of its scope.**
8. **A field holds a copy**, unless it is a **relation**: a reference that names the collection its target lives in (`bestFriend: (Person in people)?`), kept consistent by the compiler.

You never ask *"when should I free this?"* or *"which allocator should I use?"*. The question in Fa is *"where should this object live?"*. And the answer is part of the type: a `Person` stored in the collection `persons` has the type `Person in persons` (see [owned references](/advanced/owned-references)).

## Objects live on the stack

Objects are never allocated on the heap. This has three consequences worth understanding:

- Heap allocation happens at the collection level, not at the object level. Allocations are few and large, rather than many and scattered, and they are released at once.
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
let object2 = object

-- unlike Rust, there is no ownership transfer: the original is still usable
object.hello = "you"

console.log(object2.hello) -- "world" (TypeScript would print "you")
```

### Isn't copying expensive?

Less than it looks, for two reasons.

Fa still uses references wherever it is safe to: function parameters, iteration, aliases and relations. You only copy when a copy is what you actually meant.

And because Fa is [structurally typed](/advanced/structural-typing), a function usually takes only the fields it needs rather than the whole object:

```fa
function parametersAreReferences = ({ foo: String, bar: String }) => {
	-- `foo` and `bar` are references into the caller's object: nothing is copied
}
```

The same code in TypeScript, destructuring the parameter, copies every destructured field. The default copy behavior costs close to nothing, and buys simplicity and memory safety.

### Recursive types

An object cannot contain itself directly, because its size would be infinite:

```fa
type Node = {
	value: String
	next: Node -- error: `Node` has infinite size
}
```

Recursion through a [union](/types/unions), including an [optional](/values/optionals) (`T?` is the union `T | None`), is allowed. The compiler stores the recursive part on the heap automatically, but it is still a copy: each value owns its children, which are copied with it and destroyed with it.

```fa
type Node = {
	value: String
	next: Node? -- valid: recursion through an optional
}

type Tree = Union {
	Leaf = { value: Integer }
	Branch = { left: Tree, right: Tree }
}
```

There is no `Box` type, and nothing to write: the heap allocation is an implementation detail.

Because each value owns its children, these structures are trees: two nodes can't share the same child. For structures where objects need to point to each other (graphs, doubly linked lists, parents and children), store the objects in a `Bag` and link them with [relations](/advanced/relations): `next: (Node in nodes)?`.

## Collections own the heap

Collections are the only explicit way to use the heap in Fa. Instead of `alloc` and `realloc`, you add elements to a collection, and each collection uses the allocation strategy that suits it.

A collection is destroyed when its scope ends, releasing everything it holds in one step.

### Iterating yields references

Iterating over a collection yields references to its elements, so nothing is copied. When the collection is mutable, the loop variable is a mutable reference: writing to it writes into the collection.

```fa
mutable array = [1, 2, 3]

for element in array {
	element += 1 -- writes into the array
}

console.log(array) -- [2, 3, 4]
```

### Aliases

Reading an element out of a collection copies it. To work on the stored value itself, create an alias with `use`:

```fa
mutable people: Array(Human) = [
	{ name = "John", age = 20 }
	{ name = "Jane", age = 21 }
	{ name = "Joe", age = 22 }
]

-- a copy: the array is untouched
mutable copiedHuman = people[1]
copiedHuman.age = 23
console.log(people[1].age) -- 21

-- an alias: the same value as `people[1]`
use favoriteHuman = people[1]
favoriteHuman.age = 23 -- exactly the same as writing `people[1].age = 23`
console.log(people[1].age) -- 23
```

An alias to an element of a collection is called an [owned reference](/advanced/owned-references): its type tells which collection owns it (`Human in people`), and the compiler tracks when it is invalidated. After that, it must be checked with `is not none` or, for an array, declared again before it is used.

Collections that return a reference when an element is added, like [`Bag`](/collections/bags), are used with `use` too:

```fa
mutable bag: Bag(Human) = []

use john = bag.add({ name = "John", age = 20 }) -- a reference to the stored human
john.age += 1 -- modifies the human stored in the bag
```

### Removing while iterating

Adding or removing elements of a collection while iterating over it would invalidate the references the iteration hands out, so the compiler rejects it:

```fa
mutable bag: Bag(Human) = []

for human in bag {
	if human.name == "John" {
		bag.delete(human) -- error: `bag` is being iterated
	}
}
```

Use `deleteWhere` instead:

```fa
bag.deleteWhere((human) => human.name == "John")
```

## Parameters are references

Parameters are immutable references: passing a value to a function never copies it. A function that modifies a parameter declares it with the `mutates` effect, and can only be called with a `mutable` value:

```fa
function birthday = (human: Human) mutates human => {
	human.age += 1
}

mutable john = Human { name = "John", age = 20 }
birthday(john) -- `john` is modified in place, without any copy
```

A function that can move or remove the elements of a collection declares the `invalidates` effect, so that the compiler knows which references are no longer valid after the call. See [defining functions](/functions/defining-functions#mutating-parameters).

## Lifetimes and destruction

Every value is destroyed at the end of the scope that declared it, in reverse order of declaration.

```fa
function example = () => {
	mutable a: Array(Integer) = []
	mutable b: Array(Integer) = []
} -- `b` is released, then `a`
```

Because references cannot outlive what they point into, and because nothing is shared implicitly, destruction never has to be deferred, counted or traced. There is no garbage collector, no reference counting, and no `free`.

Returning a reference out of a function is allowed only when it points into something the caller already owns. Otherwise, the value is copied out. See [owned references](/advanced/owned-references#returning-a-reference).

## Closures

A closure captures the values it uses **by copy**, exactly like an assignment. A closure can therefore be stored, returned, and sent to another thread without any lifetime question arising.

```fa
type Greeter = (name: String) => String

function makeGreeter = (greeting: String): Greeter => (name) => "{greeting}, {name}!"

let greetings = [makeGreeter("Hello"), makeGreeter("Hi")]
```

Capturing a reference created with `use` is allowed only when the compiler can prove that the closure does not escape. See [closures](/functions/defining-functions#closures).

## References

Instead of "wild" references (a number pointing at an arbitrary memory location, as in C, C++ or Go), Fa has two kinds of safe references.

The danger of a wild reference is that it can dangle, and access memory that is no longer valid. Languages solve this in different ways:

- JavaScript and Go use a garbage collector, at the cost of memory and throughput.
- Swift uses reference counting and weak references, which is still unsafe when misused.
- Rust uses ownership and borrowing, which is sound but has a steep learning curve.
- Zig, C and C++ leave it entirely to the programmer.

Fa takes a different route. Every reference names the collection it points into (`Person in people`), and comes in one of two kinds:

- a **local reference** ([owned reference](/advanced/owned-references)), declared with `use` or received as a parameter, is tracked statically: after a call that invalidates it, the compiler makes you check it or set it again;
- a **stored reference** is a field, and always one side of a [**relation**](/advanced/relations): its target lives in a stable-address collection (`Bag`, `GrowingBag`, `Columnar`), and deleting the target updates the field (an optional becomes `none`, a bag drops the reference).

There are no weak pointers: neither kind can dangle, and neither requires a lifetime annotation.

## Summary

| You write | What happens |
| --- | --- |
| `let b = a`, `mutable b = a` | copy (or move, if `a` is not used afterwards) |
| `use b = a` | alias, no copy |
| `f(a)` | immutable reference, no copy |
| `f(a)` where `f` declares `mutates a` | mutable reference, no copy (`a` must be `mutable`) |
| `for item in collection` | reference, no copy |
| `use item = bag.add(x)` | `x` is moved into the bag, `item` is a reference to it |
| `collection.take(index)` | the element is moved out of the collection |
| `next: Node?` in a type | a copy: the child is stored on the heap, owned by its parent |
| `next: (Node in nodes)?` in a type | a relation: becomes `none` when the target is deleted from `nodes` |
| end of scope | every value declared in it is destroyed |
