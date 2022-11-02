There are three possible syntaxes I think of to delimitate blocks.

Fa will go with solution #1, which is: meaningful identation and line breaks.

Not all developpers will like it, but no language can please every developper.

40% of programmers use Python, so indentation is not a wrong design.

I should be careful though to encourage small files / small functions / small types.

## 1. Indentation

Like Python, Nim, Ruby, Elixir, Coffeescript, Pug.

```coffee
if x == 12
  print("This is cool")
else
  print("Other syntaxes are not as elegant")

const heroes =
  - name = "Heracles"
    strength = 121
  - name = "Ajax"
    strength = 119 

children =
  - Title:
    variant = "red"
  - Button:
    variant = "secondary"
```

**Pros:**
  - very elegant, no noise, human readable
  - very dense (not as much characters for the same result)
  - Python users will like it
  - no complicated keystrokes

**Cons:**
  - unconvenient for big files or big functions as the developer will start to "lose" himself (in what block am I again?). It can be helped with the IDE though (colored line indents, jump to start / end of bracket)
  - more difficult to parse (difficulty should not be an argument)

## 2. Brackets

The classic, used by C, C++, Javascript, Typescript, Swift, Go, Rust; with the exception that line breaks are meaningful.

```swift
if x == 12
  print("This is cool")
else
  print("Other syntaxes are not as elegant")
end

let heroes = [
  - name: "Heracles"
    strength: 121
  - name: "Ajax"
    strength: 119
]

let add(x, y): Number {
  ...
}

let add = (x, y): Number -> {
  ...
}

let add = (x, y): Number => x + y

children = [
  - Title {
    variant: "red"
  }
  - Button {
    variant: "secondary"
  }
]

on.click = (event) -> {

}
```

**Pros:**
  - most developpers already use that kind of syntax, so adoption will be easier
  - easy to parse by a machine
  - users can make indentation mistakes, formatters will fix them
  - auto-format helps to copy/paste blocks that don't have the same indentation level

**Cons:**
  - very noisy
  - the necessary keystrokes are not so easy to do on non-qwerty keyboards

## 3. Slashes

```swift
if x == 12
  print("This is cool")
else
  print("Other syntaxes are not as elegant")
/if

let heroes =
  - name: "Heracles"
    strength: 121
  - name: "Ajax"
    strength: 119
/heroes

let add(x, y): Number ->
  ...
/add

openPopup:
  width = 180
  height = 100
  title = "Hello :)"
/openPopup

children =
  - Title:
    variant = "red"
  /Title
  - Button:
    variant = "secondary"
  /Button
/children
```

**Pros:**
  - easy to parse by a machine
  - the end statement is meaningful, which means you know what block it refers to
  - users can make indentation mistakes, formatters will fix them
  - auto-format helps to copy/paste blocks that don't have the same indentation level
  - no complicated keystrokes

**Cons:**
  - users are not used to this syntax. Snippets are **necessary** or developping will be hell
  - it may get boilerplatey. Much more lines to achive the same as #1
  - when using a lot of small elements that create a block, it will become *very* boilerplatey
