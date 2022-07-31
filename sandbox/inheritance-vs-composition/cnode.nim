type
  NodeKind* = enum
    StringLiteral
    NumberLiteral
    BooleanLiteral
    Add
    VariableDeclaration

  Node* = ref object
    start*: int
    length*: int

    case kind*: NodeKind
    of StringLiteral:
      stringValue*: string
    of NumberLiteral:
      numberValue*: float
    of BooleanLiteral:
      booleanValue*: bool
    of Add:
      left*: Node
      right*: Node
    of VariableDeclaration:
      name*: string
      initializer*: Node

proc `$`*(node: Node): string =
  case node.kind
  of StringLiteral:
     node.stringValue
  of NumberLiteral:
    $node.numberValue
  of BooleanLiteral:
    $node.booleanValue
  of Add:
    $node.left & " + " & $node.right
  of VariableDeclaration:
    node.name
  