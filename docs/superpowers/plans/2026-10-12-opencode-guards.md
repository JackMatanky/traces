# opencode Guard Plugins Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Enforce the mechanically-checkable ALWAYS/NEVER rules from `~/.config/opencode/AGENTS.md` via two global opencode plugins (Rust, Python) plus bash permission rules in the global opencode config.

**Architecture:** Two self-contained, zero-dependency JS plugins in `~/.config/opencode/plugins/` (auto-discovered by opencode). Each exports a pure `scan*()` function (unit-testable with `node:test`) plus a default plugin export wiring `tool.execute.before` (BLOCK via throw) and `tool.execute.after` (WARN via output append). Only new edit/write content is scanned; test/example/bench paths are exempt. Bash-command rules go in `permission.bash` of `~/.config/opencode/opencode.jsonc`.

**Tech Stack:** Node.js (v26, built-in `node:test`), plain JS, opencode plugin hooks. No npm dependencies.

**Spec:** `docs/superpowers/specs/2026-10-12-opencode-guards-design.md`

**Important environment facts:**
- `~/.config/opencode/` is NOT a git repository — plugin tasks have no commit steps. Only the plan/spec docs in this repo are committed.
- opencode loads config and plugins once at startup; a restart is required before scratch-session verification.
- The plugins dir `~/.config/opencode/plugins/` already contains `herdr-agent-state.js` (do not touch).

---

### Task 1: rust-guard plugin (scan + hooks + tests)

**Files:**
- Create: `~/.config/opencode/plugins/rust-guard.js`
- Test: `~/.config/opencode/guard-tests/rust-guard.test.mjs`

- [ ] **Step 1: Write the failing tests**

Create `~/.config/opencode/guard-tests/rust-guard.test.mjs`:

```js
import { test } from "node:test";
import assert from "node:assert/strict";
import { scanRust } from "../plugins/rust-guard.js";

test("blocks unwrap in src", () => {
  const hits = scanRust("/p/src/lib.rs", "let x = foo.unwrap();");
  assert.equal(hits[0]?.rule.id, "no-unwrap");
  assert.equal(hits[0]?.severity, "block");
  assert.equal(hits[0]?.line, 1);
});

test("allows unwrap in tests/examples/benches/test modules", () => {
  assert.deepEqual(scanRust("/p/tests/foo.rs", "foo.unwrap();"), []);
  assert.deepEqual(scanRust("/p/examples/foo.rs", "foo.unwrap();"), []);
  assert.deepEqual(scanRust("/p/benches/foo.rs", "foo.unwrap();"), []);
  assert.deepEqual(scanRust("/p/src/test_helpers.rs", "foo.unwrap();"), []);
});

test("does not flag unwrap on unrelated files", () => {
  assert.deepEqual(scanRust("/p/README.md", "foo.unwrap();"), []);
});

test("blocks println! and dbg!", () => {
  const hits = scanRust("/p/src/a.rs", 'println!("hi");\nlet y = dbg!(x);');
  assert.ok(hits.some((h) => h.rule.id === "no-println" && h.severity === "block"));
  assert.equal(hits.filter((h) => h.rule.id === "no-println").length, 2);
});

test("blocks unsafe without Safety comment", () => {
  const hits = scanRust("/p/src/a.rs", "let p = ptr;\nunsafe { p.read() }");
  assert.ok(hits.some((h) => h.rule.id === "safety-comment" && h.severity === "block"));
});

test("allows unsafe with Safety comment within 5 lines", () => {
  const content = "// Safety: caller guarantees validity.\nunsafe { ptr.read() }";
  assert.deepEqual(scanRust("/p/src/a.rs", content), []);
  const spaced = "let a = 1;\n\n\n// Safety: ok\nunsafe { ptr.read() }";
  assert.deepEqual(scanRust("/p/src/a.rs", spaced), []);
});

test("blocks unsafe when Safety comment is more than 5 lines away", () => {
  const content = "// Safety: stale\nlet a = 1;\nlet b = 2;\nlet c = 3;\nlet d = 4;\nunsafe { ptr.read() }";
  assert.ok(scanRust("/p/src/a.rs", content).some((h) => h.rule.id === "safety-comment"));
});

test("blocks unsafe when an intervening code line appears", () => {
  const content = "// Safety: ok\nlet a = 1;\nunsafe { ptr.read() }";
  assert.ok(scanRust("/p/src/a.rs", content).some((h) => h.rule.id === "safety-comment"));
});

test("warns on empty expect", () => {
  const hits = scanRust("/p/src/a.rs", "let x = foo.expect();");
  assert.equal(hits[0]?.rule.id, "expect-text");
  assert.equal(hits[0]?.severity, "warn");
});

test("does not warn on expect with message", () => {
  assert.deepEqual(scanRust("/p/src/a.rs", 'let x = foo.expect("invariant holds");'), []);
});

test("default export returns hooks", async () => {
  const hooks = await (await import("../plugins/rust-guard.js")).default();
  assert.equal(typeof hooks["tool.execute.before"], "function");
  assert.equal(typeof hooks["tool.execute.after"], "function");
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd ~/.config/opencode/guard-tests && node --test rust-guard.test.mjs`
Expected: FAIL — `Cannot find module '../plugins/rust-guard.js'`

- [ ] **Step 3: Write the plugin**

Create `~/.config/opencode/plugins/rust-guard.js`:

```js
const EXEMPT = [/\/tests\//, /\/examples\//, /\/benches\//, /(^|\/)test_/];

const NO_UNWRAP = {
  id: "no-unwrap",
  message:
    'AGENTS.md bans .unwrap() in library code — use .expect("reason") for invariants or handle the Result.',
  test: (line) => /\.\s*unwrap\s*\(/.test(line),
};

const NO_PRINTLN = {
  id: "no-println",
  message: "AGENTS.md: use tracing macros instead of println!/dbg! in library code.",
  test: (line) => /\bprintln!\s*\(|\bdbg!\s*\(/.test(line),
};

const SAFETY_COMMENT = {
  id: "safety-comment",
  message:
    "AGENTS.md: unsafe requires a // Safety: comment on the nearest preceding non-empty line(s).",
  test: null,
};

const EXPECT_TEXT = {
  id: "expect-text",
  message: '.expect() should carry a descriptive message: .expect("why this invariant holds").',
  test: (line) => /\.expect\s*\(\s*\)/.test(line),
};

function safetyViolation(lines, i) {
  if (!/\bunsafe\s*(\{|fn\b|impl\b|extern\b)/.test(lines[i])) return false;
  for (let j = i - 1, seen = 0; j >= 0 && seen < 5; j--) {
    const prev = lines[j].trim();
    if (prev === "") continue;
    seen++;
    if (prev.includes("// Safety:")) return false;
    const isCommentish = /^(\/\/|#!|#\[)/.test(prev);
    if (!isCommentish) return true;
  }
  return true;
}

export function scanRust(filePath, newContent) {
  if (!filePath.endsWith(".rs")) return [];
  if (EXEMPT.some((re) => re.test(filePath))) return [];
  const lines = newContent.split("\n");
  const hits = [];
  lines.forEach((line, i) => {
    const record = (rule, severity) =>
      hits.push({ rule, severity, line: i + 1, text: line.trim() });
    if (NO_UNWRAP.test(line)) record(NO_UNWRAP, "block");
    if (NO_PRINTLN.test(line)) record(NO_PRINTLN, "block");
    if (safetyViolation(lines, i)) record(SAFETY_COMMENT, "block");
    if (EXPECT_TEXT.test(line)) record(EXPECT_TEXT, "warn");
  });
  return hits;
}

export default async () => {
  let warnQueue = [];
  return {
    "tool.execute.before": async (input, output) => {
      if (input.tool !== "edit" && input.tool !== "write") return;
      const args = output.args ?? {};
      const content = input.tool === "write" ? args.content : args.newString;
      if (typeof content !== "string" || typeof args.filePath !== "string") return;
      const hits = scanRust(args.filePath, content);
      warnQueue = hits.filter((h) => h.severity === "warn");
      const block = hits.find((h) => h.severity === "block");
      if (block) {
        throw new Error(
          `[rust-guard] ${block.rule.id} (line ${block.line}): ${block.rule.message} Offending line: ${block.text}`
        );
      }
    },
    "tool.execute.after": async (input, output) => {
      if (
        (input.tool !== "edit" && input.tool !== "write") ||
        warnQueue.length === 0
      )
        return;
      const notes = warnQueue
        .map((h) => `[rust-guard] ${h.rule.id} (line ${h.line}): ${h.rule.message}`)
        .join("\n");
      warnQueue = [];
      if (typeof output.output === "string") output.output += `\n\n${notes}`;
      else output.output = notes;
    },
  };
};
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd ~/.config/opencode/guard-tests && node --test rust-guard.test.mjs`
Expected: PASS — all tests, 0 failures

- [ ] **Step 5: Syntax-check the plugin**

Run: `node --check ~/.config/opencode/plugins/rust-guard.js`
Expected: no output (exit 0)

- [ ] **Step 6: Confirm no commit is needed**

The plugins directory is not a git repository. Verify with `git -C ~/.config/opencode status` (expected: fatal, not a repository). No commit step for plugin files.

---

### Task 2: py-guard plugin (scan + hooks + tests)

**Files:**
- Create: `~/.config/opencode/plugins/py-guard.js`
- Test: `~/.config/opencode/guard-tests/py-guard.test.mjs`

- [ ] **Step 1: Write the failing tests**

Create `~/.config/opencode/guard-tests/py-guard.test.mjs`:

```js
import { test } from "node:test";
import assert from "node:assert/strict";
import { scanPy } from "../plugins/py-guard.js";

test("warns on Any annotation", () => {
  const hits = scanPy("/p/pkg/mod.py", "def f(x: Any) -> None:\n    pass");
  assert.ok(hits.some((h) => h.rule.id === "no-any" && h.severity === "warn"));
});

test("warns on Any inside subscript", () => {
  const hits = scanPy("/p/pkg/mod.py", "def f(x: list[Any]):\n    pass");
  assert.ok(hits.some((h) => h.rule.id === "no-any"));
});

test("does not flag Any outside type positions", () => {
  assert.deepEqual(scanPy("/p/pkg/mod.py", 'print("Any")\n# use Any carefully\n'), []);
});

test("exempts test files", () => {
  assert.deepEqual(scanPy("/p/tests/test_a.py", "x: Any"), []);
  assert.deepEqual(scanPy("/p/pkg/conftest.py", "x: Any"), []);
  assert.deepEqual(scanPy("/p/pkg/test_helpers.py", "x: Any"), []);
  assert.deepEqual(scanPy("/p/tests/a.py", "x: Any"), []);
});

test("warns on bare python invocation", () => {
  const hits = scanPy("/p/pkg/mod.py", 'os.system("python x.py")');
  assert.ok(hits.some((h) => h.rule.id === "no-sys-python"));
  const hits3 = scanPy("/p/pkg/mod.py", "python3 setup.py build");
  assert.ok(hits3.some((h) => h.rule.id === "no-sys-python"));
});

test("does not flag uv-managed python", () => {
  assert.deepEqual(scanPy("/p/pkg/mod.py", "uv run python x.py"), []);
  assert.deepEqual(scanPy("/p/pkg/mod.py", "uvx ruff check ."), []);
  assert.deepEqual(scanPy("/p/pkg/mod.py", "import opencode_python_docs\n"), []);
});

test("ignores non-python files", () => {
  assert.deepEqual(scanPy("/p/notes.md", "x: Any"), []);
});

test("default export returns hooks", async () => {
  const hooks = await (await import("../plugins/py-guard.js")).default();
  assert.equal(typeof hooks["tool.execute.before"], "function");
  assert.equal(typeof hooks["tool.execute.after"], "function");
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd ~/.config/opencode/guard-tests && node --test py-guard.test.mjs`
Expected: FAIL — `Cannot find module '../plugins/py-guard.js'`

- [ ] **Step 3: Write the plugin**

Create `~/.config/opencode/plugins/py-guard.js`:

```js
const EXEMPT = [/\/tests\//, /(^|\/)test_/, /_test\.py$/, /conftest\.py$/];

const NO_ANY = {
  id: "no-any",
  message: "AGENTS.md: avoid Any — use a concrete type, object, or TypeVar.",
  test: (line) => /:\s*Any\b|->\s*Any\b|\[[^\]]*\bAny\b[^\]]*\]/.test(line),
};

const NO_SYS_PYTHON = {
  id: "no-sys-python",
  message: "AGENTS.md: use `uv run python` / `uvx` instead of bare python/python3.",
  test: (line) =>
    /(^|[^.\w-])(python3?)(\s|$)/.test(line) &&
    !/\buv\s+(run|tool)\b|\buvx\b/.test(line),
};

export function scanPy(filePath, newContent) {
  if (!filePath.endsWith(".py")) return [];
  if (EXEMPT.some((re) => re.test(filePath))) return [];
  const lines = newContent.split("\n");
  const hits = [];
  lines.forEach((line, i) => {
    for (const rule of [NO_ANY, NO_SYS_PYTHON]) {
      if (rule.test(line)) {
        hits.push({ rule, severity: "warn", line: i + 1, text: line.trim() });
      }
    }
  });
  return hits;
}

export default async () => {
  let warnQueue = [];
  return {
    "tool.execute.before": async (input, output) => {
      if (input.tool !== "edit" && input.tool !== "write") return;
      const args = output.args ?? {};
      const content = input.tool === "write" ? args.content : args.newString;
      if (typeof content !== "string" || typeof args.filePath !== "string") return;
      warnQueue = scanPy(args.filePath, content).filter((h) => h.severity === "warn");
    },
    "tool.execute.after": async (input, output) => {
      if (
        (input.tool !== "edit" && input.tool !== "write") ||
        warnQueue.length === 0
      )
        return;
      const notes = warnQueue
        .map((h) => `[py-guard] ${h.rule.id} (line ${h.line}): ${h.rule.message}`)
        .join("\n");
      warnQueue = [];
      if (typeof output.output === "string") output.output += `\n\n${notes}`;
      else output.output = notes;
    },
  };
};
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd ~/.config/opencode/guard-tests && node --test py-guard.test.mjs`
Expected: PASS — all tests, 0 failures

- [ ] **Step 5: Syntax-check the plugin**

Run: `node --check ~/.config/opencode/plugins/py-guard.js`
Expected: no output (exit 0)

---

### Task 3: bash permission rules in global opencode config

**Files:**
- Modify: `~/.config/opencode/opencode.jsonc` (inside the existing `permission.bash` object)

- [ ] **Step 1: Read the current bash permission block**

Run: `sed -n '/"permission"/,/"lsp"/p' ~/.config/opencode/opencode.jsonc`
Note the exact last entry of the `bash` object (currently `"git restore *": "ask",`) so the new entries are inserted inside the object without disturbing existing rules. **Insertion order matters** — opencode evaluates the LAST matching rule; these new patterns do not overlap any existing git patterns, so appending at the end of the `bash` object is safe.

- [ ] **Step 2: Insert the three new rules**

In `~/.config/opencode/opencode.jsonc`, inside `permission.bash`, immediately after the `"git restore *": "ask",` line, insert:

```jsonc
    "cargo build --features python*": "deny",
    "*target-cpu=native*": "deny",
    "cargo bench *": "ask",
```

Preserve all surrounding entries, comments, and commas exactly as they are.

- [ ] **Step 3: Validate the config still parses**

Run: `node -e "const fs=require('fs');const s=fs.readFileSync(process.env.HOME+'/.config/opencode/opencode.jsonc','utf8');const stripped=s.replace(/\/\/.*$/gm,'');JSON.parse(stripped);console.log('ok')"`
Expected: prints `ok`

---

### Task 4: full verification (both plugins + config)

**Files:** none new

- [ ] **Step 1: Run the whole guard test suite**

Run: `cd ~/.config/opencode/guard-tests && node --test *.test.mjs`
Expected: all tests pass, 0 failures (both suites)

- [ ] **Step 2: Confirm plugin files pass syntax check**

Run: `node --check ~/.config/opencode/plugins/rust-guard.js && node --check ~/.config/opencode/plugins/py-guard.js && echo ok`
Expected: prints `ok`

- [ ] **Step 3: Restart opencode and run scratch checks**

Tell the user to quit and restart opencode (config/plugins load once at startup). Then in a scratch session, verify:
1. Ask the agent to edit a temp `.rs` file in `/tmp` adding `foo.unwrap()` → expect the edit rejected with `[rust-guard] no-unwrap ...` in the error.
2. Ask the agent to add `foo.expect()` to a temp `.rs` file → edit succeeds, tool output ends with `[rust-guard] expect-text ...`.
3. Ask the agent to run `cargo build --features python` → expect permission denial.
4. Ask the agent to run `cargo bench -- --test` → expect an `ask` permission prompt.

---

### Task 5: commit the plan/spec docs in this repo

**Files:**
- Commit: `docs/superpowers/plans/2026-10-12-opencode-guards.md` (this file)

- [ ] **Step 1: Commit the plan document**

```bash
git add docs/superpowers/plans/2026-10-12-opencode-guards.md
git commit -m "docs: implementation plan for opencode guard plugins"
```

Expected: commit succeeds (hk pre-commit hooks run on the markdown file; all must pass).
