import strutils
import options

import ../ast/nodes
include ./grammars

let parser = peg(Statement, stack: seq[FaNode]):
  Statement <- VariableDeclaration

  Atom <- Literal | Identifier
  Expression <- Atom * (?Operation)

  TypeAtom <- Identifier
  TypeExpression <- TypeAtom
  
  # [--- Literals ---]
  Literal <- NullLiteral | BooleanLiteral | IntegerLiteral | NumberLiteral | StringLiteral

  NullLiteral <- "null":
    echo "NullLiteral: ", $0
    stack.add(FaNode(kind: FaNodeKind.Null))

  BooleanLiteral <- Keywords.boolean:
    echo "BooleanLiteral: ", $0
    let value = if $0 == "true" or $0 == "yes": true else: false
    stack.add(FaNode(kind: FaNodeKind.BooleanLiteral, booleanValue: value))

  IntegerLiteral <- Numerics.integer:
    echo "IntegerLiteral: ", $0
    let value = parseInt($0)
    stack.add(FaNode(kind: FaNodeKind.IntegerLiteral, integerValue: value))

  NumberLiteral <- Numerics.number:
    echo "NumberLiteral: ", $0
    let value = parseFloat($0)
    stack.add(FaNode(kind: FaNodeKind.NumberLiteral, numberValue: value))
  
  StringLiteral <- Controls.between('"', '"'):
    echo "StringLiteral: ", $0
    stack.add(FaNode(kind: FaNodeKind.StringLiteral, stringValue: $1))

  # [--- Identifiers ---]
  Identifier <- >Others.identifier:
    echo "Identifier: ", $0
    stack.add(FaNode(kind: FaNodeKind.Identifier, name: $0))

  # [--- Operations ---]
  Operation <- Controls.blank * (
    >{ '+', '-' } * Controls.blank * Expression ^ 10 |
    >{ '*', '/' } * Controls.blank * Expression ^ 20
  ):
    echo "[Operation]"


  # [--- Declarations ---]
  VariableDeclaration <- "let" * Controls.space * Identifier * >?TypeDeclaration * >?Assignment:
    echo "[VariableDeclaration]"
    echo "  ", $0
    let isTyped = ($1 != "")
    let isAssigned = ($2 != "")
    echo "  isTyped? ", $isTyped
    echo "  isAssigned? ", $isAssigned
    let valueExpression = if isAssigned: stack.pop() else: nil
    let typeExpression = if isTyped: stack.pop() else: nil
    let identifier = stack.pop()
    let node = FaNode(
      kind: FaNodeKind.VariableDeclaration,
      variableIdentifier: identifier,
      variableExpression: valueExpression,
      variableTypeExpression: typeExpression,
    )
    # echo "node.variableIdentifier.name: ", node.variableIdentifier.name
    # echo "node.variableTypeExpression.kind: ", node.variableTypeExpression.kind
    echo "node.variableExpression.kind: ", node.variableExpression.kind
    stack.add(node)
  
  # [--- Others ---]
  TypeDeclaration <- Controls.blank * ':' * Controls.blank * >TypeExpression  
  Assignment <- Controls.blank * '=' * Controls.blank * >Expression


proc parseFa*(expression: string): FaNode =
  var stack: seq[FaNode] = @[]
  echo "Success? ", parser.match(expression, stack).ok
  return stack[0]

