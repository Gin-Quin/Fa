# JSON

`Json`, from `std.json`, parses JSON into the expected Fa type and serializes values as JSON text.

```fa
use { Json } = std.json

let person: { name: String, age: Integer } = try Json.parse("{{\"name\":\"Ada\",\"age\":36}}")
let text = Json.stringify(person)
```

`Json.parse` accepts a `String` or [`Bytes`](/values/bytes). Invalid JSON, invalid UTF-8 bytes, or a value that does not fit the expected type throws `JsonError`. The byte form parses directly from the bytes without constructing a String for the whole document.

Syntax errors have Fa's messages on every target, independently of the JavaScript engine. The message starts with `JSON Parse error: `; invalid UTF-8 bytes report `invalid UTF-8 in JSON input`.
The same wording and error precedence apply at compile time. These errors do not
include a host parser's line, column or byte offset. JSON accepts up to 1000
nested containers (objects and arrays); a 1001st container fails with
`JSON Parse error: Exceeded the maximum depth`.

`Json.stringify` writes a lone surrogate as its `\uXXXX` escape, using lowercase hexadecimal digits. JSON text is therefore always well-formed. Parsing that text preserves every UTF-16 code unit: `Json.parse(Json.stringify(text))` gives back the original string. The same rule holds at compile time and on every target.

`Json.parse` accepts a lone-surrogate escape and produces that surrogate as a String code unit. An escaped high surrogate immediately followed by an escaped low surrogate becomes the pair.

`std.json` is currently unavailable on `native-freestanding` (AN004); float parsing and printing without libc are separate standard-library work.

:::implementation
## Implementation details

Native serialization bypasses surrogate inspection when the String flags rule it out; otherwise it escapes canonical or virtual cut halves without materializing a borrowed span. Parsing preserves surrogate units as WTF-8, normalizing adjacent halves after unescaping. Bytes input is validated as strict UTF-8 before parsing its tokens directly. JavaScript uses its well-formed JSON.stringify and a strict decoder for byte string tokens.

The interpreter reads containers with an explicit heap stack, using the native
parser's grammar, diagnostic wording, precedence and 1000-container limit.
UTF-8 validation precedes grammar checks on Bytes input.

JavaScript retains the engine's JSON parser for successful String input. On a syntax error, an iterative Fa scanner reproduces the native parser's error grammar and precedence, including unfinished strings taking precedence over invalid escapes inside them. Failed byte input may decode the whole document for this error scan; successful byte input only decodes string tokens. Engine exception wording and offsets do not enter `JsonError`. String inputs
long enough to exceed the depth limit are checked with the same iterative scanner
before the engine parse; shorter successful inputs incur no scanner pass. The byte
token parser checks its explicit container stack against the same limit.

The compile-time writer uses an explicit work stack for collections, objects and named unions, with the same declaration order, omitted optional fields, enumeration values and flag paths as runtime serialization. Recursive schema names are scoped to the current serialization subtree, including separate declarations with the same name. It escapes raw surrogate units before producing UTF-8 JSON and charges output storage and visits to the interpreter budgets. `Json.parse` dispatches an already evaluated `String | Bytes` input by its runtime member; the byte branch retains strict UTF-8 validation.
:::
