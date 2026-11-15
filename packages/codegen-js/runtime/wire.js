import { faCodec } from "./codec.js";

// @fa-helper faStaticCodec deps: faCodec
/** A worklist of statically generated steps, never a schema interpreter.
 * Generic factories close over concrete TypeInfo operations at construction.
 * @param {(out:any,value:any,pending:any[])=>void} write
 * @param {(input:any,target:any,key:any,pending:any[])=>void} read
 * @returns {any}
 */
export function faStaticCodec(write, read) {
	const codec = faCodec.fromFunctions((out, value) => {
		const pending = [write, value];
		while (pending.length !== 0) {
			const item = pending.pop();
			pending.pop()(out, item, pending);
		}
	}, (input) => {
		const result = [];
		const pending = [read, result, 0];
		while (pending.length !== 0) {
			const key = pending.pop();
			const target = pending.pop();
			pending.pop()(input, target, key, pending);
		}
		return result[0];
	});
	codec.$write = write;
	codec.$read = read;
	return codec;
}

// @fa-helper faWireLength deps: faCodec
/** @param {any} input @returns {number} */
export function faWireLength(input) {
	const length = faCodec.readVarUint(input);
	if (length > 0xffffffff) throw new TypeError("a Fa wire collection is too long for JavaScript");
	return length;
}
