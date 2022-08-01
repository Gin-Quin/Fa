import strutils
import options

import ../ast/nodes
include ./grammars

let parser = peg(CodeBlock, stack: seq[FaNode]):

  CodeBlock <- *Statement
  Statement <- (VariableDeclaration | IfStatement | Expression) * Controls.endOfLine

  Atom <- Literal | Group | Identifier
  Expression <- (LeftOperation * RightExpression) | (Atom * RightExpression)
  RightExpression <- *(RightOperation | Index | Operation | CallOperation)

  TypeAtom <- Identifier
  TypeExpression <- TypeAtom

  Group <- ('(' * Controls.blank * Expression * Controls.blank * ')') ^ 0
  
  # [--- Literals ---]
  Literal <- NullLiteral | BooleanLiteral | IntegerLiteral | NumberLiteral | StringLiteral

  NullLiteral <- "null":
    stack.add(FaNode(kind: FaNodeKind.Null))

  BooleanLiteral <- Keywords.boolean:
    let value = if $0 == "true" or $0 == "yes": true else: false
    stack.add(FaNode(kind: FaNodeKind.BooleanLiteral, booleanValue: value))

  IntegerLiteral <- Numerics.integer:
    let value = parseInt($0)
    stack.add(FaNode(kind: FaNodeKind.IntegerLiteral, integerValue: value))

  NumberLiteral <- Numerics.number:
    let value = parseFloat($0)
    stack.add(FaNode(kind: FaNodeKind.NumberLiteral, numberValue: value))
  
  StringLiteral <- Controls.between('"', '"'):
    stack.add(FaNode(kind: FaNodeKind.StringLiteral, stringValue: $1))

  # [--- Identifiers ---]
  Identifier <- >Others.identifier:
    stack.add(FaNode(kind: FaNodeKind.Identifier, name: $0))

  # [--- Operations ---]
  # see https://www.tektutorialshub.com/typescript/operator-precedence-in-typescript/
  # for Typescript operator precedence
  Operation <- Controls.blank * (
    >("=" | "+=" | "-=" | "**=" | "*=" | "/=" | "//=" | "%=") * Controls.blank * Expression ^^ 6 |
    >("==" | "!=" | "is") * Controls.blank * Expression ^ 22 |
    >('<' | "<=" | '>' | ">=" | "in") * Controls.blank * Expression ^ 24 |
    >{ '+', '-' } * Controls.blank * Expression ^ 28 |
    >( '*' | '/' | "//" ) * Controls.blank * Expression ^ 30 |
    >( "**" ) * Controls.blank * Expression ^^ 32 |
    >( '.' ) * Controls.blank * Expression ^ 40
  ):
    let operator = $1
    let rightNode = stack.pop()
    let leftNode = stack.pop()
    stack.add(FaNode(
      kind: FaNodeKind.Operation,
      operator: operator,
      leftOperationNode: leftNode,
      rightOperationNode: rightNode
    ))

  LeftOperation <- *( >"++" | >"--" | >"!" | >"-" | (>"run" * ' '))  * Controls.blank * Expression ^ 34 * Controls.blank:
    for i in 1 ..< capture.len:
      let operator = capture[^i].s
      let rightNode = stack.pop()
      stack.add(FaNode(
        kind: FaNodeKind.LeftOperation,
        leftOperator: operator,
        rightNode: rightNode
      ))
  
  RightOperation <- Controls.blank * >( "++" | "--" ) ^ 36 * Controls.blank:
    let operator = $1
    let leftNode = stack.pop()
    stack.add(FaNode(
      kind: FaNodeKind.RightOperation,
      rightOperator: operator,
      leftNode: leftNode,
    ))

  CallOperation <- Controls.blank * ('(' * Controls.blank * (>Expression * *(Controls.blank * ',' * Controls.blank * >Expression)) * Controls.blank * ')') ^ 40:
    var parameters = newSeq[FaNode](capture.len - 1)
    for i in 1 ..< capture.len:
      parameters[^i] = stack.pop()
    let callableExpression = stack.pop()
    stack.add(FaNode(
      kind: FaNodeKind.CallOperation,
      callableExpression: callableExpression,
      parameters: parameters
    ))
  
  Index <- Controls.blank * ('[' * Controls.blank * Expression * Controls.blank * ']') ^ 40:
    let index = stack.pop()
    let indexableExpression = stack.pop()
    stack.add(FaNode(
      kind: FaNodeKind.Index,
      indexableExpression: indexableExpression,
      index: index
    ))

  # [--- Declarations ---]
  VariableDeclaration <- "let " * Controls.blank * Identifier * >?TypeDeclaration * >?Assignment:
    let isTyped = ($1 != "")
    let isAssigned = ($2 != "")
    let valueExpression = if isAssigned: stack.pop() else: nil
    let typeExpression = if isTyped: stack.pop() else: nil
    let identifier = stack.pop()
    let node = FaNode(
      kind: FaNodeKind.VariableDeclaration,
      variableIdentifier: identifier,
      variableExpression: valueExpression,
      variableTypeExpression: typeExpression,
    )
    stack.add(node)

  # [--- Statements ---]
  IfStatement <- R("level", *'\t') * "if " * Controls.blank * Expression * +('\n' * &R("level") * &'\t' * >Statement):
    echo "IfStatement: ", $0
    var codeBlock = newSeq[FaNode](capture.len - 1)
    for i in 1 ..< capture.len:
      codeBlock[^i] = stack.pop()
    let ifExpression = stack.pop()
    stack.add(FaNode(
      kind: FaNodeKind.IfStatement,
      ifExpression: ifExpression,
      ifCodeBlock: codeBlock
    ))

  # [--- Others ---]
  TypeDeclaration <- Controls.blank * ':' * Controls.blank * >TypeExpression  
  Assignment <- Controls.blank * '=' * Controls.blank * >Expression


proc parseFa*(expression: string): seq[FaNode] =
  var stack: seq[FaNode] = @[]
  let match = parser.match(expression, stack)
  if match.ok:
    echo "🎉 Parsing successful"
  else:
    echo "🤕 Error while parsing"
  if stack.len == 0:
    echo "⛔️ Empty stack!"
  return stack

