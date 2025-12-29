# Memory Management

Fa can be compiled to multiple targets: Javascript, Typescript, Native or WebAssembly.

Some of these targets (native and webassembly) are low-level and require explicit memory management. Unless Typescript, when programming with Fa you must manage memory. To allow maximum performance and lean WebAssembly output, Fa does not come with a garbage collector.

The good news is: even though you must manage memory yourself, it is **very simple**. Here is everything you need to know to master memory management in Fa:

1. Objects are allocated on the stack, and only on the stack.
2. **Collections** (arrays, maps, sets, arenas, ...) are the only way to use the heap.
3. Every assignment is a **copy**. This is optimized when assigning a value that is about to be dropped (a move instead of a copy is made instead).
4. There are no pointers or references. Instead, Fa introduces the unique concept of **relations**. A **relation** is a kind of memory-safe weak reference that declares how it's defined and removed.
5. **Iterating over a collection** returns **references** to avoid copying the whole collection.
6. All **functions parameters** are **immutable references**. This means that your parameters are never copied when passed to a function.
7. The only way to have a memory leak in Fa is by creating a **circular references** in a collection. Even though it will very rarely happen, it's good to know how to detect and avoid it.

This set of rules allows Fa to have a very simple memory model that is very easy to understand and reason about, while still allowing peak performance and lean binary output (native or WebAssembly). You will feel like you are programming in a higher level language, but with the best performance.

## Objects

Objects are never allocated on the heap. This can seem surprising at first, but it has many advantages:

- Heap allocation is made at collection level, rather than object level. This means your allocations are large fields easy to deallocate at once rather than very small allocations scattered all over the place.
- Objects always stay relatively small (compared to collections), so it's not a performance issue to copy them around.
- They are 100% safe to use, as circular reference and dangling pointers cannot exist on objects on the stack.

```ts
let object = {
  x = 12
  hello = "world"
  nested = {
    y = 21
  }
}

let object2 = object // unlike Typescript, this is a deep copy (structured clone) and not a pointer

// unlike Rust, since there is no ownership, you can still use the original object after copying it
object.hello = "you"

console.log(object2.hello) // "you" -- would log "world" in Typescript
```

### But... copying objects is heavy!

It may seem that using copy instead of references will have a huge impact on performance.

It's actually not true, for several reasons:

- Fa still uses references most of the time, i.e. whenever it's safe (iterating over collections, aliases, relations). You actually only copy when you **should** copy.
- Because of the structural typing approach of Fa, you will, most of the time, need to pick "fields" of objects, without requiring to copy of all it.

If you come from a Typescript background, everytime you destructure function parameters, like this:

```ts
function parametersAreCopied({ foo, bar }: { foo: string, bar: string }) {
  // ...
}
```

...then you actually create a copy of all destructured fields when you pass to the function. Which, if you're a modern Typescript programmer, you probably do quite often.

Our take is that the default "copy" behavior brings zero to very low performance cost while providing a lot of simplicity and memory safety.

## Collections

Collections are the only way to use the heap in Fa. They are used to store data in Fa, and are the only way to allocate memory on the heap.

You don't need any `alloc` or `realloc` in Fa. Instead, you can simply add elements to a collection.

### Iterating over a collection

When iterating over a collection, Fa returns **references** to the elements. This means that you are not copying the whole collection, but only the reference to the element you are iterating over.

```ts
let array = [1, 2, 3]

for array >> element, index {
  console.log("{index} = element") // will log [1, 2, 3]
}
```

### Aliases

If you want to get the value of a collection and update / modify it, you can use an alias with the `use` keyword:

```ts
let arrayOfHumans: Array(Human) = [("John", 20), ("Jane", 21), ("Joe", 22)]

// what if we want to do a series of operations on myFavoriteHuman?
// we can't assign our human to a variable since this will copy the human
let myFavoriteHuman = arrayOfHumans[1]
myFavoriteHuman.age = 23 // this will not modify the original array
console.log(arrayOfHumans[1].age) // will still log 21

// to leverage this, we can use an alias
// this will not copy the human, but will create a reference to the original human
use myFavoriteHuman = arrayOfHumans[1]

myFavoriteHuman.age = 23 // is exactly the same as writing `arrayOfHumans[1].age = 23`

console.log(arrayOfHumans[1].age) // will log 23
```

### The `Bag` collection

The `Bag` collection is a special collection optimized to store unordered objects. It's very similar to a `Set`, but without the unicity constraint.

```ts
let bag = Bag(Human)

bag.add(Human("John", 20))
bag.add(Human("Jane", 21))
bag.add(Human("John", 22))

for bag >> human {
  console.log(human.name) // will log "John", "Jane", "John" -- order is not guaranteed
}

bag.delete(Human("John", 20)) // delete the first human with name "John" and age 20
```

## References

Fa has a unique way of handling references. Instead of relying on "wild" references, also called pointers in C / C++ / Go, Fa defines what we call "owned references" and "relationships".

A "wild" reference is a number that points to an arbitrary memory location. The danger of using these references is that you can easily create dangling pointers that access invalid memory locations.

This has always been a complicated problem, solved in different ways:

- Javascript and Go use a garbage collector, but at the cost of high memory usage and reduced performance.
- Swift uses weak references, but this is still unsafe.
- Rust solves this problem using "ownership," "borrowing," and enforcing strict rules, but at the cost of a very steep learning curve.
- Zig, C, and C++ do not solve the issue, giving complete freedom to the programmer to handle things their way—which can be both good and bad.

Fa takes a new approach that is safe, easy to understand, and enjoyable to use by employing "owned references" and "relationships."
