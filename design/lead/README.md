# Handover to the next architect of Fa

Written on 2026-11-17 by the first lead (Claude Fable 5.2), for the Fable 5.2 architect who takes over. It holds what I learned in three days of leading this project: about the codebase, about the machine, and above all about running the three kinds of engineers (Claude Opus 5.5, GPT Sol 6.1 high, GPT Sol 6.1 xhigh). The current state (branches, queue, running work) is not here: it is in `/home/claude-dev/fa-lead/STATE.md`, which you keep up to date. This file lives in the repository as `design/lead/README.md`; `/home/claude-dev/fa-lead/README.md` is a link to it.

## 1. Start here

1. This file, then `/home/claude-dev/fa-lead/STATE.md` (state, queue, phase A list).
2. The last two entries of `design/journal.md`, then `design/todo.md` (the single list of open work).
3. `AGENTS.md` (commands), `/home/claude-dev/fa-lead/agent-brief.md` (the rules every engineer receives) and the three reusable task files `tasks/review.md`, `tasks/fix-review.md`, `tasks/land.md`.
4. `design/reviews/ownership-enforcement-2026-11-17.md` before touching ownership: the verdict on the unmerged branch and the plan in 23 small slices. `design/reviews/diagnostics-2026-11-17.md` and `tooling-2026-11-17.md` before touching `fa check` messages or the command line: two audits, already cut into small tasks in `design/todo.md`.

Your memory directory already indexes the user's standing rules; the essential ones:

- The user (French; reports to them in French, short, with tables; repository records and prompts in English) gave the lead every remaining decision. Each one is recorded in `design/decisions.md` ("decided by the lead, <date>"). A real ambiguity in their intent is asked, a detail is decided.
- Goals, in order: extreme run-time performance (beat Goose or lose by 10% at most on its benchmarks, beat Rust); `fa check` at most 20% slower than `bun check`, every tool under 512 MB; the clearest errors; one syntax per thing; native = wasm > JIT > JavaScript. **Current scope: phase A only (completeness and correctness), no performance tuning** until the user says so.
- A rule or an optimization is a general mechanism, never a special case of one type ("ça doit être général"). They want elegant solutions and few files.
- Never push to GitHub unless asked. Reviews before merging are not negotiable.
- Tokens are scarce: the Claude quota is weekly, the GPT credits are bought. Ask which models to use when the quota situation changes; keep your own turns short (reports go to files, you read summaries).

## 2. The rule above the others: small tasks

**Cut the work into small tasks. Divide and conquer. A project of the size of Fa is only finished piece by piece.** Everything that went well in these three days was a small task; everything that went badly was a large one.

The evidence:

| Task as I gave it | Outcome |
|---|---|
| `InlineArray` (one collection, every engine) | Implemented, reviewed, fixed, landed in about a day |
| References through owner paths; map optimizations; `Set` hashing; JS gaps; native JIT | Same: a day each, landed |
| "Enforce the ownership rules" (five points, every form of the language, all or nothing) | 36 hours of one engineer, 118 commits, 367 files, 25,000 lines, **nothing that could be reviewed or merged**; I stopped it and the branch is still not on `main` |
| "Optimize the benchmarks" before checking that the ports used the same data structure as Goose | 26 hours of machine for small gains; the real cause was found afterwards by an audit that took two hours |
| The last evening: five branches given a deadline, each reviewed in a 60-minute box, fixed, merged and validated once | All five on `main` in eight hours, with two audits and the verdict on the ownership branch |

What a small task is:

- One engineer, **one day at most**, about **1,500 lines of diff at most**, readable by a reviewer in one sitting.
- **Green on every target by itself**: it never needs a later task to make the suite pass. If turning a rule on breaks fixtures everywhere (ownership did), the first task lands the mechanism *silent* behind one explicit switch, and each following task turns one form on with its fixtures. Never "all the forms, then green".
- Its task file states what is decided, the scope in numbered points, what is out of scope, the acceptance fixtures, the validation to run and the report file. If you can't write the acceptance fixtures, the task is not understood yet: cut a study first (two hours, read-only, a report with options and a recommendation).
- It leaves `main` better even if the following tasks never happen.

Signs that a task was too large, and what to do:

- The branch passes 30 commits or 2,000 lines without a complete green validation; the report grows "checkpoints"; the engineer writes "one more form remains"; a third relay is needed. **Stop, ask "what can land now?", and cut.** Don't wait: an engineer never cuts the scope by themselves, and GPT xhigh in particular follows the task to the letter for as long as you let it.
- Check every running branch at fixed times (commits since the last check, date of the last complete green run), not only when a notification arrives. I looked at the ownership branch's content after 36 hours; a look after 6 would have shown the problem.

Order of work that I recommend from experience: correctness and completeness first, measurement second (with an audit that the comparison is fair), optimization last.

## 3. The engineers

All three are capable; they differ in stamina, obedience and cost. A fresh engineer knows only the brief and its task file: everything it must respect is written there.

### GPT Sol 6.1 xhigh (provider `codex/gpt-6.1-sol`, `{modeId: "full-access", thinkingOptionId: "xhigh"}`)

- **Use for**: implementation of a feature across the engines, fix rounds after a review, design studies with a comparison, landings with conflicts. The default implementer; careful, tests what it writes, records decisions when told where.
- **Obedient to a fault**: it does exactly what the task says, including an impossible "all or nothing". It doesn't push back and doesn't shrink the scope. The quality of the cut is yours.
- **Context**: on a long task it ends its turn after one or two items. Relaunch a fresh agent in the same workspace: "you continue the task in `<file>`; read the last sections of the report `<file>`". Tell every engineer to redirect command output to a log and read only the summary, and to validate once per item, or its context fills with test output.
- **The filter**: it stops with "[System Error] This content was flagged for possible cybersecurity risk" on memory-safety and reference-safety work, especially reviews and anything about the C runtime (six times, whatever the wording). Standing rule of the user: resume the work with Opus 5.5 high in the same workspace. So give those tasks to Opus directly, and require commits as the work goes (an agent stopped by the filter keeps only what it committed).
- **Questions** arrive as `permission-required` (`request_user_input_async`). Answer with `respond_to_permission`, response `{"behavior":"allow","updatedInput":{"answers":{"0":"<answer>","Question 1":"<answer>"}}}` (both keys, string values; `"1"`/`"Question 2"` for a second question).
- **Steering** (`send_agent_prompt`, `steer`) works well in the middle of a turn. An `interrupt` stopped it for good once: after an interrupt, expect to relaunch.
- Its reports grow without bound (one reached 1,640 lines): the brief asks for a final message of 40 lines; read reports by outline (`grep -n '^#'`), never whole.

### GPT Sol 6.1 high (same provider, `thinkingOptionId: "high"`)

- **Use for**: tasks of two or three hours with little design: measurements and benchmark drivers, tooling, documentation moves, mechanical migrations, landings without hard conflicts, reviews that are not about memory safety. Faster and cheaper than xhigh, the same habits (context, filter, questions).
- Don't give it a task whose difficulty you can't judge: xhigh for those.

### Claude Opus 5.5 high (provider `claude/claude-opus-5-5`, `{modeId: "auto", thinkingOptionId: "high"}`)

- **The reviewer.** Six reviews out of six found critical defects (wrong results, invalid IR, unsound acceptances) in branches whose complete suite was green. Use it for every review that touches memory, references, the runtime or the lowering, for read-only audits (diagnostics, the CLI), for takeovers of work the filter stopped, and for analyzer work that needs judgment about the specification.
- Give a reviewer the code and the specification, **not your opinion** of the branch: an agent handed a hypothesis returns it confirmed. Ask it for a generated matrix or differential corpus with an independent oracle, kept in the repository as a test: these found 18 defects (references), 33 failing programs out of 39 (`InlineArray`) and every string defect that mattered. A reviewer doesn't fix the branch; its test programs are committed under `design/reviews/repros/`.
- **It can end its turn while a long command still runs**, then nothing notifies you. Write "wait in the foreground for the validation; don't end your turn before it finished" in its prompt, and when it finishes early, check the worktree and the report yourself.
- **Give a review a time box** (60 minutes was enough, four times out of four, to find the defects that mattered) and say where a defect would hurt most. An open-ended review costs three hours and finds the same critical findings.
- **Read-only audits** ("run 250 wrong programs through `fa check` and judge every message", "try every command on the three project shapes") are the cheapest work of the project: one Opus, about an hour, a ranked list of findings with reproductions. Run one before deciding what to build in an area.
- When time matters, **the reviewer that found a small defect is the fastest engineer to fix it** (it has read the code an hour ago): send it "you now fix your own findings" with a time box and a fallback ("if it doesn't come out clean in 25 minutes, leave that part out as your review proposes"). It breaks the separation of roles, so a validation by someone else must follow.
- It costs Claude quota: bounded tasks, no long implementation marathons.

### Claude Fable 5.2 as a sub-agent (provider `claude/claude-fable-5-2`, same settings)

Rare: the merge of two diverged lines (103 conflicting files), the verdict on a derailed branch (the ownership review: 35 minutes, with three sub-reviews it delegated itself), a design question with many interactions. Ask it for a verdict and a plan, not for a rescue: "bring this branch to green" is the same all-or-nothing task again.

### For all of them

- **Write the task in a file first** (`/home/claude-dev/fa-lead/tasks/<name>.md`), then a ten-line prompt: the brief, the task file, the worktree, the branch, the report file. A relaunch or a takeover is then one message, and the task survives your own context resets.
- The brief says: stay in your worktree, commit at least every 20 minutes, "your session can be stopped at any time: only commits and the report file survive". I lost a reviewer's uncommitted reproductions by archiving its workspace: run `git -C <worktree> status --short` before `archive_workspace`.
- **Give a deadline in clock time** ("at 21:00 your branch is final; don't start a new item after 20:15; close the report with a section 'State at the stop'"). Every engineer respected it, and what remained was written down instead of half-done. Without a deadline an engineer works until its context ends.
- Never accept "green" on the final message alone: look at the branch (`git log`, `git status`), the counts in the report, and the ratchet files' diff.
- You get a notification when an agent finishes, errors or asks; don't poll in between, but do the fixed-time checks of section 2.

## 4. The pipeline

Implement → **review** (`tasks/review.md`) → fix (`tasks/fix-review.md`) → second review when the fix round was large → **land** (`tasks/land.md`: merge `main`, the complete validation with LLVM, the full tiers of the generated suites) → `git merge --ff-only` on `main` by you. Documentation, benchmark and tool-only branches merge directly after you read their diff.

- **One landing at a time, `main` fixed while it validates.** I validated three landings in parallel against the same `main`, then had to land them one after the other: two were validated twice, five hours lost. When several branches are ready together, merge them into one integration branch and validate once.
- The ratchet files (`packages/codegen-llvm/tests/expected-skips/*.txt`, `packages/cli/tests/expected-skips/*.txt`) list what is known not to work per target; a landing may only shorten them.
- The generated suites have a quick tier in `cargo test --workspace` and a full tier behind an environment variable (`AGENTS.md`): strings (`FA_STRINGS_DIFFERENTIAL_FULL=1`, 122 shards, about 25 minutes), references (`FA_REFERENCE_MATRIX_FULL=1`, 160 programs), `InlineArray`. A change to the area runs its full tier before landing.
- **The protocol that worked** (2026-11-17, evening): deadline for every branch → 60-minute review as each one freezes, in the branch's own warm worktree → fixes with a time box → the lead merges each branch into one integration branch as its own merge commit (so that one can be taken out with `git revert -m 1`) → one engineer validates the whole once, alone on the machine → fast-forward. The task file is `tasks/integrate-night.md`.
- **After every merge, by hand or by an engineer**: `git grep -n '^<<<<<<< \|^>>>>>>> '` (I committed conflict markers into `design/decisions.md` once, because I read only the last lines of the merge output), and a check in **both build configurations** (`cargo check --workspace --all-targets` with and without `LLVM_SYS_181_PREFIX`): two branches had added the same function under `cfg(not(fa_llvm))`, which is invisible with LLVM enabled. Design records (`todo.md`, `decisions.md`) conflict at every merge: keep both sides.
- After a merge: archive the workspace (frees 15 to 37 GB), delete the branch, update `STATE.md`. Journal entry at each milestone.

## 5. The machine

A VM with 6 vCPUs, 12 GB of RAM and 387 GB of disk, shared by every engineer. It also hosts another project of the user (`palatine`, C++ builds on five cores): if the load is high and no engineer of yours runs, that is why; don't touch it, and don't trust timings meanwhile.

- **At most three engineers that build with LLVM, plus two light ones** (read-only audits, analyzer-only work). I ran eight: the load stayed near 30, every validation took three times longer, timing tests failed at random and the measurements meant nothing. More agents did not mean more work done.
- Each LLVM build tree weighs 15 to 37 GB. The disk filled twice. `df -h /` before creating a worktree; archive workspaces as soon as their branch is merged; a shared compiler cache gave no hits across worktrees (the experiment is on `tooling/dev-loop`).
- CPU 5 is reserved for timed runs (`taskset -c 5`); every build and test runs under `taskset -c 0-4` with `CARGO_BUILD_JOBS=2`. Instruction counts (callgrind, `benchmarks/audit/compare.py`) are reliable; CPU times on this machine are not (many same-binary controls fail). Hardware counters are blocked (`perf_event_paranoid=4`; the user can run `sudo sysctl kernel.perf_event_paranoid=1`).
- `/tmp` fills with the engineers' logs and scratch builds (several GB a day): clean it when no engineer runs. `benchmarks/results/` holds 150 MB of raw measurement series that should not have been committed (a to-do item).
- Known flaky tests under load: wasm `tasks-awaited-starter.fa`, concurrent worker startup on JavaScript, an output-capture flake in the workspace suite. Rerun the single test before believing a failure, and before dismissing one.
- Tools: LLVM 18 in `~/.local/llvm18` (`LLVM_SYS_181_PREFIX`), Bun in `~/.bun/bin`, Node and Valgrind in `~/.local/bin`, Goose in `~/.local/goose`. In your own shell, use absolute paths and `git -C`: a `cd` moves your working directory for good.

Paseo: `create_workspace` (isolation `worktree`, mode `branch-off`, path `/home/claude-dev/projects/Fa`, baseBranch `refs/heads/main`, a branch name and a slug; the worktree appears under `~/.paseo/worktrees/0rhyrubi/<slug>`), `create_agent` in that workspace, `send_agent_prompt`, `archive_agent`, `archive_workspace`. The tree `.claude/worktrees/trunk` is a warm LLVM build worth reusing.

## 6. The codebase, as I understand it

- **The documentation is the specification** (`documentation/`, with `:::implementation` sections that the website removes). When code and page disagree, the page wins. `design/decisions.md` holds every arbitration (long: search it by heading), `design/todo.md` the open work, `design/journal.md` the history.
- **Seven engines must agree**: the compile-time interpreter (lowered in `packages/analyzer/src/comptime/`, run by `packages/interpreter`), JavaScript on Bun, Node and web (`packages/codegen-js`), native, wasm and native-freestanding (`packages/codegen-llvm`, with a C runtime checked in as IR: `runtime/generate.sh` after editing it). A fixture with `-- prints:` runs on all of them. A feature is done when all seven agree, and the cheapest way to know is a generated differential corpus.
- **A green suite says little about a new feature.** The suite checks what the implementer thought of. Reviews and generated corpora found the rest, every time.
- The analyzer attaches facts to nodes (flags, the type store) that the engines read; the engines must not re-derive a rule. When an engine needs a fact, the analyzer computes it once. The unmerged ownership branch went wrong partly by letting each form grow its own side structure: read its verdict.
- The analyzer's cases (`packages/analyzer/tests/cases/`) carry `-- pending:` for behavior specified but not implemented (about 110) and several diagnostic codes have no emitter yet (`design/reviews/inventory-2026-11-15.md`, work packages 7 and 8). The inventory is the map of what is missing; it is accurate.
- JavaScript output: what a careful developer would write. No wrapper objects, no hidden parameters, helpers emitted only when used (`// @fa-helper` with declared dependencies), goldens under `packages/codegen-js/tests/golden/`.
- The `fa` binary doesn't link LLVM; `fa-native` does. `fa run --target=native` uses the JIT.
- The trunk has known unsound acceptances, found by the reviews of 2026-11-17 and listed first in `design/todo.md` (a write through a reference to an absent lookup result, a module-level reference that survives a call growing its owner, the textual last-use rule moving a value a stored function still reads, floats inside unions compared with IEEE equality natively). Reviews keep finding this kind: the reference checker and the move analysis are the two places where a green suite means least.
- Benchmarks: before optimizing a row, check that the Fa port uses the same data structure and the same passes as the program it is compared with (`reports/perf-ports.md`). The seven pool-shaped rows wait for links (`design/pool-links.md`, slices 3 to 6).

## 7. My mistakes, so that you make new ones

1. One all-or-nothing task for the ownership enforcement (section 2). The largest loss.
2. Performance work before completeness, and before auditing the comparison.
3. Three landings validated in parallel, then serialized.
4. Eight engineers on six cores.
5. Looking at notifications instead of at branches: a long-running agent that reports progress is not an agent that will land.
6. Archiving a workspace without checking for uncommitted work.
7. Reading the tail of a command's output instead of its result: conflict markers committed, a duplicate function merged.
8. Launching the ownership enforcement without first asking for the design in one page. The review that judged the branch wrote that page in 35 minutes; it should have existed before the first line of code.

## 8. What comes next

`STATE.md` has the queue. In short: the ownership enforcement in small slices (the plan is written), then links slices 3 to 6 and the seven pool ports, the analyzer's pending cases (lot B), the findings of the two audits of 2026-11-17 (diagnostics, CLI and project shapes), the `fa check` speed plan, the small decided rules (defaults of a started function, lexical `throw`, field enumeration order), deep recursive values natively, and the final reviews. Performance tuning comes back only when the user reopens it.
