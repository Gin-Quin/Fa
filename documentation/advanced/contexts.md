# Contexts

Contexts are a way to pass data down to functions without having to explicitly pass it as arguments.

To use it, define a function with a parameter named `context`:

```fa
type MyContext = {
	user: {
		name: string;
		email: string;
	};
}

function greet = (context: MyContext) => {
  console.log(`Hello, ${context.user.name}!`);
}

function ciao = (context: MyContext) => {
  console.log(`Ciao, ${context.user.name}!`);
}
```

It can be used as any function parameter:

```fa
function doTheGreetings = () => {
	greet(context = {
		user: {
			name: 'John Doe',
			email: 'john.doe@example.com',
		}
	})
	
	ciao(context = {
		user: {
			name: 'John Doe',
			email: 'john.doe@example.com',
		}
	})
}
```

But you can also set the context *before* calling the function, which then will be automatically passed to the function:

```fa
function doTheGreetings = () => {
	context = MyContext {
		user: {
			name: 'John Doe',
			email: 'john.doe@example.com',
		}
	}
	
	greet() -- no need to pass the context explicitly
	ciao()
}
```

:::caution
Setting a new context will override the previous one.
:::

## Deep contexts

Context flows through nested calls. If a function doesn't declare a `context` parameter but it calls (directly or indirectly) another function that does, the active context still applies.

```fa
function outer = () => {
	context = MyContext {
		user: {
			name: "John Doe",
			email: "john.doe@example.com",
		}
	}

	inner()
}

function inner = () => {
	greet() -- uses the current context
}
```

The compiler will analyze the call stack, so calling a function that needs a `context` parameter without having set it will result in a compile error.

```fa
function outer = () => {
	middle() -- error: missing context
}

function inner = () => {
	greet()
}
```

## Scoped contexts

If you want to set a context only for a specific block of code, you can create a new scope:

```fa
function doTheGreetings = () => {
	context = MyContext {
		user: {
			name: 'John Doe',
			email: 'john.doe@example.com',
		}
	}

	do {
		-- create a new context for the block
		context = MyContext {
			user: {
				name: 'Jane Smith',
				email: 'jane.smith@example.com',
			}
		}
		
		greet() -- uses the scoped context "Jane"
		ciao()
	}
	
	greet() -- uses the outer context "John"
	ciao()
}
```

:::note
Contexts should be used sparingly as it transmits information implicitly.
:::
