import npeg
import strutils

grammar Symbol:
  stop <- !1
  number <- +Digit * ?('.' * *Digit)

let parser = peg("result", stack: seq[float]):
  result <- *Blank * expression * *Blank * Symbol.stop
  
  number <- >Symbol.number:
    # echo "Found number: ", $1
    stack.add(parseFloat($1))
  
  expression <- number * *operation:
    # echo "expression! ", $0
    discard

  operation <- *Blank * (
    >{ '+', '-' } * *Blank * expression ^ 10 |
    >{ '*', '/' } * *Blank * expression ^ 20
  ):
    let operation = $1
    # echo "operation! ", operation
    case operation
    of "+": stack.add(stack.pop() + stack.pop())
    of "-": stack.add(- stack.pop() + stack.pop())
    of "*": stack.add(stack.pop() * stack.pop())
    of "/":
      let (left, right) = (stack.pop(), stack.pop())
      stack.add(right / left)


proc eval(expression: string): float =
  var stack: seq[float]
  doAssert parser.match(expression, stack).ok
  result = stack[0]
  echo expression & " = "  & $result
  return result

discard eval("1 + 2 * 3")
discard eval("3 + 2 * 1")
discard eval("3 * 2 + 1")
discard eval("10 - 5")
discard eval("5 - 10")
discard eval("10 / 5")
discard eval("5 / 10")
