import npeg, strutils

var total: float = 0

let parser = peg("result"):
  result <- space * expression * space * !1

  # computing
  expression <- >literal * space * ?(>operator * space * expression):
    let value = parseFloat($1)
    if capture.len <= 2: total += value
    if capture.len > 2:
      case $2
      of "+": total += value
      of "-": total -= value
      of "*": total *= value
      of "/": total /= value

  # operators
  operator <- '+' | '*' | '/' | '-'
  
  # literals
  literal <- number
  number <- +Digit * ?('.' * +Digit):
    echo "Found number: " & $0
  space <- *(' ' | '\t' | ' ' | '\n')

echo parser.match("2.5 + 4 * 3").ok

echo "Total: " & $total
