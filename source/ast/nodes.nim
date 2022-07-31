import ./nodes/kinds

export FaNodeKind

type FaNode* = ref object
  start*: int
  length*: int

  case kind*: FaNodeKind
    of Null:
      discard
    of BooleanLiteral:
      booleanValue*: bool
    of IntegerLiteral:
      integerValue*: int
    of NumberLiteral:
      numberValue*: float
    of StringLiteral:
      stringValue*: string
    of Identifier:
      name*: string
    
    of VariableDeclaration:
      variableIdentifier*: FaNode
      variableTypeExpression*: FaNode
      variableExpression*: FaNode
