# flags are graphql-like syntax brought into the fa language

flags Actions = # let's define all possible values
	run
	walk
	talk

flags Actions = { run, walk, talk }

# it looks like enums but an enum expects on single value where flags always expect multiple values
let action: Action = { run, walk }

# it is transpiled into an record of (true | undefined)
# JS
const action: { run?: true, walk?: true, talk?: true } = { run: true, walk: true }

# flags can be deep and include other flags
flags AuthorKeys = 
	id
	firstName
	lastName
	books
		title
		pagesCount

# flags are very useful to select specific properties of an object
let findAuthors = (select: AuthorKeys) -> ...

findAuthors:
	select = 
		id
		books # should there be a ':' here?
			title

findAuthors(select = { id, books { title } })
findAuthors(select = { id, books }) # in opposition to GraphQL, it is possible to select "all subflags"

# the special keyword "all" enable to select all flags
let action: Actions = all

# thanks to parameters inheritance, it is possible to pass flags as solo arguments
let select(...AuthorKeys) -> ...

select(id, books { title })

select:
	id
	books
		title


# it possible to get the type of the keys of an object thanks to the built-in type "Keys"
type AuthorKeys = Keys<Author>

# do not mix it up with "Key" that returns a string key of a type
type AuthorKey = Key<Author>

# it is possible to extend flags by other flags
flags UserKeys =
	id
	email
	name

flags AuthorKey =
	...UserKeys
	books
		id
		title
		