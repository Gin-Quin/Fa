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
  _ <- *Blank
  Statement <- (VariableDeclaration | IfStatement | Expression) * Controls.endOfLine

  Atom <- Literal | Group | Identifier
  Expression <- (LeftOperation * RightExpression) | (Atom * RightExpression)
  RightExpression <- *(RightOperation | Index | Operation | CallOperation)

  Group <- ('(' * _ * Expression * _ * ')') ^ 0
  
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
  Operation <- _ * (
    >("=" | "+=" | "-=" | "**=" | "*=" | "/=" | "//=" | "%=") * _ * Expression ^^ 6 |
    >("==" | "!=" | "is") * _ * Expression ^ 22 |
    >('<' | "<=" | '>' | ">=" | "in") * _ * Expression ^ 24 |
    >{ '+', '-' } * _ * Expression ^ 28 |
    >( '*' | '/' | "//" ) * _ * Expression ^ 30 |
    >( "**" ) * _ * Expression ^^ 32 |
    >( '.' ) * _ * Expression ^ 40
  ):
    data.nodes.add(FaNode(
      kind: FaNodeKind.Operation,
      operator: $1,
      leftOperationNode: data.nodes.pop(),
      rightOperationNode: data.nodes.pop()
    ))

  LeftOperation <- *( >"++" | >"--" | >"!" | >"-" | (>"run" * ' '))  * _ * Expression ^ 34 * _:
    for i in 1 ..< capture.len:
      data.nodes.add(FaNode(
        kind: FaNodeKind.LeftOperation,
        leftOperator: capture[^i].s,
        rightNode: data.nodes.pop()
      ))
  
  RightOperation <- _ * >( "++" | "--" ) ^ 36 * _:
    data.nodes.add(FaNode(
      kind: FaNodeKind.RightOperation,
      rightOperator: $1,
      leftNode: data.nodes.pop(),
    ))

  CallOperation <- _ * ('(' * _ * ?((>Identifier * >Assignment) * *(_ * ',' * _ * (>Identifier * >Assignment))) * _ * ')') ^ 40:
    var parameters = newSeq[FaNode](capture.len - 1)
    for i in 1 ..< capture.len:
      parameters[^i] = data.nodes.pop()
    let callableExpression = data.nodes.pop()
    data.nodes.add(FaNode(
      kind: FaNodeKind.CallOperation,
      callableExpression: callableExpression,
      parameters: parameters
    ))
  
  Index <- _ * ('[' * _ * Expression * _ * ']') ^ 40:
    let index = data.nodes.pop()
    let indexableExpression = data.nodes.pop()
    data.nodes.add(FaNode(
      kind: FaNodeKind.Index,
      indexableExpression: indexableExpression,
      index: index
    ))

  # [--- Declarations ---]
  VariableDeclaration <- _ * Identifier * (>(TypeDeclaration * >?Assignment) | >Assignment):
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
  IfStatement <- "if " * _ * Expression:
    echo "IfStatement: ", $0
    var codeBlock = newSeq[FaNode](capture.len - 1)
    for i in 1 ..< capture.len:
      codeBlock[^i] = data.nodes.pop()
    let ifExpression = data.nodes.pop()
    data.nodes.add(FaNode(
      kind: FaNodeKind.IfStatement,
      ifExpression: ifExpression,
      ifCodeBlock: codeBlock
    ))

  # # [--- Others ---]
  TypeDeclaration <- _ * ':' * _ * >Expression  
  Assignment <- _ * '=' * _ * >Expression

  Property <- _ * Identifier * >Assignment
  PropertyDeclaration <- _ * Identifier * (>(TypeDeclaration * >?Assignment) | >Assignment):
    let isTyped = ($1 != "")
    let isAssigned = ($2 != "") or ($3 != "")
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
    



proc parseFa*(expression: ptr UncheckedArray[char], length: int): ParserData =
  result = ParserData(
    data: expression,
    length: length,
    reader: indentedContentReader(expression, length),
  )
  for line in result.reader(0):
    echo line
    let match = faParser.match(expression.toOpenArray(line.start, line.stop), result)
    if match.ok == false:
      echo "🤕 Error while parsing line"
  if result.nodes.len == 0:
    echo "⛔️ Empty data.nodes!"
