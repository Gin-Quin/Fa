import npeg

grammar Controls:
  stop <- !1
  space <- +Blank
  blank <- *Blank
  between(start, stop) <- start * >*(1 - stop) * stop
  endOfLine <- *Blank * ('\n' | !1)

grammar Keywords:
  yes <- "yes"
  no <- "no"
  true <- "true"
  false <- "false"
  null <- "null"
  trueish <- true | yes
  falsish <- false | no
  boolean <- trueish | falsish

grammar Numerics:
  integer <- +Digit * ?unsignedExponentSuffix
  number <- +Digit * ?decimalSuffix * ?(signedExponentSuffix * ?decimalSuffix)
  
  decimalSuffix <- "." * *Digit
  unsignedExponentSuffix <- 'e' * +Digit
  signedExponentSuffix <- 'e' * ?'-' * +Digit

grammar Others:
  identifier <- +Alpha
