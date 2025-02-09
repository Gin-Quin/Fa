use BaseParser

# fa comes with a bundler written in Javascript that is extensible.
# most programming languages (like C) only accept to import other files of the same language.
# frontend development brought files mix: you can have .js, .css, .png files importing each other, and that is quite awesome
# nim's peg extends the nim programming language by adding rules to create a PEG. That's actually awesome but it mixes up two different languages in the same file: standard nim and nim's peg. Things become cmoplicated when we have to parse and typecheck.
# fa tends to a "one file <=> one language" solution but allows to mix up files.

type Parser =
  ...BaseParser { parse }
  
  name = "Hello"
  words = Map<String, Integer>()
  
  rules =
    pairs =
      - pair
      - space
      - *(',' * pair) * !1
    word =
      - repeat(Alpha, 1..infinity)
    number =
      - repeat(Digit, 1..infinity)
    pair =
      onMatch(match) ->
        words[match[0]] = words[match[1]]
      - word
      - space
      - '='
      - space
      - number

# because we use the # before the variable name, the expression must be computable at compile-time
# here it works because we pass a constant string
#words = Parser().match("one=1,two=2,three=3,four=4")
print: #words
