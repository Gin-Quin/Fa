import strutils
import options

import ../ast/nodes
include ./grammars
import ../readers/Reader
import ../readers/indentedContentReader

type ParserData* = ref object
  data*: ptr UncheckedArray[char]
  length*: int
  reader*: Reader
  nodes*: seq[FaNode]

let faParser* = peg(Statement, data: ParserData):
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
    data.nodes.add(FaNode(kind: FaNodeKind.Null))

  BooleanLiteral <- Keywords.boolean:
    let value = if $0 == "true" or $0 == "yes": true else: false
    data.nodes.add(FaNode(kind: FaNodeKind.BooleanLiteral, booleanValue: value))

  IntegerLiteral <- Numerics.integer:
    let value = parseInt($0)
    data.nodes.add(FaNode(kind: FaNodeKind.IntegerLiteral, integerValue: value))

  NumberLiteral <- Numerics.number:
    let value = parseFloat($0)
    data.nodes.add(FaNode(kind: FaNodeKind.NumberLiteral, numberValue: value))
  
  StringLiteral <- Controls.between('"', '"'):
    data.nodes.add(FaNode(kind: FaNodeKind.StringLiteral, stringValue: $1))

  # [--- Identifiers ---]
  Identifier <- >Others.identifier:
    data.nodes.add(FaNode(kind: FaNodeKind.Identifier, name: $0))

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
    let rightNode = data.nodes.pop()
    let leftNode = data.nodes.pop()
    data.nodes.add(FaNode(
      kind: FaNodeKind.Operation,
      operator: operator,
      leftOperationNode: leftNode,
      rightOperationNode: rightNode
    ))

  LeftOperation <- *( >"++" | >"--" | >"!" | >"-" | (>"run" * ' '))  * Controls.blank * Expression ^ 34 * Controls.blank:
    for i in 1 ..< capture.len:
      let operator = capture[^i].s
      let rightNode = data .nodes.pop()
      data .nodes.add(FaNode(
        kind: FaNodeKind.LeftOperation,
        leftOperator: operator,
        rightNode: rightNode
      ))
  
  RightOperation <- Controls.blank * >( "++" | "--" ) ^ 36 * Controls.blank:
    let operator = $1
    let leftNode = data.nodes.pop()
    data.nodes.add(FaNode(
      kind: FaNodeKind.RightOperation,
      rightOperator: operator,
      leftNode: leftNode,
    ))

  CallOperation <- Controls.blank * ('(' * Controls.blank * (>Expression * *(Controls.blank * ',' * Controls.blank * >Expression)) * Controls.blank * ')') ^ 40:
    var parameters = newSeq[FaNode](capture.len - 1)
    for i in 1 ..< capture.len:
      parameters[^i] = data .nodes.pop()
    let callableExpression = data.nodes.pop()
    data.nodes.add(FaNode(
      kind: FaNodeKind.CallOperation,
      callableExpression: callableExpression,
      parameters: parameters
    ))
  
  Index <- Controls.blank * ('[' * Controls.blank * Expression * Controls.blank * ']') ^ 40:
    let index = data.nodes.pop()
    let indexableExpression = data.nodes.pop()
    data.nodes.add(FaNode(
      kind: FaNodeKind.Index,
      indexableExpression: indexableExpression,
      index: index
    ))

  # [--- Declarations ---]
  VariableDeclaration <- "let " * Controls.blank * Identifier * >?TypeDeclaration * >?Assignment:
    let isTyped = ($1 != "")
    let isAssigned = ($2 != "")
    let valueExpression = if isAssigned: data.nodes.pop() else: nil
    let typeExpression = if isTyped: data.nodes.pop() else: nil
    let identifier = data.nodes.pop()
    let node = FaNode(
      kind: FaNodeKind.VariableDeclaration,
      variableIdentifier: identifier,
      variableExpression: valueExpression,
      variableTypeExpression: typeExpression,
    )
    data.nodes.add(node)

  # [--- Statements ---]
  IfStatement <- "if " * Controls.blank * Expression:
    echo "IfStatement: ", $0
    var codeBlock = newSeq[FaNode](capture.len - 1)
    for i in 1 ..< capture.len:
      codeBlock[^i] = data .nodes.pop()
    let ifExpression = data.nodes.pop()
    data.nodes.add(FaNode(
      kind: FaNodeKind.IfStatement,
      ifExpression: ifExpression,
      ifCodeBlock: codeBlock
    ))

  # # [--- Others ---]
  TypeDeclaration <- Controls.blank * ':' * Controls.blank * >TypeExpression  
  Assignment <- Controls.blank * '=' * Controls.blank * >Expression


proc parseFa*(expression: ptr UncheckedArray[char], length: int): ParserData =
  result.data = expression
  result.length = length
  result.reader = indentedContentReader(result.data, length)
  for line in result.reader(0):
    let match = faParser.match(expression.toOpenArray(line.start, line.stop), result)
    if match.ok == false:
      echo "🤕 Error while parsing line"
  if result.nodes.len == 0:
    echo "⛔️ Empty data.nodes!"
