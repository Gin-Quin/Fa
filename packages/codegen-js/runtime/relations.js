// Relations (advanced brief 8.5, 8.8): the sides are properties of the
// elements. A direct side `(T in c)?`, a reverse one-to-one side, a
// symmetric side and an owner field hold the target object or `undefined`; a
// many-to-many direct side and the `Pool` reverse sides hold a `Set` of
// objects. The compiler calls these helpers for every write of a direct side
// and before every removal from a related collection, so that both sides
// always agree.

// @fa-helper faRelateOne
/**
 * `holder.direct = target` for a one-to-many relation (`many`: the reverse
 * side is a `Set` of holders), a one-to-one relation (the reverse side holds
 * the holder) or a symmetric one (`reverse` is `direct`).
 * @param {any} holder
 * @param {string} direct
 * @param {string} reverse
 * @param {any} target the new target, or `undefined`
 * @param {boolean} many
 * @returns {void}
 */
export function faRelateOne(holder, direct, reverse, target, many) {
	const old = holder[direct];
	if (old === target) {
		return;
	}
	if (many) {
		if (old !== undefined) {
			old[reverse].delete(holder);
		}
		holder[direct] = target;
		if (target !== undefined) {
			target[reverse].add(holder);
		}
		return;
	}
	if (old !== undefined) {
		old[reverse] = undefined;
	}
	if (target !== undefined) {
		// the previous holder of the target loses it
		const previous = target[reverse];
		if (previous !== undefined) {
			previous[direct] = undefined;
		}
		target[reverse] = holder;
	}
	holder[direct] = target;
}

// @fa-helper faRelateAdd
/**
 * `holder.direct.add(target)` of a many-to-many relation: whether the link
 * is new.
 * @param {any} holder
 * @param {string} direct
 * @param {string} reverse
 * @param {any} target
 * @returns {boolean}
 */
export function faRelateAdd(holder, direct, reverse, target) {
	if (holder[direct].has(target)) {
		return false;
	}
	holder[direct].add(target);
	target[reverse].add(holder);
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
 * @returns {boolean}
 */
export function faRelateDelete(holder, direct, reverse, target) {
	if (!holder[direct].has(target)) {
		return false;
	}
	holder[direct].delete(target);
	target[reverse].delete(holder);
	return true;
}

// @fa-helper faRelateClear
/**
 * `holder.direct.clear()` of a many-to-many relation.
 * @param {any} holder
 * @param {string} direct
 * @param {string} reverse
 * @returns {void}
 */
export function faRelateClear(holder, direct, reverse) {
	for (const target of holder[direct]) {
		target[reverse].delete(holder);
	}
	holder[direct].clear();
}

// @fa-helper faUnlink
/**
 * Unlinks an element about to leave its collection (8.5 `unlink`). `links`
 * lists the relations it leaves, as `[kind, holder, direct, reverse]`:
 * `kind` 0 one-to-many, 1 one-to-one, 2 many-to-many, 3 symmetric, 4 owner
 * field; `holder` 1 when the element holds the direct side, 0 when it is a
 * target. `owner` is the owner of the collection (owner fields), or
 * `undefined`.
 * @param {any} element
 * @param {Array<[number, number, string, string]>} links
 * @param {any} owner
 * @returns {void}
 */
export function faUnlink(element, links, owner) {
	for (const [kind, holder, direct, reverse] of links) {
		if (holder === 1) {
			const target = element[direct];
			if (kind === 2) {
				for (const item of target) {
					item[reverse].delete(element);
				}
			} else if (target !== undefined) {
				if (kind === 0) {
					target[reverse].delete(element);
				} else {
					target[reverse] = undefined;
				}
			}
			continue;
		}
		switch (kind) {
			case 0:
				for (const item of element[reverse]) {
					item[direct] = undefined;
				}
				break;
			case 1:
				if (element[reverse] !== undefined) {
					element[reverse][direct] = undefined;
				}
				break;
			case 2:
				for (const item of element[reverse]) {
					item[direct].delete(element);
				}
				break;
			case 3:
				if (element[direct] !== undefined && element[direct] !== element) {
					element[direct][direct] = undefined;
				}
				break;
			default:
				if (owner !== undefined && owner[direct] === element) {
					owner[direct] = undefined;
				}
		}
	}
}
