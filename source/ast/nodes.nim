import ./nodes/kinds

export FaNodeKind

type FaNode* = ref object
  start*: int
  length*: int

  case kind*: FaNodeKind
    # [--- Literals ---]
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
    
    # [--- Operations ---]
    of Operation:
      operator*: string
      leftOperationNode*: FaNode
      rightOperationNode*: FaNode

    of RightOperation:
      rightOperator*: string
      leftNode*: FaNode

    of LeftOperation:
      leftOperator*: string
      rightNode*: FaNode

    of CallOperation:
      callableExpression*: FaNode
      parameters*: seq[FaNode]

    of Index:
      indexableExpression*: FaNode
      index*: FaNode

    # [--- Declarations ---]
    of VariableDeclaration:
      variableIdentifier*: FaNode
      variableTypeExpression*: FaNode
      variableExpression*: FaNode

    # [--- Others ---]
