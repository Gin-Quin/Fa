# Function Composition

Function composition is a powerful technique in Fa that allows you to combine multiple functions into a single function. In Fa, function composition is done using the `*` operator.

## Basic Composition

When you compose functions using the `*` operator, the functions are applied from left to right, opposite to mathematical function composition.

```ts
// Define some simple functions
increment = (x: Number): Number => x + 1
double = (x: Number): Number => x * 2
square = (x: Number): Number => x * x

// Compose functions
composed = increment * double

// Using the composed function
result = composed(3)  // Equivalent to double(increment(3))

// Programmatically add a function to the composition
composed.add(square)

result = composed(3)  // Equivalent to square(double(increment(3)))
```

## Empty Composition

You can create an empty composition and add functions to it later. This is useful when you need to build a composition dynamically:

```ts
// Create an empty composition with specified input and output types
myComposition = Composition(Number)

// Add functions to the empty composition
myComposition.add(n => n + 1)
myComposition.add(n => n * 2)

// Now the composition contains two functions
result = myComposition(5)  // "8"
```

Empty compositions are particularly useful for building up processing pipelines conditionally or at runtime.

## Type Safety

Function composition in Fa is type-safe. The output type of one function must match the input type of the next function in the composition chain.

```ts
// Type-safe composition
stringLength = (s: String): Number => s.length
toString = (n: Number): String => n.toString()

// Valid composition: Number -> String -> Number
getDigitCount = toString * stringLength  // Takes a Number, returns a Number

console.log(#typeToString(toString * stringLength)) // (Number) => Number
console.log(#typeToString(stringLength * toString)) // (String) => String
```

## Composition Range

Fa allows you to extract specific ranges from a composition using array-like indexing syntax. This is useful when you want to reuse parts of a larger composition.

```ts
// Define some simple functions
one = (x: Number): Number => x + 1
two = (x: Number): Number => x * 2
three = (x: Number): Number => x - 3
four = (x: Number): Number => x / 4

// Create a composition of all functions
oneToFour = one * two * three * four

// Extract ranges from the composition
twoToFour = oneToFour[1..]    // Equivalent to two * three * four
twoToThree = oneToFour[1..2]  // Equivalent to two * three
oneToThree = oneToFour[..2]   // Equivalent to one * two * three

// You can also extract a single function from the composition
justTwo = oneToFour[1]        // Equivalent to two
```

This feature makes compositions more flexible and reusable, allowing you to build complex function pipelines and extract specific parts as needed.

## Practical Applications

### Middleware Pattern

Function composition is perfect for implementing middleware patterns, especially when each middleware function enriches a context object with new properties:

```ts
type Timestamp = { timestamp: Integer }
type Auth = { auth: String }
type Logged = { logged: Boolean }

// Define middleware functions that transform and enrich a context
addTimestamp = (context: Context): Type(context) & Timestamp => {
  return {
    ...context,
    timestamp: getCurrentTime()
  }
}

addAuthHeader = (context: Context): Type(context) & Auth => {
  return {
    ...context,
    auth: getAuthToken()
  }
}

logRequest = (context: Context & Timestamp): Type(context) & Logged => {
  log(`Logging request at ${context.timestamp}`)
  return {
    ...context,
    logged: true
  }
}

// Compose middleware - each function in the chain expects the context enriched by previous functions
requestMiddleware = addTimestamp * addAuthHeader * logRequest

// The final type is inferred correctly through the composition chain
// Type: (Context) => Context & { timestamp: Integer, auth: String, logged: Boolean }

// Apply all middleware to a base context
processRequest = (baseContext: Context) => {
  enrichedContext = requestMiddleware(baseContext)
  // enrichedContext now has timestamp, auth, and logged properties
  console.log(enrichedContext.timestamp)
  console.log(enrichedContext.auth)
  console.log(enrichedContext.logged)
  return handleRequest(enrichedContext)
}
```

In this pattern, each middleware function adds to the context, and subsequent functions can rely on the presence of properties added by earlier functions. The type system ensures that the functions are composed in the correct order, as a function requiring a property must come after the function that adds that property.

### Plugin System

The same composition pattern can be applied to create plugin systems, where each plugin extends the functionality of an application:

```ts
type Config = { config: Record<String, Any> }
type Router = { router: RouterType }
type Database = { db: DatabaseConnection }

// Core application setup
setupConfig = (app: BaseApp): app & Config => {
  return {
    ...app,
    config: loadConfiguration()
  }
}

// Database plugin
addDatabase = (app: BaseApp & Config): app & Database => {
  return {
    ...app,
    db: connectToDatabase(app.config.dbUrl)
  }
}

// Router plugin
addRouter = (app: BaseApp & Config): app & Router => {
  return {
    ...app,
    router: createRouter(app.config.routes)
  }
}

// Compose the application with its plugins
createApp = setupConfig * addDatabase * addRouter

// Initialize the application
app = createApp({ name: "MyApp", version: "1.0.0" })
// app now has config, db, and router properties
```

This approach allows for a modular application design where plugins can be easily added, removed, or reordered as needed, while maintaining type safety throughout the composition chain.

### Data Migrations

Function composition can be used to define data migrations that transform objects:

```ts
// Migration functions that transform data from one version to another
migrateV1toV2 = (data: DataV1): DataV2 => {
  // Transform DataV1 to DataV2
  return {
    ...data,
    newField: "default",
    renamedField: data.oldField
  }
}

migrateV2toV3 = (data: DataV2): DataV3 => {
  // Transform DataV2 to DataV3
  return {
    ...data,
    restructuredData: {
      nested: data.someField
    }
  }
}

// Compose migrations
migrateV1toV3 = migrateV1toV2 * migrateV2toV3

// Apply migration to old data
latestData = migrateV1toV3(oldestData)
```

It's also possible to chain the migrations and to infer the types:

```ts
migrate = () => ({
  foo: "foo",
}) * (v1) => ({
  ...v1,
  bar: "bar",
}) * (v2) => ({
  ...omit(v2, { bar }),
  baz: "baz",
})

latestData = migrate(oldestData)

dataV3FromV2 = migrate[1..](dataV2)

type DataV1 = Return(migrate[0])
type DataV2 = Return(migrate[1])
type DataV3 = Return(migrate[2])
type LatestData = Return(migrate)
```

### Function Pipelines

Create data processing pipelines by composing transformation functions:

```ts
// Data transformation functions
filterEven = (numbers: [Number]): [Number] => numbers.filter(n => n % 2 == 0)
multiplyByTwo = (numbers: [Number]): [Number] => numbers.map(n => n * 2)
sum = (numbers: [Number]): Number => numbers.reduce((acc, n) => acc + n, 0)

// Create a pipeline to process data
processingPipeline = filterEven * multiplyByTwo * sum

// Use the pipeline
result = processingPipeline([1, 2, 3, 4, 5, 6])  // 2+4+6 -> 4+8+12 -> 24
```

## Advanced Composition

You can create more complex compositions by combining the `*` operator with other functional programming techniques:

```ts
// Partial application to create specialized functions
makeGreeter = (greeting: String) => (name: String): String => `${greeting}, ${name}!`

// Create specialized greeters
helloGreeter = makeGreeter("Hello")
hiGreeter = makeGreeter("Hi")

// Function that formats a string
formatBold = (s: String): String => `**${s}**`

// Compose specialized greeters with formatting
formalGreeting = helloGreeter * formatBold
casualGreeting = hiGreeter * formatBold

// Use the composed functions
formalGreeting("John")  // "**Hello, John!**"
casualGreeting("Jane")  // "**Hi, Jane!**"
```

Function composition with the `*` operator is a cornerstone of functional programming in Fa, enabling clean, modular, and reusable code.
