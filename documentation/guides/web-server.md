# Implementing a web server

:::caution
The `std.http` standard library API shown here is **provisional** and will change. The language features it demonstrates — errors, contexts, composition, reactivity — are stable.
:::

This guide builds a small JSON API, and shows how Fa's features fit together in a server.

## A minimal server

```fa
-- application/main.fa
use { Server, Response } = std.http

export function = (): Int | ServerError => {
  mutable server = Server(port = 3000)

  server.get("/health", (request: Request): Response => Response.text("ok"))

  server.listen()!

  return 0
}
```

```bash
fa develop
```

Target `bun` during development for fast reloads, and `native` for deployment:

```
platforms = ["bun", "native"]
```

## Typed request and response bodies

Types are the schema. `Json.parse` takes the type you expect, and returns it or an error:

```fa
type CreateUser = {
  name: String
  email: String
  age: Int(min = 0, max = 150)?
}

server.post("/users", (request: Request): Response => {
  let body = when Json.parse(CreateUser, request.body) is {
    CreateUser >> value => value
    ParseError >> error => return Response.badRequest(error.message)
  }

  let user = createUser(body)

  return when user is {
    User >> created => Response.json(created, status = 201)
    Error >> error => Response.serverError(error)
  }
})
```

The range on `age` is enforced by the parser: a payload with `age: 200` is rejected before your handler runs, with no validation code of your own.

## Errors as values

A handler returns a `Response`, so every failure has to be turned into one. This is the point where Fa's error model pays off: the compiler will not let you forget a case.

```fa
type ApiError = Errors {
  NotFound = { resource: String, id: Int }
  Unauthorized
  Conflict = { message: String }
}

function toResponse = (error: ApiError): Response => when error is {
  NotFound >> e => Response.json({ error = "{e.resource} {e.id} not found" }, status = 404)
  Unauthorized => Response.json({ error = "unauthorized" }, status = 401)
  Conflict >> e => Response.json({ error = e.message }, status = 409)
}

server.get("/users/{id}", (request: Request): Response => {
  let id = when Int.fromString(request.parameters["id"]!) is {
    Int >> value => value
    Error => return Response.badRequest("id must be a number")
  }

  return when loadUser(id) is {
    User >> user => Response.json(user)
    ApiError >> error => toResponse(error)
  }
})
```

Adding a member to `ApiError` makes `toResponse` fail to compile until you handle it.

## Middleware by composition

Middleware is [function composition](../functions/function-composition#middleware): each step adds fields to the context, and the type system enforces the ordering.

```fa
type RequestContext = { request: Request }
type Timed = { startedAt: Int }
type Authenticated = { user: User }

function withTiming = (Ctx: Type(Object), context: Ctx): Ctx & Timed => {
  ...context
  startedAt = now()
}

function withAuth = (Ctx: Type(RequestContext), context: Ctx): (Ctx & Authenticated) | ApiError => {
  let token = context.request.headers["authorization"] ?? ""
  let user = verifyToken(token)!

  return {
    ...context
    user
  }
}

function handler = (Ctx: Type(Authenticated), context: Ctx): Response =>
  Response.json({ greeting = "Hello, {context.user.name}" })

server.get("/me", withTiming ||> withAuth ||> handler)
```

Putting `handler` before `withAuth` is a compile error, because its constraint would not be satisfied. An `ApiError` from `withAuth` short-circuits the chain — see [errors in a composition](../functions/function-composition#errors-in-a-composition).

## Ambient values with contexts

A database transaction or a request id is genuinely ambient. Pass it with a [context](../advanced/contexts) rather than threading it through every function:

```fa
type Database = { connection: Connection }

server.post("/orders", (request: Request): Response => {
  use transaction = pool.begin()

  context = Database { connection = transaction.connection }

  let order = placeOrder(request.body)

  return when order is {
    Order >> created => {
      transaction.commit()
      Response.json(created, status = 201)
    }
    Error >> error => toResponse(error)
  }
}) -- the transaction is rolled back here if it was not committed
```

`placeOrder` and everything it calls receive the `Database` context without declaring it, and the compiler rejects a call path that would need it without one.

## Concurrency

Handlers are `async`, and the runtime schedules them. Nothing needs to be marked:

```fa
server.get("/dashboard", async (request: Request): Response => {
  let results = await [
    start loadProfile(userId)
    start loadOrders(userId)
    start loadNotifications(userId)
  ]

  return Response.json({
    profile = results[0]!
    orders = results[1]!
    notifications = results[2]!
  })
})
```

See [concurrency](../advanced/concurrency).

## Reactive server state

Reactivity is not only for user interfaces. Use it to keep derived state consistent:

```fa
reactive configuration = loadConfiguration()

derived poolSize = configuration.database.connections

effect {
  connectionPool.resize(poolSize)
}
```

Reloading the configuration resizes the pool, with no callback wiring.

## Routes from the file system

Generate the routing table at compile time, so it costs nothing at startup:

```fa
static routeFiles = @readDirectory("../routes")

@for routeFiles >> file {
  server.route(file.path, routes[file.name])
}
```

See [metaprogramming](../advanced/metaprogramming).

## Testing

Handlers are ordinary functions, so they test without a running server:

```fa
export namespace = {
  test "rejects a malformed body" {
    let response = handleCreateUser(Request { body = "{ not json" })

    assert(response.status == 400)
  }
}
```

## Deploying

```bash
fa build --platform native
```

The binary in `build/native/` has no runtime dependency: no Node, no interpreter, no shared libraries beyond libc. Deploy it as it is, or in a scratch container.

For edge runtimes, build for WebAssembly:

```bash
fa build --platform wasm
```
