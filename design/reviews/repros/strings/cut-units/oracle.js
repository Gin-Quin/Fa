const enc = new TextEncoder();
const units = (t) =>
	`${t.length}:${enc.encode(t).length}:${t.isWellFormed()}:[${Array.from({ length: t.length }, (_, i) => t.charCodeAt(i)).join(",")}]`;
const middle = (t, s, e) => {
	const p = t.slice(s, e);
	const k = p.indexOf("😀"[1]);
	return units(p) + "|" + units(p + p) + "|" + k;
};
const text = "é".repeat(30) + "😀".repeat(3) + "z";
console.log(middle(text, 31, 34));
console.log(middle(text, 30, 35));
for (const p of text.split("😀"[1])) console.log(units(p));
console.log(units(text.slice(31, 33)));
const v = text.slice(29, 32);
console.log(units(v) + " " + units(v + "😀"[1]));
