# Structural typing

I want to allow to transpile some Fa's structural typing to Rust system, but... it's quite complicated.

It's quite possible, if I want to use Rust as a target language, that I'll have to give up some features of structural typing.

## Problem 1: Building an object from another object

For copiable types, it's straightforward. But for containers, it's not.

Three solutions that I've found:

1. Use `Cow` to avoid copying the container (there is one owner, and immutable references are built with Cow::Borrowed).
2. Use `Rc` to allow to clone the container (works but there is the overhead of the Rc).
3. Use **atomic traits**.
4. Use **opportunistic traits** (a variant of atomic traits).

## Problem 2: Nested structural objects

Let's have this Typescript example:

```
type SubObject = { x: number }
type SuperObject = { ...SubObject, y: number }

function logAllSubObjects(subObjects: Array<SubObject>) { }

const superObjects: Array<SuperObject> = [
  { x: 1, y: 2 },
  { x: 3, y: 4 },
  { x: 5, y: 6 },
  ...
]

logAllSubObjects(superObjects)
```

In TypeScript, this works because the type system is able to infer that `SuperObject` has all the properties of `SubObject`.

But in Rust... it's complicated. A solution would be to map over the array and to create a new array of `SubObject` using one of the solutions above.

But mapping over an array is terrible performance wise.

Atomic traits may be the only solution.

## Atomic traits

Atomic traits are traits that implement one single value via a getter.

They are defined at the lowest level of a program, and generated when needed.

When you define structs:

```rs
struct MyStruct {
  x: i32,
  y: i32,
}

struct MyOtherStruct {
  x: i32,
  z: i32,
}
```

Then Fa will generate one atomic trait for every field of the struct:

```rs
trait x_i32 {
  fn x(&self) -> i32;
}

trait y_i32 {
  fn y(&self) -> i32;
}

trait z_i32 {
  fn z(&self) -> i32;
}
```

The trait x_i32 can be reused by every other struct that has a field `x` of type `i32`. It's generated once and only once, even if it's used in many structs.

Then, Fa will also generate the implementations of every atomic trait:

```rs
impl x_i32 for MyStruct {
  #[inline(always)]
  fn x(&self) -> i32 { self.x }
}

impl y_i32 for MyStruct {
  #[inline(always)]
  fn y(&self) -> i32 { self.y }
}

impl x_i32 for MyOtherStruct {
  #[inline(always)]
  fn x(&self) -> i32 { self.x }
}

impl z_i32 for MyOtherStruct {
  #[inline(always)]
  fn z(&self) -> i32 { self.z }
}
```

Then, it's possible to compose the different atomic traits to build a custom type:

```rs
fn call_x<HasX: x_i32>(x: HasX) {
  println!("x: {}", x.x());
}

fn call_x_y<HasXAndY: x_i32 + y_i32>(x: HasXAndY) {
  println!("x: {}", x.x());
  println!("y: {}", x.y());
}
```

This will work with **every type** that implements `x_i32`.

```rs
call_x(MyStruct { x: 1, y: 2 });
call_x(MyOtherStruct { x: 3, z: 4 });

call_x_y(MyStruct { x: 1, y: 2 });
```

This works and allows structural typing in Rust (🎉), but has some downsides:

- Very bad compilation times: all crates must be re-compiled on any change from any other crate.
- The final binary size will be bigger than pure Rust, and will grow more and more as the project size increases.

But the good news is that there is no runtime cost! Fa will be the fastest programming language that implements structural typing ever ⚡️

### Dynamic dispatch

It's not possible to use atomic traits with dynamic dispatch, as dynamic dispatch does not allow composing these traits.

### Generics

It's possible to use generic types into traits:

```rs
struct MyStruct<T> {
  vector: Vec<T>,
}

trait Vector<T> {
  fn vector(&self) -> &Vec<T>;
}
```

## Opportunistic traits

Another technique that is similar to atomic traits is "opportunistic" traits.

I.e. traits are implemented on the fly, when needed.

It has some advantages:

- No need for a "central" atomic traits crate. Instead, the just-in-time "opportunistic" traits are implemented by crate. So it can speed up compilation times.
- The final binary size might be smaller.

The two techniques are quite close, starting with the easiest one to implement and then adding an option to use the other one can be a way to make the best of both worlds. It'll also allow to compare the two techniques IRL to see which one is best.

With opportunistic traits, the idea is the following:

- Every struct has (when necessary, i.e. when it's used in a function signature) its interface counterpart.

```rs
struct MyStruct {
  x: i32,
  y: i32,
}

// Only generate this when it's used in a function signature
trait MyStructInterface {
  fn x(&self) -> i32;
  fn y(&self) -> i32;
}
```


Any function declared anywhere using MySuperStruct as a parameter will trigger the generation of the trait:

```rs
// This will cause the generation of the trait MyStructInterface that maps all the fields of MyStruct
fn print_my_struct(my_struct: impl MyStructInterface) {
  println!("x: {}", my_struct.x());
  println!("y: {}", my_struct.y());
}
```

Whenever a conversion from a type to MyStructInterface is needed, the implementation will be generated.

```rs
struct MySuperStruct {
  x: i32,
  y: i32,
  z: i32,
}

// This will only be generated when MySuperStruct is used in a function signature that requires a MyStruct type
impl MyStructInterface for MySuperStruct {
  fn x(&self) -> i32 { self.x }
  fn y(&self) -> i32 { self.y }
}

// ...
// there can be many other implementations
```

Whenever the function `print_my_struct` is called with a `MySuperStruct`, the implementation of `MyStructInterface` for `MySuperStruct` will be generated.

```rs
let my_super_struct = MySuperStruct { x: 1, y: 2, z: 3 };

// This triggers the generation of the implementation of MyStructInterface for MySuperStruct
print_my_struct(my_super_struct);
```

✨ Unless atomic traits are surprisingly performants, opportunistic traits seem to be the perfect solution, with no runtime overhead and mathematically minimalist code generation.
