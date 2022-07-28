import npeg

import json

proc hoge(x: int): JsonNode =
    if x mod 2 == 0:
        return %x
    return %"odd"


# import strutils

# grammar Symbol:
#   stop <- !1

# let parser = peg("result", stack: seq[float]):


# proc eval(expression: string): float =
#   var stack: seq[float]
#   doAssert parser.match(expression, stack).ok
#   result = stack[0]
#   echo expression & " = "  & $result
#   return result

# discard eval("1 + 2 * 3")
# discard eval("3 + 2 * 1")
# discard eval("3 * 2 + 1")
# discard eval("10 - 5")
# discard eval("5 - 10")
# discard eval("10 / 5")
# discard eval("5 / 10")
