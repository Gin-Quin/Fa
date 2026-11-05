// Relations (advanced brief 8.5, 8.8): the sides are properties of the
// elements. A direct side `(T in c)?`, a reverse one-to-one side, a
// symmetric side and an owner field hold the target object or `undefined`; a
// many-to-many direct side and the `Pool` reverse sides hold a `Set` of
// objects. The compiler calls these helpers for every write of a direct side
// and before every removal from a related collection, so that both sides
// always agree.

// @fa-helper faSides
/**
 * The property of an object whose type has relation sides: the names of its
 * sides (a constant of the type). Erased generic code reads it, so that a
 * copy has no links and printing, equality and hashing leave the sides out
 * (D18, D21), as the code generated per type does.
 */
export const faSides = Symbol.for("fa.sides");

// @fa-helper faRelateOne
/**
 * `holder.direct = target` for a one-to-many relation (`many`: the reverse
 * side is a `Set` of holders), a one-to-one relation (the reverse side holds
 * the holder) or a symmetric one (`reverse` is `direct`). `notify`, given
 * when the sides are reactive, is called with each object and side changed
 * (`faNotifyField`).
 * @param {any} holder
 * @param {string} direct
 * @param {string} reverse
 * @param {any} target the new target, or `undefined`
 * @param {boolean} many
 * @param {(object: any, side: string) => void} [notify]
 * @returns {void}
 */
export function faRelateOne(holder, direct, reverse, target, many, notify) {
	const old = holder[direct];
	if (old === target) {
		return;
	}
	if (many) {
		if (old !== undefined) {
			old[reverse].delete(holder);
			notify?.(old, reverse);
		}
		holder[direct] = target;
		notify?.(holder, direct);
		if (target !== undefined) {
			target[reverse].add(holder);
			notify?.(target, reverse);
		}
		return;
	}
	if (old !== undefined) {
		old[reverse] = undefined;
		notify?.(old, reverse);
	}
	if (target !== undefined) {
		// the previous holder of the target loses it
		const previous = target[reverse];
		if (previous !== undefined) {
			previous[direct] = undefined;
			notify?.(previous, direct);
		}
		target[reverse] = holder;
		notify?.(target, reverse);
	}
	holder[direct] = target;
	notify?.(holder, direct);
}

// @fa-helper faRelateAdd
/**
 * `holder.direct.add(target)` of a many-to-many relation: whether the link
 * is new.
 * @param {any} holder
 * @param {string} direct
 * @param {string} reverse
 * @param {any} target
 * @param {(object: any, side: string) => void} [notify]
 * @returns {boolean}
 */
export function faRelateAdd(holder, direct, reverse, target, notify) {
	if (holder[direct].has(target)) {
		return false;
	}
	holder[direct].add(target);
	target[reverse].add(holder);
	notify?.(holder, direct);
	notify?.(target, reverse);
	return true;
}

// @fa-helper faRelateDelete
/**
 * `holder.direct.delete(target)` of a many-to-many relation: whether there
 * was a link.
 * @param {any} holder
 * @param {string} direct
 * @param {string} reverse
 * @param {any} target
 * @param {(object: any, side: string) => void} [notify]
 * @returns {boolean}
 */
export function faRelateDelete(holder, direct, reverse, target, notify) {
	if (!holder[direct].has(target)) {
		return false;
	}
	holder[direct].delete(target);
	target[reverse].delete(holder);
	notify?.(holder, direct);
	notify?.(target, reverse);
	return true;
}

// @fa-helper faRelateClear
/**
 * `holder.direct.clear()` of a many-to-many relation.
 * @param {any} holder
 * @param {string} direct
 * @param {string} reverse
 * @param {(object: any, side: string) => void} [notify]
 * @returns {void}
 */
export function faRelateClear(holder, direct, reverse, notify) {
	for (const target of holder[direct]) {
		target[reverse].delete(holder);
		notify?.(target, reverse);
	}
	holder[direct].clear();
	notify?.(holder, direct);
}

// @fa-helper faUnlink
/**
 * Unlinks an element about to leave its collection (8.5 `unlink`): the
 * sides that point at it forget it, and its own sides are emptied (an
 * element taken out is a value without links, D18). `links`
 * lists the relations it leaves, as `[kind, holder, direct, reverse]`:
 * `kind` 0 one-to-many, 1 one-to-one, 2 many-to-many, 3 symmetric, 4 owner
 * field; `holder` 1 when the element holds the direct side, 0 when it is a
 * target. `owner` is the owner of the collection (owner fields), or
 * `undefined`. `notify`, given when the sides are reactive, is called with
 * each other object and side changed.
 * @param {any} element
 * @param {Array<[number, number, string, string]>} links
 * @param {any} owner
 * @param {(object: any, side: string) => void} [notify]
 * @returns {void}
 */
export function faUnlink(element, links, owner, notify) {
	for (const [kind, holder, direct, reverse] of links) {
		if (holder === 1) {
			const target = element[direct];
			if (kind === 2) {
				for (const item of target) {
					item[reverse].delete(element);
					notify?.(item, reverse);
				}
				element[direct] = new Set();
			} else if (target !== undefined) {
				if (kind === 0) {
					target[reverse].delete(element);
				} else {
					target[reverse] = undefined;
				}
				notify?.(target, reverse);
				element[direct] = undefined;
			}
			continue;
		}
		switch (kind) {
			case 0:
				for (const item of element[reverse]) {
					item[direct] = undefined;
					notify?.(item, direct);
				}
				element[reverse] = new Set();
				break;
			case 1:
				if (element[reverse] !== undefined) {
					element[reverse][direct] = undefined;
					notify?.(element[reverse], direct);
					element[reverse] = undefined;
				}
				break;
			case 2:
				for (const item of element[reverse]) {
					item[direct].delete(element);
					notify?.(item, direct);
				}
				element[reverse] = new Set();
				break;
			case 3:
				if (element[direct] !== undefined) {
					if (element[direct] !== element) {
						element[direct][direct] = undefined;
						notify?.(element[direct], direct);
					}
					element[direct] = undefined;
				}
				break;
			default:
				if (owner !== undefined && owner[direct] === element) {
					owner[direct] = undefined;
					notify?.(owner, direct);
				}
		}
	}
}
