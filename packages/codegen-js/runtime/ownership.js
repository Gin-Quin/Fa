// Shared ownership of foreign resources and other shared handles. Fa objects and
// collections retain their structural value semantics.

// @fa-helper faUniqueDrop
/** A unique handle transferred by checked generic code, never retained. */
export const faUniqueDrop = Symbol("fa.unique.drop");

// @fa-helper faSharedOwners
/** @type {WeakMap<object, { count:number, destroy:(value:any)=>void, resource:boolean, shared?:{retain:()=>void,release:()=>void} }>} */
export const faSharedOwners = new WeakMap();

// @fa-helper faOwnShared deps: faSharedOwners
/**
 * Creates the first owning handle. The finalizer releases the instance's
 * fields as well as the external resource, exactly once.
 * @template {object} T
 * @param {T} value
 * @param {(value: T) => void} destroy
 * @param {boolean} resource whether its fields can contain a unique Task
 * @returns {T}
 */
export function faOwnShared(value, destroy, resource = false) {
	faSharedOwners.set(value, { count: 1, destroy, resource });
	return value;
}

// @fa-helper faRetainShared deps: faSharedOwners
/**
 * Copies a handle, retaining the same instance and mutable state.
 * @template T
 * @param {T} value
 * @returns {T}
 */
export function faRetainShared(value) {
	if ((typeof value === "object" && value !== null) || typeof value === "function") {
		const owner = faSharedOwners.get(value);
		if (owner !== undefined) {
			owner.shared?.retain();
			owner.count += 1;
		}
	}
	return value;
}

// @fa-helper faReleaseShared deps: faSharedOwners
/**
 * Releases one owner. A cleared or already released handle needs no cleanup.
 * @param {object | undefined | null} value
 * @returns {void}
 */
export function faReleaseShared(value) {
	if (value === undefined) return;
	const owner = faSharedOwners.get(value);
	if (owner === undefined || owner.count === 0) return;
	owner.count -= 1;
	if (owner.shared !== undefined) {
		owner.shared.release();
		return;
	}
	if (owner.count === 0) {
		owner.destroy(value);
	}
}

// @fa-helper faDropOwned deps: faSharedOwners faReleaseShared faUniqueDrop
/**
 * Releases the owning handles in an erased value, including task results.
 * A shared instance releases its own fields only with its last handle.
 * @param {unknown} value
 * @returns {void}
 */
export function faDropOwned(value) {
	/** @type {unknown[]} */
	const pending = [value];
	while (pending.length > 0) {
		const item = /** @type {any} */ (pending.pop());
		if ((typeof item !== "object" || item === null) && typeof item !== "function") continue;
		if (typeof item[faUniqueDrop] === "function") {
			item[faUniqueDrop]();
		} else if (faSharedOwners.has(item)) {
			faReleaseShared(item);
		} else if (typeof item === "function") {
			// A plain function has no registered owning environment.
			continue;
		} else {
			const parts =
				typeof item[Symbol.iterator] === "function" ? Array.from(item) : Object.values(item);
			for (let index = parts.length - 1; index >= 0; index -= 1) {
				pending.push(parts[index]);
			}
		}
	}
}
