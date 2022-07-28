

# I. A module is a function and @export is a 'return' equivalent
# There can be only one @export statement in the whole file
# Like a function, this export is typed


# We want it possible to export :
# - single values like an integer,
# - multiple values with names,

export { zabu, coco, caca }

let x = { y: 12, z: 121 }
let call
let zabu() = add(5, 17)
let x = print(5, 17)

let node: Node.ClassNode
