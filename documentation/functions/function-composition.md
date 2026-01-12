# Function Composition

Function composition is a powerful technique in Fa that allows you to combine multiple functions into a single function. In Fa, function composition is done using the `||>` operator.

## Basic Composition

When you compose functions using the `||>` operator, the functions are applied from left to right, opposite to mathematical function composition.

```fa
-- Define some simple functions
function increment = (x: Number): Number => x + 1
function double = (x: Number): Number => x * 2
function square = (x: Number): Number => x * x

-- Compose functions
function incrementAndDouble = increment ||> double

-- Using the composed function
let result = incrementAndDouble(3)  -- Equivalent to double(increment(3))

-- extend the composition with another function
function incrementAndDoubleAndSquare = incrementAndDouble ||> square

let result = incrementAndDoubleAndSquare(3)  -- Equivalent to square(double(increment(3)))
```

## Type Safety

Function composition in Fa is type-safe. The output type of one function must match the input type of the next function in the composition chain.

```fa
-- Type-safe composition
function stringLength = (s: String): Number => s.length
function toString = (n: Number): String => n.toString()

-- Valid composition: Number ||> String ||> Number
function getDigitCount = toString ||> stringLength  -- Takes a Number, returns a Number

console.log(#typeToString(toString ||> stringLength)) -- (Number) => Number
console.log(#typeToString(stringLength ||> toString)) -- (String) => String
```

## Composition Range

Fa allows you to extract specific ranges from a composition using array-like indexing syntax. This is useful when you want to reuse parts of a larger composition.

```fa
-- Define some simple functions
function one = (x: Number): Number => x + 1
function two = (x: Number): Number => x * 2
function three = (x: Number): Number => x - 3
function four = (x: Number): Number => x / 4

-- Create a composition of all functions
function oneToFour = one ||> two ||> three ||> four

-- Extract ranges from the composition
function twoToFour = oneToFour[1..]    -- Equivalent to (two ||> three ||> four)
function twoToThree = oneToFour[1..2]  -- Equivalent to (two ||> three)
function oneToThree = oneToFour[..2]   -- Equivalent to (one ||> two ||> three)

-- You can also extract a single function from the composition
function justTwo = oneToFour[1]        -- Equivalent to (two)
```

This feature makes compositions more flexible and reusable, allowing you to build complex function pipelines and extract specific parts as needed.

## Practical Applications

### Middleware Pattern

Function composition is perfect for implementing middleware patterns, especially when each middleware function enriches a context object with new properties:

```fa
type Timestamp = { timestamp: Integer }
type Auth = { auth: String }
type Logged = { logged: Boolean }

-- Define middleware functions that transform and enrich a context
function addTimestamp = (context: Context): Type(context) & Timestamp {{
  ...context
  timestamp = getCurrentTime()
}}

function addAuthHeader = (context: Context): Type(context) & Auth {{
  ...context
  auth = getAuthToken()
}}

function logRequest = (context: Context & Timestamp): Type(context) & Logged {
  log(`Logging request at ${context.timestamp}`)
  return {
    ...context
    logged = true
  }
}

-- Compose middleware - each function in the chain expects the context enriched by previous functions
function requestMiddleware = addTimestamp ||> addAuthHeader ||> logRequest

-- The final type is inferred correctly through the composition chain
-- Type: (Context) => Context & { timestamp: Integer, auth: String, logged: Boolean }

-- Apply all middleware to a base context
function processRequest = (baseContext: Context) => {
  enrichedContext = requestMiddleware(baseContext)
  -- enrichedContext now has timestamp, auth, and logged properties
  console.log(enrichedContext.timestamp)
  console.log(enrichedContext.auth)
  console.log(enrichedContext.logged)
  return handleRequest(enrichedContext)
}
```

In this pattern, each middleware function adds to the context, and subsequent functions can rely on the presence of properties added by earlier functions. The type system ensures that the functions are composed in the correct order, as a function requiring a property must come after the function that adds that property.

### Plugin System

The same composition pattern can be applied to create plugin systems, where each plugin extends the functionality of an application:

```fa
type Config = { config: Record<String, Any> }
type Router = { router: RouterType }
type Database = { db: DatabaseConnection }

-- Core application setup
function setupConfig = (app: BaseApp): app & Config {
  return {
    ...app,
    config: loadConfiguration()
  }
}

-- Database plugin
function addDatabase = (app: BaseApp & Config): app & Database {
  return {
    ...app,
    db: connectToDatabase(app.config.dbUrl)
  }
}

-- Router plugin
function addRouter = (app: BaseApp & Config): app & Router {
  return {
    ...app,
    router: createRouter(app.config.routes)
  }
}

-- Compose the application with its plugins
let createApp = setupConfig ||> addDatabase ||> addRouter

-- Initialize the application
let app = createApp({ name: "MyApp", version: "1.0.0" })
-- app now has config, db, and router properties
```

This approach allows for a modular application design where plugins can be easily added, removed, or reordered as needed, while maintaining type safety throughout the composition chain.

### Data Migrations

Function composition can be used to define data migrations that transform objects:

```fa
type Data = Return(migrate)
type DataV1 = Return(migrateToV1)
type DataV2 = Return(migrateToV2)
type DataV3 = Return(migrateToV3)

function migrate = migrateToV1 ||> migrateToV2 ||> migrateToV3

function migrateToV1 = () => {{
	foo = "foo"
}}

function migrateToV2 = (v1: V1) => {{
	...v1
	bar = "bar"
}}
function migrateToV3 = (v2: V2) => {{
	...v2 - { bar }
	baz = "baz"
}}

-- If you want to upgrade a v1 object to a v3:
function migrateFromV1 = migrate[1..]
```

### Function Pipelines

Create data processing pipelines by composing transformation functions:

```fa
-- Data transformation functions
function filterEven = (numbers: [Number]) => numbers.filter(n => n % 2 == 0)
function multiplyByTwo = (numbers: [Number]) => numbers.map(n => n * 2)
function sum = (numbers: [Number]) => numbers.reduce((acc, n) => acc + n, 0)

-- Create a pipeline to process data
function processingPipeline = filterEven ||> multiplyByTwo ||> sum

-- Use the pipeline
let result = processingPipeline([1, 2, 3, 4, 5, 6])  -- 2+4+6 ||> 4+8+12 ||> 24
```

## Advanced Composition

You can create more complex compositions by combining the `||>` operator with other functional programming techniques:

```fa
-- Partial application to create specialized functions
function makeGreeter = (greeting: String) => (name: String) => `${greeting}, ${name}!`

-- Create specialized greeters
function helloGreeter = makeGreeter("Hello")
function hiGreeter = makeGreeter("Hi")

-- Function that formats a string
function formatBold = (s: String): String => `**${s}**`

-- Compose specialized greeters with formatting
function formalGreeting = helloGreeter ||> formatBold
function casualGreeting = hiGreeter ||> formatBold

-- Use the composed functions
formalGreeting("John")  -- "**Hello, John!**"
casualGreeting("Jane")  -- "**Hi, Jane!**"
```

Function composition with the `||>` operator is a cornerstone of functional programming in Fa, enabling clean, modular, and reusable code.
