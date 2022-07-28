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
