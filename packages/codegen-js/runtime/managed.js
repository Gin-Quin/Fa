// Runtime locations whose storage moves from local Fa values to shared records.

// @fa-helper faManagedValues
/** @type {WeakMap<object, { remote:boolean, mutate?:(operation:string,args:any[])=>any }>} */
export const faManagedValues = new WeakMap();

// @fa-helper faManagedStorage deps: faManagedValues
/** The record owns these fields; its setter releases the previous edges.
 * @param {unknown} value @returns {boolean}
 */
export function faManagedStorage(value) {
	return ((typeof value === "object" && value !== null) || typeof value === "function")
		&& faManagedValues.get(value)?.remote === true;
}

// @fa-helper faManagedMutate deps: faManagedValues
/** @param {object} value @param {string} operation @param {any[]} args @returns {any} */
export function faManagedMutate(value, operation, args) {
	const location = faManagedValues.get(value);
	if (location?.mutate === undefined) throw new Error("a shared Fa view has no mutation handler");
	return location.mutate(operation, args);
}
