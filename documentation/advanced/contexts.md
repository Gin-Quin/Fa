# Contexts

A context passes data down a call chain without threading it through every function signature.

Declare a parameter named `context`, and give it a type:

```fa
type MyContext = {
  user: {
    name: String
    email: String
  }
}

function greet = (context: MyContext) => {
  console.log("Hello, {context.user.name}!")
}

function ciao = (context: MyContext) => {
  console.log("Ciao, {context.user.name}!")
}
```

## Passing a context

`context` is an ordinary parameter, so you can pass it explicitly:

```fa
function doTheGreetings = () => {
  greet(context = MyContext {
    user = { name = "John Doe", email = "john.doe@example.com" }
  })

  ciao(context = MyContext {
    user = { name = "John Doe", email = "john.doe@example.com" }
  })
}
```

Or set it once, before calling:

```fa
function doTheGreetings = () => {
  context = MyContext {
    user = { name = "John Doe", email = "john.doe@example.com" }
  }

  greet() -- the context is passed automatically
  ciao()
}
```

## Contexts flow through calls

A function that does not declare a `context` parameter still lets the active context through to the functions it calls:

```fa
function outer = () => {
  context = MyContext {
    user = { name = "John Doe", email = "john.doe@example.com" }
  }

  inner()
}

function inner = () => {
  greet() -- uses the context set by `outer`
}
```

The compiler analyses the call graph, so calling a function that needs a context you have not set is a compile error — at the call site, not at runtime:

```fa
function outer = () => {
  inner() -- error: `inner` reaches `greet`, which requires a MyContext
}

function inner = () => {
  greet()
}
```

## Several contexts at once

Contexts are keyed by **type**, so unrelated contexts coexist:

```fa
type UserContext = { user: User }
type RequestContext = { requestId: String }

function handle = () => {
  context = UserContext { user = currentUser }
  context = RequestContext { requestId = "abc-123" }

  logRequest() -- receives the RequestContext
  greet()      -- receives the UserContext
}

function logRequest = (context: RequestContext) => {
  console.log("Request {context.requestId}")
}
```

Setting a context of a type that is already active replaces it for the rest of the scope.

## Scoped contexts

To change a context for part of a function only, open a [`do`](../control-flow/do) block:

```fa
function doTheGreetings = () => {
  context = MyContext {
    user = { name = "John Doe", email = "john.doe@example.com" }
  }

  do {
    context = MyContext {
      user = { name = "Jane Smith", email = "jane.smith@example.com" }
    }

    greet() -- "Jane Smith"
    ciao()
  }

  greet() -- "John Doe"
  ciao()
}
```

## Contexts are references

A context is passed like any other parameter: as an immutable reference. It is never copied down the call chain, however deep it goes.

To let a callee modify the context, declare the parameter `mutable`:

```fa
function addBreadcrumb = (mutable context: TraceContext, label: String) => {
  context.breadcrumbs.push(label)
}
```

## Contexts and concurrency

A context does not cross a thread boundary on its own. Passing work to a worker or a channel copies the data you send, so a context needed on the other side has to be sent explicitly. See [concurrency](./concurrency).

## When to use them

Contexts suit values that are genuinely ambient for a whole call chain: the current user, a request id, a database transaction, a locale, a cancellation token.

:::note
Use contexts sparingly. They move information implicitly, which is exactly what makes them convenient and exactly what makes code harder to follow. If only two functions need the value, pass it as an argument.
:::
