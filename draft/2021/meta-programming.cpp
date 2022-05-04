// Meta programming is done through the # character

let add(x: Number, y: Number) => x + y

// runtime call
let z = add(2, 4) // will be left as-is during transpilation

// compile-time call
let z = #add(2, 4) // will transformed into "let z = 6"

// You can use # with constant values and pure functions that take constant values as 0
#aVeryConstantInteger = 12 // must be assigned immediatley and will only be assigned once

// other languages can be embedded and precompiled into fa by using language server protocols (complicated but awesome) and a custom transpiler
// notably Typescript should be a first-class target

// to make metaprogramming possible, I need to write a **Fa interpreter** (able to interpret the Fa standard library as well, like IO, etc)


// Some metaprogramming use cases:

// 1. Type reflection
const x = 12
print: #typeof(x) // will print { type: "NumberLiteral", value: 12 }

// 2. Reading content from file system to avoid code generation
const #readVersion = () => #readRelativeFile("../version.txt") // read relatively to current source file location (not cwd)
const version = #readVersion() // will be transpiled into `const version = "1.0.0"` for example

// you will also access to #fileName and #directoryName

// 3. Transform style into css at compile-time
#css = ''

#parseStyle = (style: string) ->
	const className = generateClassName()
	#css += ".{className} \{ {style} \}"
	return className

const myClassName = #parseStyle("color: red;")

// -> This is so powerful!!