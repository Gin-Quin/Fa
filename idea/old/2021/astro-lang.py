# Good ideas from the language-in-progress Astro:

# Late initialization?
    # Initialization can be deferred to later.
    let hello
    hello = "こんにちは"

    # A subject cannot be used without assigning it a value first.
    let str
    print(str) # Invalid!

# An import syntax that is clear
    import my_module {
        foo,
        bar,
    }

# There is a command notation! For one argument only. Is it a good idea?
    # If a function call takes just one argument, the call parens can be ommitted given the argument is
    # not a list, tuple or a dictionary literal.
    # This restriction is in place to improve code legibility.
    print "Hello" # Strings

# Regex like in Fa
    print ||regex||

# Comprehensions!
    print (x | x in z)
    print { x: y | x in z }
    let list = [x | y in z]

    # the fa equivalent without comprehensions is actually as easy!
    # => I think comprehensions are unnecessary if object mapping is done
    for x in z: print x
    print z.map: x => x
    let list = z.map: => x

# Ranges exactly like in Fa
    let range = 1..2
    let range = 1..2..4

# Union and intersection types. Union types are interesting but I think intersection is sortof inhertiance / composition
	var identification: String | Int = 60
	var pegasus: Horse & Wing

    # Fa equivalent:    
	var pegasus: { ...Horse, ...Wing }

# If, while, etc all are expressions
    if let stock_code = get_stock_code("APPLE") {
        print("APPLE: ${stock_code}")
    }

# Destructuring use parenthesis (elegant I think; I like bot syntaxes)
    for (kind, number) in interesting_numbers {
        print "${kind}: ${number}"
    }
    for { kind, number } in interesting_numbers {
        print "${kind}: ${number}"
    }
    for kind, number in interesting_numbers {
        print "${kind}: ${number}"
    }

# Extended `for` loop with a `where` condiition.
    for i in array where i == 5 {
        return i
    }

    for i in array | if i == 5 {
        return i
    }

# Break-with breaks a loop with a value. Don't know how to do it yet in Fa.
    let name =
        for name in register where ||tony|| in name {
            break name
        } else {
            ""
        }

    let message =
        while file.has_next() where ||t+|| in file.next() {
            break "File contains tabs!"
        } else {
            "File is clean!"
        }

    # let message = files.each: file =>
    #     ||t+|| in file? "File contains tabs!"
    # >> "File is clean!"

# Love this `in`!
    if student.name in defaulter_list {
        print "${student.name} hasn't paid yet. Contact parents"
    }
    if "fave_gift" not in birthday_presents {
        print "Aaargh! Everyone hates me"
    }
    if ||dollar[s]?|| in sentence {
        sentence.replace(||dollar[s]?||, "pounds")
    }


