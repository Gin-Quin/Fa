import npeg
import strutils
import std/tables

import ./nodes

grammar Symbol:
  stop <- !1

grammar Literal:
  null <- "null"
  true <- "true"
  false <- "false"
  number <- +Digit * ?('.' * *Digit)
  string <- '"' * @'"'

let parser = peg(Root, stack: seq[JsonNode]):
  Root <- Expression * Symbol.stop
  Expression <- Null | Boolean | Number | String | Array | Object
  
  Null <- "null":
    # echo "Null! ", $0
    stack.add(JsonNode(kind: JsonNodeKind.Null))
    # let node = 
    # builder.add(node)

  Boolean <- >Literal.true | >Literal.false:
    # echo "Boolean! ", $0
    let value = if $1 == "true": true else: false
    stack.add(JsonNode(kind: JsonNodeKind.Boolean, booleanValue: value))
  
  Number <- >Literal.number:
    # echo "Number! ", $0
    let value = parseFloat($1)
    stack.add(JsonNode(kind: JsonNodeKind.Number, numberValue: value))

  String <- >Literal.string:
    # echo "String! ", $0
    let value = $1
    stack.add(JsonNode(kind: JsonNodeKind.String, stringValue: value))

  Array <- '[' * >Expression * *(',' * >Expression) * ']':
    let size = capture.len - 1
    # echo "List[", size, "]! ", $0
    let node = JsonNode(kind: JsonNodeKind.Array, arrayValue: stack[stack.len - size .. stack.len - 1])
    for i in 1 ..< capture.len:
      discard stack.pop()
    stack.add(node)

  Object <- '{' * (>Literal.string * ':' * Expression * *(',' * >Literal.string * ':' * Expression)) * '}':
    # echo "Object[", capture.len - 1, "]! ", $0
    let node = JsonNode(kind: JsonNodeKind.Object, objectValue: initTable[string, JsonNode]())
    for i in 1 ..< capture.len:
      let key: string = capture[capture.len - i].s
      node.objectValue[key] = stack.pop()
    stack.add(node)

proc eval(expression: string) =
  var stack: seq[JsonNode] = @[]
  if parser.match(expression, stack).ok == false:
    echo expression , " -> Expression does not match the grammar!"
  else:
    echo "Success! "
    for node in stack:
      echo node.kind, ": ", node.toString()
    # let root = builder.stack[0]
    # echo expression, " -> ", root.kind

eval("[null,12]")
eval("{\"x\":12,\"y\":[1,2,3,4]}")
eval("234")
eval("true")
eval("[true,false,[false,true]]")
