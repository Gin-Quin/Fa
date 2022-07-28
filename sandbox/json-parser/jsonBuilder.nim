import ./nodes
import std/tables

type Zabu* = object
  stack*: seq[JsonNode]
  key*: string

type JsonBuilder* = object
  stack*: seq[JsonNode]
  key*: string

proc add*(builder: var JsonBuilder, value: JsonNode) =
  if builder.stack.len == 0:
    builder.stack.add(value)
    return

  let parent = builder.stack[builder.stack.len - 1]

  case parent.kind
  of JsonNodeKind.Object:
    if builder.key == "":
      raise newException(ValueError, "A key must be set before adding a value to an object")
    else:
      parent.objectValue[builder.key] = value

  of JsonNodeKind.Array:
    parent.arrayValue.add(value)

  else: raise newException(ValueError, "A primitive cannot contains values")

proc setKey*(builder: var JsonBuilder, key: string) =
  builder.key = key

proc push*(builder: var JsonBuilder, node: JsonNode) =
  builder.stack.add(node)

proc pop*(builder: var JsonBuilder) =
  discard builder.stack.pop()


var b = JsonBuilder()
let node = JsonNode(kind: JsonNodeKind.String, stringValue: "hello")
b.add(node)
