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