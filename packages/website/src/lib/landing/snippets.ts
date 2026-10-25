/**
 * The Fa code shown on the landing page.
 * Every snippet must parse: run `bun run check:snippets` after changing one.
 */

export const heroFiles = [
	{
		name: "users.fa",
		label: "Typed errors",
		code: `type UserError = Errors {
	NotFound = { id: String }
	Banned = { reason: String }
}

function getUser = (id: String): User throws DbError | UserError requires db: Database => {
	let user = try db.findUser(id)

	if user is none {
		throw UserError.NotFound { id = id }
	}

	user
}`,
		diagnostic: {
			kind: "error" as const,
			message: "`getUser` can fail with `DbError | UserError`",
			location: "main.fa:4:13",
			help: "use `try` to propagate the error, or `catch` to handle it",
		},
	},
	{
		name: "people.fa",
		label: "Relations",
		code: `mutable people: Bag(Person) = []

type Person = {
	name: String
	bestFriend: (Person in people)?
	bestFriendOf: Bag(people->bestFriend)
}

use ana = people.add({ name = "Ana" })
use bao = people.add({ name = "Bao" })

ana.bestFriend = bao -- \`bao.bestFriendOf\` now contains Ana
people.delete(bao) -- \`ana.bestFriend\` becomes \`none\``,
		diagnostic: {
			kind: "success" as const,
			message: "both sides of `bestFriend` are kept in sync",
			location: "people.fa",
			help: "no weak pointer, no dangling reference, no garbage collector",
		},
	},
	{
		name: "counter.fa",
		label: "Reactivity",
		code: `reactive count = 0
derived doubled = count * 2

watch {
	console.log("{count} × 2 = {doubled}")
}

count += 1 -- logs "1 × 2 = 2"`,
		diagnostic: {
			kind: "success" as const,
			message: "`watch` runs again when `count` or `doubled` changes",
			location: "counter.fa",
			help: "reactivity is part of the language: no framework, no hooks",
		},
	},
	{
		name: "tasks.fa",
		label: "Concurrency",
		code: `async function loadDashboard = (user: User): Dashboard throws HttpError => {
	let orders = start fetchOrders(user.id)
	let messages = start fetchMessages(user.id)

	Dashboard {
		orders = try await orders
		messages = try await messages
	}
}`,
		diagnostic: {
			kind: "success" as const,
			message: "both requests run at the same time",
			location: "tasks.fa",
			help: "compiles to the event loop on the web, and to Tokio natively",
		},
	},
];

export const signature = `function checkout = (cart: Cart): Receipt throws PaymentError requires payments: Payments mutates cart => {
	let receipt = try payments.charge(cart.total)
	cart.paid = true
	receipt
}`;

/** The parts of the signature above, and what each one means at the call site. */
export const signatureParts = [
	{
		kind: "returns",
		label: "Returns",
		source: ": Receipt",
		title: "What it gives back",
		text: "The return type. Inferred when the body is a single expression, declared when it is a block, so a signature never hides anything.",
		call: `let receipt = try checkout(cart) -- type: Receipt`,
	},
	{
		kind: "throws",
		label: "Throws",
		source: "throws PaymentError",
		title: "How it can fail",
		text: "Every error is a typed value. The caller propagates it with try, or handles it with catch: forgetting both does not compile.",
		call: `checkout(cart) catch error => notify(error)`,
	},
	{
		kind: "requires",
		label: "Requires",
		source: "requires payments: Payments",
		title: "What it needs to run",
		text: "Services and context, matched by type and provided by a caller up the chain. A missing requirement is a compile error, never a runtime one.",
		call: `provide Payments = stripePayments(apiKey)

let receipt = try checkout(cart)`,
	},
	{
		kind: "mutates",
		label: "Mutates",
		source: "mutates cart",
		title: "What it changes",
		text: "Mutations are part of the contract. Only a mutable value can be passed, and the compiler knows which references stay valid after the call.",
		call: `mutable cart = Cart { total = 42, paid = false }

let receipt = try checkout(cart)
assert cart.paid`,
	},
] as const;

export const features = {
	errors: `let user = try getUser(id)
let name = loadName() catch error => "Anonymous"`,
	requirements: `provide Database = postgres(url)

let user = try getUser("42")`,
	words: `if user is none or user.age < 18 {
	return
}`,
	reactivity: `reactive count = 0
derived doubled = count * 2`,
	parallel: `for parallel image, index in images {
	thumbnails[index] = image.resize(256)
}`,
	compileTime: `@if target is "web" {
	console.log("Hello from the browser")
}`,
	tests: `test "clamps to the range" {
	assert clamp(12, 0, 10) == 10
}`,
	integers: `let age: Integer(min = 0) = 42
let days = age * 365 -- range tracked`,
};

export const memory = {
	invalidation: `mutable persons: Array(Person) = [ana, bao]

use someone = persons[1] -- type: \`Person in persons\`
persons.push({ name = "Cleo" }) -- may move every element

console.log(someone.name)`,
	invalidationErrorLine: 6,
	precise: `use first = persons[0]
persons.pop() -- invalidates the last element only

console.log(first.name) -- fine`,
};

export const agentLoop = {
	before: `function main = () => {
	greet()
}`,
	after: `function main = () => {
	provide Session = { user = currentUser }
	provide Logger = consoleLogger

	greet()
}`,
};
