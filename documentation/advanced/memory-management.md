# Memory management

Fa compiles to JavaScript, to native code and to WebAssembly. The last two are low-level targets with no garbage collector, so memory has to be managed. To allow maximum performance and lean WebAssembly output, Fa does not come with a garbage collector either: in Fa, the compiler manages memory for you, from rules simple enough to hold in your head.

Here is the whole model:

1. **Objects** are values stored **inline**, where they are declared: in the frame of a function, inside their parent object, or in a collection slot. They are never allocated on their own.
2. **Collections** (arrays, sets, maps, pools, columnars) own the **heap**. The only other heap allocations are made by the compiler: for [recursive types](#recursive-types), and for the [blocks listed below](#heap-blocks-made-by-the-compiler).
3. **Every assignment is a copy**, except for the thread-safe handles (`Mutex`, atomics, `Channel`), which are shared. The compiler turns a copy into a move when the source is not used afterwards, by its name or through an alias. The difference is never observable: identity is not observable, and Fa has no destructors, except the `destroy` of the types that hold a C resource, which can't be copied (see [foreign code](/advanced/foreign-code#releasing-c-resources)).
4. **`use` creates an alias** instead of copying. An alias cannot outlive the value it refers to.
5. **Function parameters are immutable references**, so arguments are never copied when passed. A function that modifies a parameter declares it with the `mutates` effect.
6. **Iterating over a collection yields references**, so iteration never copies.
7. **Every value is destroyed at the end of its scope.**
8. **A field holds a copy**, unless it is one side of a **relation**: a reference that names the collection its target lives in (`bestFriend: (Person in people)?`), mirrored by a declared reverse side (`bestFriendOf: Pool(people->bestFriend)`), and kept consistent by the compiler.

You never ask *"when should I free this?"* or *"which allocator should I use?"*. The question in Fa is *"where should this object live?"*. And the answer is part of the type: a `Person` stored in the collection `persons` has the type `Person in persons` (see [owned references](/advanced/owned-references)).

## Objects are stored inline

An object is never allocated on its own: a local object lives in the frame of its function, a field in its parent, an element in its collection. This has three consequences worth understanding:

- Heap allocation happens at the collection level, not at the object level. Allocations are few and large, rather than many and scattered, and they are released at once.
- Objects stay small relative to collections, so copying them is cheap.
- An object has no pointer of its own to dangle, and can't form a reference cycle, so objects are safe by construction.

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

Because each value owns its children, these structures are trees: two nodes can't share the same child. For structures where objects need to point to each other (graphs, doubly linked lists, parents and children), store the objects in a `Pool` and link them with [relations](/advanced/relations): `next: (Node in nodes)?` and its reverse side `previous: (nodes->next)?`.

## Collections own the heap

Collections are the only explicit way to use the heap in Fa. Instead of `alloc` and `realloc`, you add elements to a collection, and each collection uses the allocation strategy that suits it.

### Heap blocks made by the compiler

Besides collections and [recursive types](#recursive-types), the compiler allocates a heap block, which you never write, for:

- the frame of a [task](/advanced/concurrency#tasks), which outlives the call that started it, and is freed when the task ends;
- the environment of an escaping [closure](#closures): the copies it captured, owned by the closure, copied and destroyed with it;
- the thread-safe handles: a [`Mutex`](/advanced/concurrency#mutexes), an [atomic](/advanced/concurrency#atomics) or a [`Channel`](/advanced/concurrency#the-channel-type).

A thread-safe handle is reference-counted: copying it shares the same mutex, atomic or channel instead of creating a new one, so that several tasks and threads can use it. This is the only exception to "every assignment is a copy", and the only reference counting in Fa. The handle is destroyed when its last copy is.

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

An alias to an element of a collection is called an [owned reference](/advanced/owned-references): its type tells which collection owns it (`Human in people`), and the compiler tracks when it is invalidated. After that, it must be declared again before it is used.

`use` takes a place (a value, a field, an element), or a call that returns a reference, like `pool.add` below. Anything else is an error: `use total = computeTotal()` has no place to designate, so write `let`. See [`use` and `mutable use`](/advanced/reference-checking#use-and-mutable-use).

Collections that return a reference when an element is added, like [`Pool`](/collections/pools), are used with `use` too:

```fa
mutable pool: Pool(Human) = []

use john = pool.add({ name = "John", age = 20 }) -- a reference to the stored human
john.age += 1 -- modifies the human stored in the pool
```

The literal is built directly in the pool, without a temporary object to copy. Binding the result with `let` would copy the stored human out of the pool, so it is a bad practice:

```fa
let john = pool.add({ name = "John", age = 20 }) -- bad practice: `john` is a copy of the stored human
```

### Removing while iterating

Adding or removing elements of a collection while iterating over it would invalidate the references the iteration hands out, or change what it visits, so the compiler rejects it:

```fa
mutable pool: Pool(Human) = []

for human in pool {
	if human.name == "John" {
		pool.delete(human) -- error: `pool` is being iterated
	}
}
```

Use `deleteWhere` instead:

```fa
pool.deleteWhere((human) => human.name == "John")
```

The same goes for the operations that change a collection indirectly, like deleting a linked object, which also removes it from the [relations](/advanced/relations) that pointed at it. See [iteration](/advanced/reference-checking#iteration).

## Parameters are references

Parameters are immutable references: passing a value to a function never copies it. A function that modifies a parameter declares it with the `mutates` effect, and can only be called with a `mutable` value:

```fa
function birthday = (human: Human) mutates human => {
	human.age += 1
}

mutable john = Human { name = "John", age = 20 }
birthday(john) -- `john` is modified in place, without any copy
```

A mutation keeps the mutated place valid, but replacing a value destroys what it owned: the compiler deduces from the type which references are no longer valid after the call. Writing a number invalidates nothing; writing a string invalidates its slices; mutating a whole array, as `push` does, invalidates every reference into it. A function that removes precise elements of a collection, or shifts them, declares the `invalidates` effect instead. See [owned references](/advanced/owned-references#invalidation).

## Lifetimes and destruction

Every value is destroyed at the end of the scope that declared it, in reverse order of declaration.

```fa
function example = () => {
	mutable a: Array(Integer) = []
	mutable b: Array(Integer) = []
} -- `b` is released, then `a`
```

Because references cannot outlive what they point into, and because nothing is shared implicitly, destruction never has to be deferred, counted or traced. There is no garbage collector, no reference counting (except for the [thread-safe handles](#heap-blocks-made-by-the-compiler)), and no `free`.

Returning a reference out of a function is allowed only when it points into something the caller already owns. Otherwise, the value is copied out. See [owned references](/advanced/owned-references#returning-a-reference).

## Closures

A closure that **escapes** (it is stored, returned, or given to a started task) captures a **copy** of each value it uses, taken when it is created, exactly like an assignment. This includes the values designated by parameters, `use` aliases and loop variables. Such a closure can be stored, returned, and sent to another thread without any lifetime question arising.

```fa
type Greeter = (name: String) => String

function makeGreeter = (greeting: String): Greeter => (name) => "{greeting}, {name}!"

let greetings = [makeGreeter("Hello"), makeGreeter("Hi")]
```

A closure that does not escape (it is called during the call it is passed to, like the callback of `map`) captures by reference instead: it sees the current values, as in JavaScript, and copies nothing. The compiler checks that the call doesn't modify what the closure reads (see [arguments that alias](/advanced/reference-checking#arguments-that-alias)). A closure can't capture a `mutable use`. See [closures](/functions/defining-functions#closures).

## References

Instead of "wild" references (a number pointing at an arbitrary memory location, as in C, C++ or Go), Fa has two kinds of safe references.

The danger of a wild reference is that it can dangle, and access memory that is no longer valid. Languages solve this in different ways:

- JavaScript and Go use a garbage collector, at the cost of memory and throughput.
- Swift uses reference counting and weak references, which is still unsafe when misused.
- Rust uses ownership and borrowing, which is sound but has a steep learning curve.
- Zig, C and C++ leave it entirely to the programmer.

Fa takes a different route. Every reference names the collection it points into (`Person in people`), and comes in one of two kinds:

- a **local reference** ([owned reference](/advanced/owned-references)), declared with `use` or received as a parameter, is tracked statically: after a call that invalidates it, the compiler makes you declare it again;
- a **stored reference** is a field, and always one side of a [**relation**](/advanced/relations) whose reverse side is declared too: both sides live in stable-address collections (`Pool`, `Columnar`, `GrowingPool`), every write updates both sides, and deleting an object eagerly cleans up every relation it was part of (an optional becomes `none`, a pool drops the reference).

There are no weak pointers and no generation counters: neither kind can dangle, and neither requires a lifetime annotation. The exact rules of the compiler are in [reference checking](/advanced/reference-checking).

## Summary

| You write | What happens |
| --- | --- |
| `let b = a`, `mutable b = a` | copy (or move, if `a` is not used afterwards: this is never observable) |
| `use b = a` | alias of the place `a`, no copy (`a` must be a place, or a call that returns a reference) |
| `f(a)` | immutable reference, no copy |
| `f(a)` where `f` declares `mutates a` | mutable reference, no copy (`a` must be `mutable`) |
| `for item in collection` | reference, no copy |
| `use item = pool.add({ … })` | the object is built directly in the pool, `item` is a reference to it |
| `use item = pool.add(x)` | `x` is moved into the pool (copied without its extra fields if it has more than the element type), `item` is a reference to it |
| `let item = pool.add(…)` | bad practice: `item` is a copy of the stored element |
| `collection.take(index)` | the element is moved out of the collection |
| `next: Node?` in a type | a copy: the child is stored on the heap, owned by its parent |
| `next: (Node in nodes)?` and `previous: (nodes->next)?` in a type | a relation: both sides are kept in sync, and `next` becomes `none` when its target is deleted from `nodes` |
| end of scope | every value declared in it is destroyed |
