Typescript is not great to handle typing of middlewares.

The problem is the following: we have an array of functions, and we want to compose them together.

```ts
const middlewares = [
  functionA(ctx: BaseContext): ContextA {
    ctx.valueA = "hello"
  },
  functionB(ctx: BaseContext): ContextB {
    ctx.valueB = "world"
  },
  // ...
]

const ctx = compose(middlewares, baseCtx)
```

Now, we want to compose them together. The `ctx` value can be transformed by functionA, then functionB, and so on. There are two issues:

1. The type of the `ctx` value is not known from one function to the next.
2. The final type of the `ctx` value is not known.

Problem 2 can actually be solved, but problem 1 is impossible unless generating **a lot of tuples**.

```ts
type Middlewares<ContextA, ContextB extends ContextA, ContextC extends ContextB, ContextD extends ContextC> = [
  Middleware<BaseContext, ContextA>,
] | [
  Middleware<BaseContext, ContextA>,
  Middleware<ContextA, ContextB>,
] | [
  Middleware<BaseContext, ContextA>,
  Middleware<ContextA, ContextB>,
  Middleware<ContextB, ContextC>,
] | [
  Middleware<BaseContext, ContextA>,
  Middleware<ContextA, ContextB>,
  Middleware<ContextB, ContextC>,
  Middleware<ContextC, ContextD>,
]
// ...
```

In Fa, we can solve this problem in two ways:

1. Have a type functions that maps the exact types at a given index.
2. Introduce a new "Composition" type that looks like an array.


Solution 1:

```ts
type Middlewares = Array(
  Middleware(BaseContext, any)
  Map = (index, Self) => Middleware(
    index === 0 ? BaseContext : Self[index - 1]
    any
  )
)
```

Solution 2:

```ts
type Middlewares = Composition(BaseContext)

middlewares = mutable Middlewares [
  middlewareA
  middlewareB
  // ...
]

// can push items like an array
middlewares.push(middlewareC)

// can call all functions at once (unlike an array)
const composed = middlewares.call(ctx)
```

Solution 1 is much more flexible and powerful, but add some compile-time complexity (since we use a type function).

Solution 2 is more rigid, but easier to understand, and have better compile-time performance.
