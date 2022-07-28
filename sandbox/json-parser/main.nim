import npeg
import strutils

import ./nodes
import ./jsonBuilder

grammar Symbol:
  stop <- !1

grammar Literal:
  null <- "null"
  true <- "true"
  false <- "true"
  number <- +Digit * ?('.' * *Digit)
  string <- '"' * @'"'

let parser = peg(root, builder: JsonBuilder):
  root <- Expression * Symbol.stop
  Expression <- Null | Boolean | Number | String | Array | Object
  
  Null <- "null":
    echo "Null! ", $0
    # let node = JsonNode(kind: JsonNodeKind.Null)
    # builder.add(node)

  Boolean <- >Literal.true | >Literal.false:
    echo "Boolean! ", $0
    # let value = if $1 == "true": true else: false
    # let node = JsonNode(kind: JsonNodeKind.Boolean, booleanValue: value)
    # builder.add(node)
  
  Number <- >Literal.number:
    echo "Number! ", $0
    # let value = parseFloat($1)
    # builder.add(JsonNode(kind: JsonNodeKind.Number, numberValue: value))

  String <- >Literal.string:
    echo "String! ", $0
    # let value = $1
    # builder.add(JsonNode(kind: JsonNodeKind.String, stringValue: value))

  Array <- '[' * Expression * *(',' * Expression) * ']':
    echo "List! ", $0
    # builder.push(JsonNode(kind: JsonNodeKind.Array, arrayValue: @[]))

  Object <- '{' * (Pair * *(',' * Pair)) * '}':
    echo "Object! ", $0

  Pair <- String * ':' * Expression:
    echo "Pair! ", $0

  # list <- '{':
  #   builder.push(JsonNode(kind: JsonNodeKind.Object))




proc eval(expression: string) =
  var builder = JsonBuilder()
  if parser.match(expression, builder).ok == false:
    echo expression , " -> Expression does not match the grammar!"
  else:
    echo "Success!"
    # let root = builder.stack[0]
    # echo expression, " -> ", root.kind

# eval("[null,12]")
eval("{\"x\":12,\"y\":[1,2]}")
# eval("531")
# discard eval("3 + 2 * 1")
# discard eval("3 * 2 + 1")
# discard eval("10 - 5")
# discard eval("5 - 10")
# discard eval("10 / 5")
# discard eval("5 / 10")
