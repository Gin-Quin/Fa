import std/tables

type JsonNodeKind* = enum
  Null
  Boolean
  Number
  String
  Array
  Object

type JsonNode* = ref object
  case kind*: JsonNodeKind
  of Null:
    discard
  of Boolean:
    booleanValue*: bool
  of Number:
    numberValue*: float
  of String:
    stringValue*: string
  of Array:
    arrayValue*: seq[JsonNode]
  of Object:
    objectValue*: Table[string, JsonNode]

proc toString*(node: JsonNode): string =
  case node.kind
  of Null: return "null"
  of Boolean: return $(node.booleanValue)
  of Number: return $(node.numberValue)
  of String: return node.stringValue

  of Array:
    var stringified = "["
    for index, node in node.arrayValue:
      if index > 0: stringified.add(", ")
      stringified.add(node.toString())
    stringified.add("]")
    return stringified

  of Object:
    var stringified = "{"
    for key, node in node.objectValue:
      if stringified.len > 1: stringified.add(", ")
      stringified.add(key & ": " & node.toString())
    stringified.add("}")
    return stringified
