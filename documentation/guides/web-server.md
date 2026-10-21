# Implementing a web server

:::caution
The `std.http` standard library API shown here is **provisional** and will change. The language features it demonstrates (errors, requirements, composition, concurrency, reactivity) are stable.
:::

This guide builds a small JSON API, and shows how Fa's features fit together in a server.

## A minimal server

```fa
-- application/main.fa
use { Server, Request, Response, ServerError } = std.http

export function = (): Integer throws ServerError => {
	mutable server = Server { port = 3000 }

	server.get("/health", (request: Request): Response => Response.text("ok"))

	try server.listen()

	return 0
}
```

```bash
fa develop
```

Target `bun` during development for fast reloads, and `native` for deployment:

```fa
-- package.fa
export = {
	name = "api"
	version = "0.1.0"
	targets = ["bun", "native"]
}
```

## Typed request and response bodies

Types are the schema. `Json.parse` reads the type you expect from the context, and either returns a value of that type or throws an error:

```fa
-- application/handleCreateUser.fa
use { Request, Response } = std.http

type CreateUser = {
	name: String
	email: String
	age: Integer(min = 0, max = 150)?
}

export function = (request: Request): Response => {
	let body: CreateUser = Json.parse(request.body) catch error => return Response.badRequest(error.message)

	let user = createUser(body) catch error => return toResponse(error)

	return Response.json(user, status = 201)
}
```

```fa
server.post("/users", handleCreateUser)
```

The range on `age` is enforced by the parser: a payload with `"age": 200` is rejected before your handler runs, with no validation code of your own.

`catch` handles a failure where it happens. Here, each `catch` turns the error into a response and returns it from the handler.

## Errors are part of the signature

A handler returns a `Response`, so every failure has to be turned into one. This is the point where Fa's error model pays off: the compiler will not let you forget a case.

```fa
type ApiError = Errors {
	NotFound = { resource: String, id: Integer }
	Unauthorized
	Conflict = { message: String }
}

function toResponse = (error: ApiError): Response => when error is {
	ApiError.NotFound as notFound => Response.json({ error = "{notFound.resource} {notFound.id} not found" }, status = 404)
	ApiError.Unauthorized => Response.json({ error = "unauthorized" }, status = 401)
	ApiError.Conflict as { message } => Response.json({ error = message }, status = 409)
}

function loadUser = (id: Integer): User throws ApiError => {
	let user = users.get(id)

	if user is none {
		throw ApiError.NotFound { resource = "user", id }
	}

	return user
}

server.get("/users/{{id}}", (request: Request): Response => {
	let text = request.parameters["id"] catch error => return Response.badRequest("missing id")
	let id = Integer.fromString(text) catch error => return Response.badRequest("id must be a number")
	let user = loadUser(id) catch error => return toResponse(error)

	return Response.json(user)
})
```

`loadUser` declares `throws ApiError`: a caller cannot ignore it, and must either propagate it with `try` or handle it with `catch`. `when` is exhaustive, so adding a member to `ApiError` makes `toResponse` fail to compile until you handle it.

Note the route: `{` starts an interpolation in a string, so the literal braces of the route pattern are written `{{id}}`.

## Middleware by composition

Middleware is [function composition](/functions/function-composition): each step returns its input enriched with new fields, and the type system enforces the ordering. Object types are composed by spread, and objects are enriched by spread too:

```fa
type Timestamp = { timestamp: Integer }
type Authenticated = { user: User }

function addTimestamp = (request: Request): { ...Request, ...Timestamp } => {
	return { ...request, timestamp = now() }
}

function authenticate = (request: { ...Request, ...Timestamp }): { ...Request, ...Timestamp, ...Authenticated } throws ApiError => {
	let token = request.headers.get("authorization") ?? ""
	let user = try verifyToken(token)

	return { ...request, user }
}

function showProfile = (request: { ...Request, ...Authenticated }): Response =>
	Response.json({ greeting = "Hello, {request.user.name}" })

function profile = addTimestamp then authenticate then showProfile

server.get("/me", (request: Request): Response => profile(request) catch error => toResponse(error))
```

Putting `showProfile` before `authenticate` is a compile error, because its parameter needs a `user` field that nothing has added yet. The composition `profile` can fail with the errors of its steps, here `ApiError`, which the route handles with `catch`.

## Requirements for ambient values

A database transaction or a request id is genuinely ambient. Rather than threading it through every function as an argument, declare it as a [requirement](/advanced/requirements) with `requires`, and give it a value with `provide`:

```fa
type Database = {
	connection: Connection
}

function placeOrder = (body: String): Order throws ApiError requires database: Database => {
	-- uses `database.connection`, and so can every function it calls
	...
}

server.post("/orders", (request: Request): Response => {
	mutable transaction = pool.begin()

	provide Database = { connection = transaction.connection }

	let order = placeOrder(request.body) catch error => return toResponse(error)

	transaction.commit()

	return Response.json(order, status = 201)
}) -- the transaction is rolled back here if it was not committed
```

`placeOrder` receives the `Database` without it being passed at the call site: requirements are matched by type. Its signature still says that it needs one, and the compiler rejects any call path where no `Database` has been provided.

## Concurrency

An `async function` can `await`. `start` runs a call in the background and returns a `Task`, so several loads can run at the same time:

```fa
async function showDashboard = (request: Request): Response => {
	let userId = request.parameters["userId"] catch error => return Response.badRequest("missing user id")

	let profileTask = start loadProfile(userId)
	let ordersTask = start loadOrders(userId)
	let notificationsTask = start loadNotifications(userId)

	let profile = await profileTask catch error => return toResponse(error)
	let orders = await ordersTask catch error => return toResponse(error)
	let notifications = await notificationsTask catch error => return toResponse(error)

	return Response.json({ profile, orders, notifications })
}

server.get("/dashboard/{{userId}}", showDashboard)
```

Async in Fa is colorless: `loadProfile` and the others are ordinary `async` functions, and the caller decides whether to block, `await` or `start` them. See [concurrency](/advanced/concurrency).

## Reactive server state

Reactivity is not only for user interfaces. Use it to keep derived state consistent:

```fa
reactive configuration = loadConfiguration()

derived poolSize = configuration.database.connections

effect {
	connectionPool.resize(poolSize)
}
```

Reloading the configuration resizes the pool, with no callback wiring. See [reactivity](/advanced/reactivity).

## Routes from the file system

Generate the routing table at compile time, so it costs nothing at startup:

```fa
static routeFiles = @readDirectory("../routes")

@for file in routeFiles {
	server.route(file.path, routes[file.name])
}
```

`routeFiles` is a `static` value: it exists only at compile time, and `@for` unrolls the loop into one `server.route` call per file. See [metaprogramming](/advanced/metaprogramming).

## Testing

Handlers are ordinary functions, so they test without a running server:

```fa
-- application/handleCreateUser.test.fa
test "rejects a malformed body" {
	let response = handleCreateUser(Request { body = "{{ not json" })

	assert response.status == 400
}
```

A handler that `requires` a value is tested by providing one with `provide`, for example an in-memory `Database`. See [testing](/architecture/testing).

## Deploying

```bash
fa build --target=native
```

The binary in `build/native/` has no runtime dependency: no Node, no interpreter, no shared libraries beyond libc. Deploy it as it is, or in a scratch container.

For edge runtimes, build for WebAssembly:

```bash
fa build --target=wasm
```
