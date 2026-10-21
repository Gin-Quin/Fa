# Function Composition

Function composition combines several functions into a single one. In Fa, functions are composed with the `then` keyword.

## Basic composition

`first then second` creates a new function that calls `first`, then passes its result to `second`. Functions are applied from left to right, in the order you read them.

```fa
-- Define some simple functions
function increment = (x: Integer): Integer => x + 1
function double = (x: Integer): Integer => x * 2
function square = (x: Integer): Integer => x * x

-- Compose functions
function incrementAndDouble = increment then double

-- Use the composed function
let result = incrementAndDouble(3) -- equivalent to double(increment(3))

-- Extend the composition with another function
function incrementAndDoubleAndSquare = incrementAndDouble then square

let result2 = incrementAndDoubleAndSquare(3) -- equivalent to square(double(increment(3)))
```

## Type safety

Function composition is type-safe: the return type of each function must match the parameter type of the next one.

```fa
function stringLength = (s: String): Integer => s.length
function toString = (n: Integer): String => n.toString()

-- Valid composition: Integer -> String -> Integer
function getDigitCount = toString then stringLength

console.log(@typeToString(getDigitCount)) -- (Integer) => Integer
```

```fa
function invalid = stringLength then stringLength -- error: `stringLength` expects a `String`, but receives a `Integer`
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

## Practical applications

### Middlewares

Composition is a natural fit for middlewares, especially when each middleware enriches a context object with new properties:

```fa
type Timestamp = { timestamp: Integer }
type Auth = { auth: String }
type Logged = { logged: Boolean }

-- Each middleware returns its input enriched with new properties
function addTimestamp = (request: Request): { ...Type(request), ...Timestamp } => {{
	...request
	timestamp = getCurrentTime()
}}

function addAuthHeader = (request: Request): { ...Type(request), ...Auth } => {{
	...request
	auth = getAuthToken()
}}

function logRequest = (request: { ...Request, ...Timestamp }): { ...Type(request), ...Logged } => {
	console.log("Logging request at {request.timestamp}")
	{
		...request
		logged = true
	}
}

-- Each middleware can rely on the properties added by the previous ones
function requestMiddleware = addTimestamp then addAuthHeader then logRequest

-- The final type is inferred through the whole chain:
-- (Request) => { ...Request, ...Timestamp, ...Auth, ...Logged }

function processRequest = (request: Request): Response => {
	let enrichedRequest = requestMiddleware(request)
	console.log(enrichedRequest.timestamp)
	console.log(enrichedRequest.auth)
	console.log(enrichedRequest.logged)
	handleRequest(enrichedRequest)
}
```

The type system ensures that the middlewares are composed in the correct order: a middleware requiring a property must come after the middleware that adds it.

### Plugins

The same pattern can be used to build plugin systems, where each plugin extends the application:

```fa
type Config = { config: AppConfig }
type Router = { router: AppRouter }
type Database = { database: DatabaseConnection }

-- Core application setup
function setupConfig = (app: App): { ...Type(app), ...Config } => {{
	...app
	config = loadConfiguration()
}}

-- Database plugin
function addDatabase = (app: { ...App, ...Config }): { ...Type(app), ...Database } => {{
	...app
	database = connectToDatabase(app.config.databaseUrl)
}}

-- Router plugin
function addRouter = (app: { ...App, ...Config }): { ...Type(app), ...Router } => {{
	...app
	router = createRouter(app.config.routes)
}}

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
function migrateToV1 = () => {{
	foo = "foo"
}}

function migrateToV2 = (v1: DataV1) => {{
	...v1
	bar = "bar"
}}

function migrateToV3 = (v2: DataV2) => {{
	...v2 - { bar }
	baz = "baz"
}}

function migrate = migrateToV1 then migrateToV2 then migrateToV3

type DataV1 = Return(migrateToV1)
type DataV2 = Return(migrateToV2)
type DataV3 = Return(migrateToV3)
type Data = Return(migrate)

-- Upgrade a v1 object to the latest version
function migrateFromV1 = migrate[1..]
```

### Data pipelines

Create data processing pipelines by composing transformation functions:

```fa
function filterEven = (numbers: Array(Integer)) => numbers.filter((n) => n modulo 2 == 0)
function multiplyByTwo = (numbers: Array(Integer)) => numbers.map((n) => n * 2)
function sum = (numbers: Array(Integer)) => numbers.reduce((total, n) => total + n, 0)

function processNumbers = filterEven then multiplyByTwo then sum

let result = processNumbers([1, 2, 3, 4, 5, 6]) -- [2, 4, 6] -> [4, 8, 12] -> 24
```

### Composing specialized functions

Composition works with any function, including functions created by a factory:

```fa
function makeGreeter = (greeting: String) => (name: String) => "{greeting}, {name}!"

function helloGreeter = makeGreeter("Hello")
function hiGreeter = makeGreeter("Hi")

function formatBold = (s: String): String => "**{s}**"

function formalGreeting = helloGreeter then formatBold
function casualGreeting = hiGreeter then formatBold

formalGreeting("John") -- "**Hello, John!**"
casualGreeting("Jane") -- "**Hi, Jane!**"
```
