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


# Second, recursive types are way more easier
# Common:
type MediaSymbol = "width" | "height";
type MediaOperator = "<" | ">" | "<=" | ">=";
type MediaExpression = `${MediaSymbol} ${MediaOperator} ${number}`;
type MediaOperator = "and" | "or";

# TS:
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


# Equivalent in Fa:
type MediaQuery = MediaExpression | `{MediaQuery} {MediaOperator} {MediaQuery}`
# in TS, this would raise the error: "Type alias 'MediaQuery' circularly references itself."
# that is because TS tries to "resolve" a type before checking if a value matches the type
# Fa wouldn't resolve the type, only keep it as-is, and check later if a value matches the type
