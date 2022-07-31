type
  Node* = ref object of RootObj
    start*: int
    length*: int

type StringLiteral* = ref object of Node
  value*: string

type NumberLiteral* = ref object of Node
  value*: float

type BooleanLiteral* = ref object of Node
  value*: bool

type Add* = ref object of Node
  left*: Node
  right*: Node

type VariableDeclaration* = ref object of Node
  name*: string
  value*: Node

method `$`*(node: Node): string {.base.} =
  quit "stringify: to override"

method `$`*(node: StringLiteral): string =
  node.value

method `$`*(node: NumberLiteral): string =
  $(node.value)

method `$`*(node: BooleanLiteral): string =
  $(node.value)

method `$`*(node: Add): string =
  $(node.left) & " + " & $(node.right)
