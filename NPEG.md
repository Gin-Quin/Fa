Fa uses the [npeg](https://github.com/zevv/npeg) library to parse source code from a given grammar.

This library uses a syntax close to regular expressions that can be used directly into nim files.

This file is a quick reference about the npeg syntax. The NPeg syntax is similar to normal PEG notation, but some changes were made to allow the grammar to be properly parsed by the Nim compiler.

## peg() and patt()

- the `peg(grammarName: string, ...rules): Parser` function allows to define a grammar (potentially recursive) from a set of rules
- the `patt(rootRuleName: string, ...rules): Parser` function creates a parser from a single anonymous pattern

Both these functions return a `Parser` object that can be use to parse a string or a file content.

### Parsing a file or a string

```coffee
proc match(parser: Parser, value: string): MatchResult
proc matchFile(parser: Parser, filepath: string): MatchResult

type MatchResult = object
  ok: bool
  matchLen: int
  matchMax: int
```

#### Retrieving catpured strings

```coffee
proc captures(match: MatchResult): seq[string]
```

## Defining a grammar

```coffee
let parser = peg "identifier":
  lower <- { 'a'..'z' }
  identifier <- *lower

doAssert parser.match("lowercaseword").ok
```


## Syntax reference

### Atoms
```ruby
0                   # matches always and consumes nothing
1                   # matches any character
n                   # matches exactly *n* characters
'x'                 # matches literal character 'x'
"xyz"               # matches literal string "xyz"
i"xyz"              # same but case insensitive
{ 'x'..'y' }        # matches any character in between 'x' and 'y'
{ 'x', 'y', 'z' }   # matches any character in the set
```

### Operators
```ruby
A * B               # concatenation (A followed by B)
A | B               # A if possible, B otherwise
A - B               # A if B does not match
(A)                 # Grouping
!A                  # Matches evertything but A
&A                  # Matches A without consuming input
?A                  # Matches A zero or one times
*A                  # Matches A zero or more times
+A                  # Matches A one or more times
@A                  # Search for A
A[n]                # Matches A exactly *n* times
A[m..n]             # Matches A between *m* and *n* times
```

### Precedence operators
```ruby
A^N                 # A is left associative with precedence N
A^^N                # A is right associative with precedence N
```

### Capturing
```ruby
>A                  # Captures the string matching A
```

### Back references
```coffee
R("tag", A)         # Create a back reference named "tag" on A
R("tag")            # Matches the named back reference
```

### Error handling
```ruby
E"message"          # Raise an exception with the given message
```


## Built-in atoms


```ruby
Alnum  <- {'A'..'Z','a'..'z','0'..'9'}  # Alphanumeric characters
Alpha  <- {'A'..'Z','a'..'z'}           # Alphabetic characters
Blank  <- {' ','\t'}                    # Space and tab
Cntrl  <- {'\x00'..'\x1f','\x7f'}       # Control characters
Digit  <- {'0'..'9'}                    # Digits
Graph  <- {'\x21'..'\x7e'}              # Visible characters
Lower  <- {'a'..'z'}                    # Lowercase characters
Print  <- {'\x21'..'\x7e',' '}          # Visible characters and spaces
Space  <- {'\9'..'\13',' '}             # Whitespace characters
Upper  <- {'A'..'Z'}                    # Uppercase characters
Xdigit <- {'A'..'F','a'..'f','0'..'9'}  # Hexadecimal digits
```
