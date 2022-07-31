import strutils
import system/io
import std/terminal

import ../nodes

proc recursivePrint(node: FaNode, level = 0): void

proc print*(node: FaNode) =
  recursivePrint(node)
  stdout.write "\n"

proc printNewLine(level: int) =
  stdout.styledWrite '\n' & ' '.repeat(level * 2)

proc printKind(node: FaNode) =
  stdout.styledWrite styleBright, styleItalic, $node.kind

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
    stdout.styledWrite styleItalic, "undefined"
    return
  case node.kind
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
    
    of FaNodeKind.VariableDeclaration:
      printKind(node)
      printChildren(@[
        node.variableIdentifier,
        node.variableTypeExpression,
        node.variableExpression,
      ], level)
