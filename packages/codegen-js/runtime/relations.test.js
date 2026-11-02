import { describe, expect, test } from "bun:test";

import { faCopyGraph } from "./copy.js";
import { faRelateAdd, faRelateClear, faRelateDelete, faRelateOne, faUnlink } from "./relations.js";

/** A person of the brief's `Town` (advanced brief 8.10), unlinked. */
function person(name) {
	return {
		name,
		bestFriend: undefined,
		bestFriendOf: new Set(),
		mentor: undefined,
		mentee: undefined,
		friends: new Set(),
		friendOf: new Set(),
		spouse: undefined,
	};
}

describe("relation writes (8.5)", () => {
	test("one-to-many: the old target loses the holder", () => {
		const [ana, bao, cleo] = [person("Ana"), person("Bao"), person("Cleo")];
		faRelateOne(ana, "bestFriend", "bestFriendOf", bao, true);
		faRelateOne(ana, "bestFriend", "bestFriendOf", cleo, true);
		expect(bao.bestFriendOf.size).toBe(0);
		expect([...cleo.bestFriendOf]).toEqual([ana]);
		faRelateOne(ana, "bestFriend", "bestFriendOf", undefined, true);
		expect(cleo.bestFriendOf.size).toBe(0);
	});

	test("one-to-one: the previous holder of the target loses it", () => {
		const [ana, bao, cleo] = [person("Ana"), person("Bao"), person("Cleo")];
		faRelateOne(ana, "mentor", "mentee", bao, false);
		faRelateOne(cleo, "mentor", "mentee", bao, false);
		expect(bao.mentee).toBe(cleo);
		expect(ana.mentor).toBeUndefined();
	});

	test("symmetric, self-links included", () => {
		const [ana, bao, cleo] = [person("Ana"), person("Bao"), person("Cleo")];
		faRelateOne(ana, "spouse", "spouse", bao, false);
		expect(bao.spouse).toBe(ana);
		faRelateOne(cleo, "spouse", "spouse", bao, false);
		expect([ana.spouse, bao.spouse, cleo.spouse]).toEqual([undefined, cleo, bao]);
		faRelateOne(ana, "spouse", "spouse", ana, false);
		expect(ana.spouse).toBe(ana);
	});

	test("many-to-many add, delete and clear", () => {
		const [ana, bao] = [person("Ana"), person("Bao")];
		expect(faRelateAdd(ana, "friends", "friendOf", bao)).toBe(true);
		expect(faRelateAdd(ana, "friends", "friendOf", bao)).toBe(false);
		expect(bao.friendOf.has(ana)).toBe(true);
		expect(faRelateDelete(ana, "friends", "friendOf", bao)).toBe(true);
		expect(faRelateDelete(ana, "friends", "friendOf", bao)).toBe(false);
		faRelateAdd(ana, "friends", "friendOf", bao);
		faRelateClear(ana, "friends", "friendOf");
		expect(bao.friendOf.size + ana.friends.size).toBe(0);
	});
});

describe("unlinking a deleted element", () => {
	test("every relation, as target and as holder, and owner fields", () => {
		const [ana, bao, cleo] = [person("Ana"), person("Bao"), person("Cleo")];
		faRelateOne(ana, "bestFriend", "bestFriendOf", bao, true);
		faRelateOne(bao, "bestFriend", "bestFriendOf", cleo, true);
		faRelateOne(ana, "mentor", "mentee", bao, false);
		faRelateAdd(cleo, "friends", "friendOf", bao);
		faRelateAdd(bao, "friends", "friendOf", ana);
		faRelateOne(cleo, "spouse", "spouse", bao, false);
		const town = { first: bao };
		const links = [
			[0, 0, "bestFriend", "bestFriendOf"],
			[0, 1, "bestFriend", "bestFriendOf"],
			[1, 0, "mentor", "mentee"],
			[1, 1, "mentor", "mentee"],
			[2, 0, "friends", "friendOf"],
			[2, 1, "friends", "friendOf"],
			[3, 0, "spouse", "spouse"],
			[4, 0, "first", "first"],
		];
		faUnlink(bao, links, town);
		expect(ana.bestFriend).toBeUndefined();
		expect(cleo.bestFriendOf.size).toBe(0);
		expect(ana.mentor).toBeUndefined();
		expect(cleo.friends.size + ana.friendOf.size).toBe(0);
		expect(cleo.spouse).toBeUndefined();
		expect(town.first).toBeUndefined();
	});
});

describe("copies of owners (8.5)", () => {
	test("faCopyGraph relinks the copies and leaves the original alone", () => {
		const [ana, bao] = [person("Ana"), person("Bao")];
		faRelateOne(bao, "bestFriend", "bestFriendOf", ana, true);
		const town = { people: new Set([ana, bao]) };
		const copy = faCopyGraph(town);
		const [anaCopy, baoCopy] = [...copy.people];
		expect(anaCopy).not.toBe(ana);
		expect(baoCopy.bestFriend).toBe(anaCopy);
		expect([...anaCopy.bestFriendOf]).toEqual([baoCopy]);
		copy.people.clear();
		expect(ana.bestFriendOf.size).toBe(1);
	});
});
