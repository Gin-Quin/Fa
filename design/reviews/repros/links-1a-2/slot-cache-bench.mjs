import { faBorrowSlot } from "/home/claude-dev/.paseo/worktrees/0rhyrubi/links-spec-references/packages/codegen-js/runtime/references.js";
const N = 1_000_000;
function fresh(container, key) { return { get $v() { return container[key]; }, set $v(v) { container[key] = v; } }; }
function run(label, borrow) {
  const values = new Array(N).fill(1);
  const t = performance.now();
  for (let r = 0; r < 5; r++) for (let i = 0; i < N; i++) { const c = borrow(values, i); c.$v = c.$v + 1; }
  const heap = process.memoryUsage().heapUsed;
  console.log(label, (performance.now() - t).toFixed(0), "ms", (heap / 1e6).toFixed(0), "MB heap", values[7]);
}
const which = process.argv[2];
if (which === "cache") run("cached faBorrowSlot", (v, i) => faBorrowSlot(v, i, false, false));
else run("fresh cell", fresh);
