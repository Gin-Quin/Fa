# Function composition

Fa has two related operators:

- `|>` pipes a **value** through functions, evaluating immediately.
- `||>` composes **functions** into a new function, calling nothing.

```fa
-- pipe a value: computed now
let result = [1, 2, 3, 4] |> filterEven |> sum

-- compose functions: builds a new function
function process = filterEven ||> sum
let result = process([1, 2, 3, 4])
```

Both apply left to right, which is the opposite of mathematical composition.

## Piping values

`x |> f` is exactly `f(x)`. It exists to let a pipeline read in the order it runs:

```fa
let slug = title
  |> trim
  |> toLowerCase
  |> replaceSpacesWithDashes
```

When the function takes more than one argument, the piped value becomes the first one:

```fa
let joined = names |> join(", ") -- join(names, ", ")
```

## Composing functions

`||>` builds a new function without calling anything:

```fa
function increment = (x: Int): Int => x + 1
function double = (x: Int): Int => x * 2
function square = (x: Int): Int => x * x

function incrementAndDouble = increment ||> double

let result = incrementAndDouble(3) -- double(increment(3)) == 8

-- extend an existing composition
function incrementAndDoubleAndSquare = incrementAndDouble ||> square
```

## Type safety

Composition is type-checked: the output of one function must be accepted by the next.

```fa
function stringLength = (s: String): Int => s.length
function toText = (n: Int): String => n.toString()

function digitCount = toText ||> stringLength

console.log(@typeToString(toText ||> stringLength)) -- "(Int) => Int"
console.log(@typeToString(stringLength ||> toText)) -- "(String) => String"
```

## Errors in a composition

If any step returns an error, the composition short-circuits: the remaining steps are skipped and the error becomes the result. The composed function's return type is the union of every step's error type.

```fa
function loadUser = (id: Int): User | DatabaseError => ...
function loadScore = (user: User): Int | ApiError => ...

-- (Int) => Int | DatabaseError | ApiError
function loadUserScore = loadUser ||> loadScore

let score = loadUserScore(42)!
```

A step whose parameter type already includes the error type receives it instead of being skipped, which is how you write a recovery step:

```fa
function withFallback = (value: Int | ApiError): Int => when value is {
  Int >> score => score
  Error => 0
}

function safeScore = loadUser ||> loadScore ||> withFallback -- (Int) => Int | DatabaseError
```

## Slicing a composition

A composition can be indexed like an array, to reuse part of a pipeline. Ranges follow the usual rule: `..` excludes the upper bound, `..=` includes it.

```fa
function one = (x: Int): Int => x + 1
function two = (x: Int): Int => x * 2
function three = (x: Int): Int => x - 3
function four = (x: Int): Int => x / 4

function oneToFour = one ||> two ||> three ||> four

function twoToFour  = oneToFour[1..]   -- two ||> three ||> four
function twoToThree = oneToFour[1..3]  -- two ||> three
function oneToThree = oneToFour[..3]   -- one ||> two ||> three
function justTwo    = oneToFour[1]     -- two
```

Slicing works only on a value whose composition chain is known at compile time. Once a composed function has been stored in a variable annotated with a plain function type, the chain is erased and slicing is no longer available:

```fa
let pipeline: (Int) => Int = oneToFour
let part = pipeline[1..] -- error: this function has no composition chain
```

## Patterns

### Middleware

Each step enriches a context object, and the type system enforces the ordering: a step requiring a field must come after the step that adds it.

```fa
type Timestamp = { timestamp: Int }
type Auth = { auth: String }
type Logged = { logged: Boolean }

function addTimestamp = (Ctx: Type(Object), context: Ctx): Ctx & Timestamp => {
  ...context
  timestamp = getCurrentTime()
}

function addAuthHeader = (Ctx: Type(Object), context: Ctx): Ctx & Auth => {
  ...context
  auth = getAuthToken()
}

function logRequest = (Ctx: Type(Timestamp), context: Ctx): Ctx & Logged => {
  console.log("Logging request at {context.timestamp}")
  return {
    ...context
    logged = true
  }
}

function requestMiddleware = addTimestamp ||> addAuthHeader ||> logRequest
-- (Context) => Context & { timestamp: Int, auth: String, logged: Boolean }
```

Reordering `logRequest` before `addTimestamp` is a compile error, because its constraint would not be satisfied.

### Plugins

The same shape builds a plugin system:

```fa
type Config = { config: Map(String, String) }
type Router = { router: RouterType }
type Database = { db: DatabaseConnection }

function setupConfig = (App: Type(Object), app: App): App & Config => {
  ...app
  config = loadConfiguration()
}

function addDatabase = (App: Type(Config), app: App): App & Database => {
  ...app
  db = connectToDatabase(app.config["dbUrl"])
}

function addRouter = (App: Type(Config), app: App): App & Router => {
  ...app
  router = createRouter(app.config["routes"])
}

function createApp = setupConfig ||> addDatabase ||> addRouter

let app = createApp({ name = "MyApp", version = "1.0.0" })
```

### Migrations

Composition slicing makes staged data migrations natural:

```fa
function migrateToV1 = (): DataV1 => {
  foo = "foo"
}

function migrateToV2 = (v1: DataV1): DataV2 => {
  ...v1
  bar = "bar"
}

function migrateToV3 = (v2: DataV2): DataV3 => {
  ...v2 - @{ bar }
  baz = "baz"
}

function migrate = migrateToV1 ||> migrateToV2 ||> migrateToV3

-- upgrade an existing v1 object to v3
function migrateFromV1 = migrate[1..]
```

### Pipelines

```fa
function filterEven = (numbers: Array(Int)): Array(Int) => numbers.filter(n => n %% 2 == 0)
function multiplyByTwo = (numbers: Array(Int)): Array(Int) => numbers.map(n => n * 2)
function sum = (numbers: Array(Int)): Int => numbers.reduce((acc, n) => acc + n, 0)

function processingPipeline = filterEven ||> multiplyByTwo ||> sum

let result = processingPipeline([1, 2, 3, 4, 5, 6]) -- 24
```

### Partial application

```fa
function makeGreeter = (greeting: String) => (name: String): String => "{greeting}, {name}!"

function helloGreeter = makeGreeter("Hello")
function formatBold = (s: String): String => "**{s}**"

function formalGreeting = helloGreeter ||> formatBold

formalGreeting("John") -- "**Hello, John!**"
```

## Choosing between `|>` and `||>`

Use `|>` when you have a value in hand and want to read the transformation top to bottom. Use `||>` when you want to *name* the pipeline and apply it repeatedly — or when you need composition slicing, which only `||>` provides.
