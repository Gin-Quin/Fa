// Counted immutable Function environments. No Task or Resource retain.

// @fa-helper faSharedOwners
/** @type {WeakMap<object, { count: number, destroy: (value: any) => void }>} */
export const faSharedOwners = new WeakMap();

// @fa-helper faOwnShared deps: faSharedOwners
/**
 * Creates the first owning handle. The finalizer releases the instance's
 * fields as well as the external resource, exactly once.
 * @template {object} T
 * @param {T} value
 * @param {(value: T) => void} destroy
 * @returns {T}
 */
export function faOwnShared(value, destroy) {
	faSharedOwners.set(value, { count: 1, destroy });
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
			owner.count += 1;
		}
	}
	return value;
}

// @fa-helper faReleaseShared deps: faSharedOwners
/**
 * Releases one owner. A cleared or already released handle needs no cleanup.
 * @param {object | undefined} value
 * @returns {void}
 */
export function faReleaseShared(value) {
	if (value === undefined) return;
	const owner = faSharedOwners.get(value);
	if (owner === undefined || owner.count === 0) return;
	owner.count -= 1;
	if (owner.count === 0) {
		owner.destroy(value);
	}
}

// @fa-helper faDropFunctions deps: faReleaseShared
/** @param {unknown} value */
export function faDropFunctions(value) {
 const pending = [value];
 const seen = new WeakSet();
 while (pending.length) {
  const item = pending.pop();
  if (typeof item === "function") { faReleaseShared(item); continue; }
  if (typeof item !== "object" || item === null || seen.has(item)) continue;
  seen.add(item);
  const values = item instanceof Map ? Array.from(item.entries()).flat() : item instanceof Set ? Array.from(item) : Object.values(item);
  for (const value of values) pending.push(value);
 }
}
