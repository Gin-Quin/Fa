import strutils
import system/io
import std/terminal
import std/sequtils

import ../nodes

proc recursivePrint(node: FaNode, level = 0): void

proc print*(node: FaNode) =
  recursivePrint(node)
  stdout.write "\n"

proc printNewLine(level: int) =
  stdout.styledWrite "\n", styleDim, "•  ".repeat(level)

proc printKind(node: FaNode) =
  stdout.styledWrite styleBright, fgMagenta, styleItalic, $node.kind

proc printSubType(subtype: string) =
  stdout.styledWrite " ", styleBright, fgGreen, subtype

proc printChildren(children: seq[FaNode], level: int) =
  for index, child in children:
    printNewLine(level)
    if index == children.len - 1:
      stdout.styledWrite  "└─ "
    else:
      stdout.styledWrite "├─ "
    recursivePrint(child, level + 1)

proc recursivePrint(node: FaNode, level = 0) =
  if node == nil:
    stdout.styledWrite styleItalic, styleDim, "nil"
    return

  case node.kind
    # [--- Literals ---]
    of FaNodeKind.Null:
      stdout.styledWrite fgBlue, styleItalic, "null"
    of FaNodeKind.BooleanLiteral:
      if node.booleanValue: stdout.styledWrite fgMagenta, "true"
      else: stdout.styledWrite fgMagenta, "false"
    of FaNodeKind.IntegerLiteral:
      stdout.styledWrite fgBlue, $node.integerValue
    of FaNodeKind.NumberLiteral:
      stdout.styledWrite fgBlue, $node.numberValue
    of FaNodeKind.StringLiteral:
      stdout.styledWrite fgGreen, '"' & node.stringValue & '"'
    of FaNodeKind.Identifier:
      stdout.styledWrite styleUnderscore, fgCyan, node.name

    # [--- Operations ---]
    of FaNodeKind.Operation:
      printKind(node)
      printSubType('"' & node.operator & '"')
      printChildren(@[
        node.leftOperationNode,
        node.rightOperationNode,
      ], level)
    of FaNodeKind.RightOperation:
      printKind(node)
      printSubType('"' & node.rightOperator & '"')
      printChildren(@[node.leftNode], level)
    of FaNodeKind.LeftOperation:
      printKind(node)
      printSubType('"' & node.leftOperator & '"')
      printChildren(@[node.rightNode], level)
    of FaNodeKind.CallOperation:
      printKind(node)
      printChildren(concat(@[node.callableExpression], node.parameters), level)
    of FaNodeKind.Index:
      printKind(node)
      printChildren(@[node.indexableExpression, node.index], level)

    # [--- Declarations ---]
    of FaNodeKind.VariableDeclaration:
      printKind(node)
      printChildren(@[
        node.variableIdentifier,
        node.variableTypeExpression,
        node.variableExpression,
      ], level)

    # [--- Statements ---]
    of FaNodeKind.IfStatement:
      printKind(node)
      printChildren(concat(@[
        node.ifExpression,
      ], node.ifCodeBlock), level)

    # [--- Others ---]
