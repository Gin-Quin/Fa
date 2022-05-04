# Fa will improve Typescript's typing in many ways


# First, there is no need to create a generic when you only want to work with the full resolved type of a parameter
# For example, this in TS:
function print<T extends Printable>(value: T): T {
	return value
}
# Would be written this way in Fa:
let print(value: Printable): Type<value> ->
	return value
# there is no "generic infering from parameter" in Fa. If there is a need to infer a type from a parameter, simply do: Type<parameter>


# --- Second, recursive types are way more easier
# Common:
type MediaSymbol = "width" | "height";
type MediaOperator = "<" | ">" | "<=" | ">=";
type MediaExpression = `${MediaSymbol} ${MediaOperator} ${number}`;
type MediaOperator = "and" | "or";

# Fa:
type MediaQuery = MediaExpression | "{MediaQuery} {MediaOperator} {MediaQuery}"

# Equivalent in TS:
type MediaComposedExpression<S, Left, Right> = Left extends MediaQuery<Left>
	? Right extends MediaQuery<Right>
		? S
		: never
	: never;

type MediaQuery<S> = S extends MediaExpression
	? S
	: S extends `${infer Left} ${MediaOperator} ${infer Right}`
	? MediaComposedExpression<S, Left, Right>
	: never;


# in TS, this would raise the error: "Type alias 'MediaQuery' circularly references itself."
# that is because TS tries to "resolve" a type before checking if a value matches the type
# Fa wouldn't resolve the type, only keep it as-is, and check later if a value matches the type


# --- Third, I'd like to resolve the impossibility in TS to pass as a type parameter a type that needs another type to be resolved

# Example:
type MyGeneric = <T1 is <T2 is String> => T3> => T3
type MyGeneric = <T1: <T2: String> => T3> => T3

# This is "type functions"


# With the type function syntax, we can write:
type MyGenericClass = <T: Number> =>
	value: T

# In TS, it would be written:
type MyGenericClass<T: Number> = {
	value: T
}

# It may seem a little hard to understand first but in the end it's very clear and logical
# Furthermore, the type functions are really powerful
# In TS the absence of this feature make "type injection" impossible

# We can even imagine type functions like this:
type MyTypeFunction = <T1, T2> ->
	if T1 is T2: return T1
	else: return T2
# ie bringing a type sublanguage with:
# - ternary operator
# - union operator
# - logical operators (and, or, not, ...)
# - intersection operator
# - if, else, else if
# - return

# TS equivalent would be:
type MyTypeFunction<T1, T2> = T1 extends T2 ? T1 : T2
# which is fine for now but when we add more conditions it will become unreadable