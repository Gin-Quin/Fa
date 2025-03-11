use std::collections::HashMap;

pub enum Type {
	/* --------------------------------- Special -------------------------------- */
	Errored(String), // could not build the type
	InternalReference { // reference to another type from the same file
		name: String,
	},
	ExternalReference { // reference to another type from another file
		uri: String,
		name: String,
	},
	Any,

	/* -------------------------------- Literals -------------------------------- */
	Null,
	Never,
	IntegerLiteral(i64),
	FloatLiteral(f64),
	StringLiteral(String),
	True,
	False,

	/* ------------------------------- Primitives ------------------------------- */
	Boolean,
	String,
	Integer(u8), // `u8` is the number of bits (can be 8, 16, 32, for bigger numbers use `BigInteger` or Float(64))
	UnsignedInteger(u8),
	BigInteger,
	Float(u8), // `u8` is the number of bits (can be 32 or 64)
	UnsignedFloat(u8),

	/* -------------------------------- Functions ------------------------------- */
	Function {
		parameters: HashMap<String, Type>,
		result: Box<Type>,
	},

	/* -------------------------------- Generics -------------------------------- */
	TypeOfValue {
		name: String,
	},
	Generic { // aka generics ; but maybe the ast is already declarative enough, and generics should have no parameters
		parameters: HashMap<String, Type>,
		result: Box<Type>,
	},
	// Generic, // should it have no parameters? (and we ruse the AST)
	GenericParameterReference { // a reference to a parameter of a generic
		name: String,
	},
	// TypeFunction {}, // a function that returns a type and that needs to run some Javascript to be resolved

	/* ------------------------------- Collections ------------------------------ */
	Tuple {
		of: Vec<Type>,
	},
	Object {
		properties: HashMap<String, Type>,
	},
	Array {
		items: Box<Type>,
	},
	Set {
		items: Box<Type>,
	},
	Map {
		key: Box<Type>,
		value: Box<Type>,
	},
	Bag {
		items: Box<Type>,
	},
	Columnar {
		items: Box<Type>,
	},

	/* --------------------------------- Others --------------------------------- */
	Union {
		of: Vec<Type>,
	},
	Enumeration {
		of: Vec<String>,
	},
	Fields {
		of: Vec<Field>,
	},
}

pub struct Field {
	pub name: String,
	pub fields: Option<Vec<Field>>,
}
