# Function Composition

Function composition combines several functions into a single one. In Fa, functions are composed with the `then` keyword.

## Basic composition

`first then second` creates a new function that calls `first`, then passes its result to `second`. Functions are applied from left to right, in the order you read them. Composing calls nothing: it only builds a new function.

```fa
-- Define some simple functions
function increment = (x: Integer): Integer => x + 1
function double = (x: Integer): Integer => x * 2
function square = (x: Integer): Integer => x * x

-- Compose functions
function incrementAndDouble = increment then double

-- Use the composed function
let result = incrementAndDouble(3) -- equivalent to double(increment(3)), 8

-- Extend the composition with another function
function incrementAndDoubleAndSquare = incrementAndDouble then square

let result2 = incrementAndDoubleAndSquare(3) -- equivalent to square(double(increment(3))), 64
```

`then` has the lowest precedence of all operators, so `a then b then c` never needs parentheses.

## Type safety

Function composition is type-safe: the return type of each function must match the parameter type of the next one.

```fa
function stringLength = (text: String): Integer => text.length
function toText = (n: Integer): String => n.toString()

-- Valid composition: Integer -> String -> Integer
function getDigitCount = toText then stringLength

console.log(@typeToString(getDigitCount)) -- "(Integer) => Integer"
```

```fa
function invalid = stringLength then stringLength -- error: `stringLength` expects a `String`, but receives an `Integer`
```

## Errors in a composition

A step of a composition can throw [errors](/values/errors). The composed function `throws` the union of the errors of all its steps:

```fa
function loadUser = (id: Integer): User throws DatabaseError => ...
function loadScore = (user: User): Integer throws ApiError => ...

-- (id: Integer) => Integer throws DatabaseError | ApiError
function loadUserScore = loadUser then loadScore
```

The composition stops at the first error: the remaining steps are not called, and the error is thrown by the composed function. It is handled like any other error:

```fa
let score = loadUserScore(42) catch error => 0
```

A step never receives the error of a previous step. To recover from an error in the middle of a chain, handle it in a function of its own, and compose that function instead:

```fa
function loadScoreOrZero = (user: User) => loadScore(user) catch error => 0

-- (id: Integer) => Integer throws DatabaseError
function loadUserScoreOrZero = loadUser then loadScoreOrZero
```

## Composition ranges

A composition keeps track of the functions it is made of. You can extract part of it using the [range](/values/ranges) syntax, as you would with an array. This is what makes compositions reusable for migrations, middlewares or plugins.

```fa
-- Define some simple functions
function one = (x: Integer): Integer => x + 1
function two = (x: Integer): Integer => x * 2
function three = (x: Integer): Integer => x - 3
function four = (x: Integer): Integer => x // 4

-- Create a composition of all functions
function oneToFour = one then two then three then four

-- Extract ranges from the composition
function twoToFour = oneToFour[1..]    -- equivalent to (two then three then four)
function twoToThree = oneToFour[1..3]  -- equivalent to (two then three)
function oneToThree = oneToFour[..=2]  -- equivalent to (one then two then three)

-- You can also extract a single function from the composition
function justTwo = oneToFour[1]        -- equivalent to two
```

Ranges follow the usual rule: `..` excludes the upper bound, `..=` includes it.

### Slicing needs a known chain

Slicing only works on a function whose composition chain is known at compile time, like a `function` declared with `then`. Once a composition is stored with a plain function type, only its signature is kept: the chain is erased, and slicing is no longer available.

```fa
-- valid: a composition can be passed as a plain function
function runAll = (pipeline: (Integer) => Integer) => pipeline(1)
runAll(oneToFour)

-- error: `pipeline` is a plain `(Integer) => Integer`, it has no composition chain
function runFromTwo = (pipeline: (Integer) => Integer) => pipeline[1..](1)
```

## Practical applications

### Middlewares

Composition is a natural fit for middlewares, especially when each middleware enriches a context object with new properties. With [generic functions](/types/generics), each middleware keeps the properties added by the previous ones:

```fa
type Timestamp = { timestamp: Integer }
type Auth = { auth: String }
type Logged = { logged: Boolean }

-- Each middleware returns its input enriched with new properties
function addTimestamp(Context: Request) = (request: Context): { ...Context, ...Timestamp } => {
	{
		...request
		timestamp = getCurrentTime()
	}
}

function addAuthHeader(Context: Request) = (request: Context): { ...Context, ...Auth } => {
	{
		...request
		auth = getAuthToken()
	}
}

function logRequest(Context: { ...Request, ...Timestamp }) = (request: Context): { ...Context, ...Logged } => {
	console.log("Logging request at {request.timestamp}")
	{
		...request
		logged = true
	}
}

-- Each middleware can rely on the properties added by the previous ones
function requestMiddleware = addTimestamp then addAuthHeader then logRequest

-- The final type is inferred through the whole chain:
-- (request: Request) => { ...Request, ...Timestamp, ...Auth, ...Logged }

function processRequest = (request: Request): Response => {
	let enrichedRequest = requestMiddleware(request)
	console.log(enrichedRequest.timestamp)
	console.log(enrichedRequest.auth)
	console.log(enrichedRequest.logged)
	handleRequest(enrichedRequest)
}
```

The type system ensures that the middlewares are composed in the correct order: a middleware requiring a property must come after the middleware that adds it. Putting `logRequest` before `addTimestamp` is a compile error, because its constraint would not be satisfied.

### Plugins

The same pattern can be used to build plugin systems, where each plugin extends the application:

```fa
type App = { name: String, version: String }
type Config = { config: AppConfig }
type Router = { router: AppRouter }
type Database = { database: DatabaseConnection }

-- Core application setup
function setupConfig(Application: App) = (app: Application): { ...Application, ...Config } => {
	{
		...app
		config = loadConfiguration()
	}
}

-- Database plugin
function addDatabase(Application: { ...App, ...Config }) = (app: Application): { ...Application, ...Database } => {
	{
		...app
		database = connectToDatabase(app.config.databaseUrl)
	}
}

-- Router plugin
function addRouter(Application: { ...App, ...Config }) = (app: Application): { ...Application, ...Router } => {
	{
		...app
		router = createRouter(app.config.routes)
	}
}

-- Compose the application with its plugins
function createApp = setupConfig then addDatabase then addRouter

-- Initialize the application
let app = createApp(App { name = "MyApp", version = "1.0.0" })
-- app now has the `config`, `database` and `router` properties
```

Plugins can be added, removed or reordered while the whole chain stays type-safe.

### Data migrations

Composition can describe the successive versions of a data structure:

```fa
type DataV1 = { foo: String }
type DataV2 = { ...DataV1, bar: String }
type DataV3 = { ...Omit(DataV2, { bar }), baz: String }

function migrateToV1 = () => DataV1 { foo = "foo" }

function migrateToV2 = (v1: DataV1) => DataV2 {
	...v1
	bar = "bar"
}

function migrateToV3 = (v2: DataV2) => DataV3 {
	foo = v2.foo
	baz = "baz"
}

function migrate = migrateToV1 then migrateToV2 then migrateToV3

-- Upgrade a v1 object to the latest version
function migrateFromV1 = migrate[1..] -- (v1: DataV1) => DataV3
```

### Data pipelines

Create data processing pipelines by composing transformation functions:

```fa
function filterEven = (numbers: Array(Integer)) => numbers.filter((n) => n %% 2 == 0)
function multiplyByTwo = (numbers: Array(Integer)) => numbers.map((n) => n * 2)
function sum = (numbers: Array(Integer)) => numbers.reduce((total, n) => total + n, 0)

function processNumbers = filterEven then multiplyByTwo then sum

let result = processNumbers([1, 2, 3, 4, 5, 6]) -- [2, 4, 6] -> [4, 8, 12] -> 24
```

### Composing specialized functions

Composition works with any function known at compile time, including functions created by a factory or by a [partial application](/functions/defining-functions#function-expressions):

```fa
function makeGreeter = (greeting: String) => (name: String) => "{greeting}, {name}!"

function helloGreeter = makeGreeter("Hello")
function hiGreeter = makeGreeter("Hi")

function formatBold = (text: String): String => "**{text}**"

function formalGreeting = helloGreeter then formatBold
function casualGreeting = hiGreeter then formatBold

formalGreeting("John") -- "**Hello, John!**"
casualGreeting("Jane") -- "**Hi, Jane!**"
```
