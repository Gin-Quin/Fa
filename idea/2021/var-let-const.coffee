# As in JS:
# - 'let' declare a variable value
# - 'const' declare a constant value (but not necessarily a compile-time constant)
# - a '#' before an identifier defines a compile-time constant

let z = 1234
const coco = "Hello"
#foo = "12"

# Question: should values defined with a hash be used with the has like `print(#foo)` or can it be used without the hash: `print(foo)`?