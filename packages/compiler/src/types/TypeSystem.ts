export type Type = PrimitiveType | ObjectType | ArrayType | UnionType

export type TypeWithProperties = ObjectType | ArrayType

export interface ObjectType {
	type: "object"
	properties: Record<string, Type>
}

export interface ArrayType {
	type: "array"
	items: Type
	properties: Record<string, Type>
}

export interface PrimitiveType {
	type: "string" | "number" | "boolean" | "null" | "undefined" | "never"
}

export interface UnionType {
	type: "union"
	types: Type[]
}

export interface FunctionType {
	type: "function"
	parameters: Type[]
	returnType: Type
}

const type = {
	type: "object",
	properties: {
		a: { type: "number" },
	},
} as const

type A = TypeSystem.Infer<typeof type>

function greaterOf(a: Type, b: Type): Type {
	if (TypeSystem.is(a, b)) return a
	if (TypeSystem.is(b, a)) return b
	return { type: "never" }
}

export namespace TypeSystem {
	export type Infer<T extends Type> = T extends { type: "number" }
		? number
		: T extends { type: "string" }
		? string
		: T extends { type: "boolean" }
		? boolean
		: T extends { type: "null" }
		? null
		: T extends { type: "undefined" }
		? undefined
		: T extends { type: "never" }
		? never
		: T extends { type: "object" }
		? { [Key in keyof T["properties"]]: Infer<T["properties"][Key]> }
		: T extends { type: "array" }
		? Infer<T["items"]>[] & {
				[Key in keyof T["properties"]]: Infer<T["properties"][Key]>
		  }
		: never

	export function isPrimitive(type: Type): type is PrimitiveType {
		return (
			type.type === "string" ||
			type.type === "number" ||
			type.type === "boolean" ||
			type.type === "null" ||
			type.type === "undefined"
		)
	}

	export function isSameType<T extends Type>(a: T, b: Type): b is T {
		return a.type === b.type
	}

	/**
	 * Check if type `a` is included in type `b` (i.e. `b` is a superset of type `a`).
	 */
	export function is<T extends Type>(a: T, b: Type): boolean {
		if (a.type === "union") {
			if (b.type === "union") {
				return a.types.every(type => b.types.some(type2 => is(type, type2)))
			}
			return a.types.every(type => is(a, type))
		}

		if (b.type === "union") {
			return b.types.some(type => is(a, type))
		}

		if (a.type !== b.type) return false

		if (a.type === "array" && b.type === "array") {
			return (
				is(a.items, b.items) &&
				Object.keys(a.properties).every(key => is(a.properties[key], b.properties[key]))
			)
		}

		if (a.type === "object" && b.type === "object") {
			return Object.keys(a.properties).every(key =>
				is(a.properties[key], b.properties[key])
			)
		}

		return true
	}

	export function union(types: Type[]): UnionType {
		return {
			type: "union",
			types: types.flatMap(type => (type.type === "union" ? type.types : type)),
		}
	}

	export function extend(
		a: TypeWithProperties,
		b: TypeWithProperties
	): TypeWithProperties {
		// TODO: Check if can merge properties?

		if (a.type === "array") {
			if (b.type === "array") {
				if (a.items.type !== b.items.type) {
					throw new Error("Cannot extend array types with different item types")
				}
				return {
					type: "array",
					items: a.items,
					properties: { ...a.properties, ...b.properties },
				}
			}
		}

		if (b.type === "array") {
			return {
				type: "array",
				items: b.items,
				properties: { ...a.properties, ...b.properties },
			}
		}

		return {
			type: "object",
			properties: { ...a.properties, ...b.properties },
		}
	}
}
