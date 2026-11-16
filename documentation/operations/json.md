# JSON

`Json`, from `std.json`, parses JSON into the expected Fa type and serializes values as JSON text.

```fa
use { Json } = std.json

let person: { name: String, age: Integer } = try Json.parse("{{\"name\":\"Ada\",\"age\":36}}")
let text = Json.stringify(person)
```

`Json.parse` accepts a `String` or [`Bytes`](/values/bytes). Invalid JSON, invalid UTF-8 bytes, or a value that does not fit the expected type throws `JsonError`. The byte form parses directly from the bytes without constructing a String for the whole document.

`Json.stringify` writes a lone surrogate as its `\uXXXX` escape, using lowercase hexadecimal digits. JSON text is therefore always well-formed. Parsing that text preserves every UTF-16 code unit: `Json.parse(Json.stringify(text))` gives back the original string. The same rule holds at compile time and on every target.

`Json.parse` accepts a lone-surrogate escape and produces that surrogate as a String code unit. An escaped high surrogate immediately followed by an escaped low surrogate becomes the pair.

`std.json` is currently unavailable on `native-freestanding` (AN004); float parsing and printing without libc are separate standard-library work.

:::implementation
## Implementation details

Native serialization bypasses surrogate inspection when the String flags rule it out; otherwise it escapes canonical or virtual cut halves without materializing a borrowed span. Parsing preserves surrogate units as WTF-8, normalizing adjacent halves after unescaping. Bytes input is validated as strict UTF-8 before parsing its tokens directly. JavaScript uses its well-formed JSON.stringify and a strict decoder for byte string tokens.
:::
