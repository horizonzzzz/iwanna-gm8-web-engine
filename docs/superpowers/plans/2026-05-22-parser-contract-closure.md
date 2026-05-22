# Parser Contract Closure Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Freeze the parser-owned lowered-logic contract into shared runtime-model types and make the IWanna-critical GML subset structurally explicit for parser, runtime-model, and browser loader consumers.

**Architecture:** Parser work stays in Rust and owns the contract shape for `logic.lowered.json`. Shared lowered types move into `iwm-runtime-model` as the single source of truth, the parser emits those types, and the browser shell loads the same shape without heuristic guessing. This plan does not expand gameplay semantics in `iwm-runtime-core`; it only closes the parser/runtime contract around expression and statement structure.

**Tech Stack:** Rust workspace, serde/serde_json, Cargo test, `iwm-parser`, `iwm-runtime-model`, `runtime/` TypeScript loader, current parser notes and runtime gap notes.

---

## File Structure

Planned files for this phase:

- Modify: `crates/iwm-runtime-model/src/lib.rs`
- Modify: `crates/iwm-parser/src/gml_lowering.rs`
- Modify: `crates/iwm-parser/src/lib.rs`
- Modify: `crates/iwm-parser/src/package_builder.rs`
- Modify: `crates/iwm-parser/tests/build_package_smoke.rs`
- Modify: `runtime/src/types.ts`
- Modify: `runtime/src/loadPackage.ts`
- Modify: `docs/notes/package-format-v1-runtime.md`
- Modify: `docs/notes/runtime-wasm-gap-analysis.md`

Responsibilities:

- `iwm-runtime-model`: shared lowered-logic schema
- `iwm-parser/src/gml_lowering.rs`: parser-owned lowering of GML into structured expressions/statements
- `iwm-parser/src/package_builder.rs`: warning generation for unsupported or raw-fallback logic
- `runtime/src/types.ts` and `runtime/src/loadPackage.ts`: browser-side package contract alignment
- `docs/notes/package-format-v1-runtime.md`: current package contract note
- `docs/notes/runtime-wasm-gap-analysis.md`: current runtime blocker note

## Preconditions

Before starting this phase:

- the workspace builds and `cargo test` is green
- `logic.raw.json` and `logic.lowered.json` are already emitted by the parser
- the browser shell already reads `logic.lowered.json` as a package artifact
- the runtime mainline remains OpenGMK-derived WASM execution, so the parser contract must be shaped for a future runner, not for an expanded TS gameplay engine

## Task 1: Move The Lowered Logic Contract Into Shared Model Types

**Files:**
- Modify: `crates/iwm-runtime-model/src/lib.rs`
- Modify: `crates/iwm-parser/src/gml_lowering.rs`
- Modify: `crates/iwm-parser/src/lib.rs`

- [ ] **Step 1: Add the shared lowered-logic types to `iwm-runtime-model`**

Add the lowered contract to `crates/iwm-runtime-model/src/lib.rs` so parser and runtime consume the same schema:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoweredLogicFile {
    pub format: String,
    pub entries: Vec<LoweredLogicEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoweredLogicEntry {
    pub block_id: String,
    pub statements: Vec<LoweredLogicStatement>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", content = "value", rename_all = "kebab-case")]
pub enum LoweredLogicExpr {
    Identifier(String),
    LiteralNumber(f64),
    LiteralBool(bool),
    LiteralText(String),
    Call {
        name: String,
        args: Vec<LoweredLogicExpr>,
    },
    MemberAccess {
        target: Box<LoweredLogicExpr>,
        member: String,
    },
    IndexAccess {
        target: Box<LoweredLogicExpr>,
        index: Box<LoweredLogicExpr>,
    },
    BinaryExpr {
        op: String,
        left: Box<LoweredLogicExpr>,
        right: Box<LoweredLogicExpr>,
    },
    Raw {
        source: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum LoweredLogicStatement {
    Assignment {
        target: LoweredLogicExpr,
        value: LoweredLogicExpr,
    },
    FunctionCall {
        name: String,
        args: Vec<LoweredLogicExpr>,
    },
    Conditional {
        condition: LoweredLogicExpr,
        then_branch: Vec<LoweredLogicStatement>,
        else_branch: Vec<LoweredLogicStatement>,
    },
    With {
        target: LoweredLogicExpr,
        body: Vec<LoweredLogicStatement>,
    },
    Repeat {
        count: LoweredLogicExpr,
        body: Vec<LoweredLogicStatement>,
    },
    While {
        condition: LoweredLogicExpr,
        body: Vec<LoweredLogicStatement>,
    },
    For {
        init: LoweredLogicExpr,
        condition: LoweredLogicExpr,
        step: LoweredLogicExpr,
        body: Vec<LoweredLogicStatement>,
    },
    Raw { source: String },
}
```

- [ ] **Step 2: Point the parser at the shared model**

Replace parser-local lowered imports with explicit `iwm_runtime_model` imports:

```rust
use iwm_runtime_model::{
    LoweredLogicEntry, LoweredLogicExpr, LoweredLogicFile, LoweredLogicStatement,
};

use crate::models::RawLogicFile;
```

- [ ] **Step 3: Re-export the shared lowered types from `iwm-parser`**

Update `crates/iwm-parser/src/lib.rs` so consumers import the lowered contract from one place:

```rust
pub use iwm_runtime_model::{
    LoweredLogicEntry, LoweredLogicExpr, LoweredLogicFile, LoweredLogicStatement,
};
```

- [ ] **Step 4: Verify the parser still builds**

Run:

```bash
rtk cargo test -p iwm-parser
```

Expected:

```text
test result: ok
```

## Task 2: Lower The IWanna-Critical GML Shapes Structurally

**Files:**
- Modify: `crates/iwm-parser/src/gml_lowering.rs`
- Modify: `crates/iwm-parser/tests/build_package_smoke.rs`
- Modify: `crates/iwm-parser/src/package_builder.rs`

- [ ] **Step 1: Add failing parser tests for the critical patterns**

Add tests that pin these shapes:

- nested calls like `instance_create(x, y - 4, choose(obj_player2, obj_player3))`
- member/index/binary expressions like `global.grav = arr[0] + 2`
- structured control flow heads for `with`, `repeat`, `while`, and `for`
- compound assignments like `x += hspeed` and `alarm[0] = 60`
- prefix/postfix increment/decrement like `i++` and `--j`

Use `crates/iwm-parser/tests/build_package_smoke.rs` and assert the lowered nodes directly, not via `Raw`.

- [ ] **Step 2: Run the parser tests to confirm the current gaps**

Run:

```bash
rtk cargo test -p iwm-parser lowered_logic_file_emits_structured_member_index_and_binary_expressions
```

Expected:

```text
pass
```

If a new test fails because a shape still falls back to `Raw`, keep the failure local to the smallest missing branch.

- [ ] **Step 3: Upgrade `lower_statement` and `lower_for_statement`**

Make the lowering order explicit:

```rust
if stmt.starts_with("if ") || stmt.starts_with("if(") {
    return lower_if_statement(stmt);
}

if stmt.starts_with("with ") || stmt.starts_with("with(") {
    return lower_block_statement(stmt, "with").map(|(head, body)| LoweredLogicStatement::With {
        target: lower_expr(&head),
        body: lower_source(&body),
    });
}

if stmt.starts_with("repeat ") || stmt.starts_with("repeat(") {
    return lower_block_statement(stmt, "repeat").map(|(head, body)| LoweredLogicStatement::Repeat {
        count: lower_expr(&head),
        body: lower_source(&body),
    });
}

if stmt.starts_with("while ") || stmt.starts_with("while(") {
    return lower_block_statement(stmt, "while").map(|(head, body)| LoweredLogicStatement::While {
        condition: lower_expr(&head),
        body: lower_source(&body),
    });
}

if stmt.starts_with("for ") || stmt.starts_with("for(") {
    return lower_for_statement(stmt);
}
```

Then insert compound assignment handling before simple `=` splitting, and keep `++` / `--` handling before the control-flow and assignment branches.

- [ ] **Step 4: Keep raw fallback explicit**

If syntax is still unsupported after the structured branches above, emit `Raw` intentionally. Do not silently collapse known critical-path shapes back into string blobs.

- [ ] **Step 5: Re-run parser verification**

Run:

```bash
rtk cargo test -p iwm-parser
```

Expected:

```text
test result: ok
```

## Task 3: Align Browser Contract And Runtime Notes

**Files:**
- Modify: `runtime/src/types.ts`
- Modify: `runtime/src/loadPackage.ts`
- Modify: `docs/notes/package-format-v1-runtime.md`
- Modify: `docs/notes/runtime-wasm-gap-analysis.md`

- [ ] **Step 1: Replace the browser-side lowered-logic placeholder types**

Change `RuntimeLoweredLogicFile` from `Array<Record<string, unknown>>` to a typed discriminated union that matches `LoweredLogicExpr` and `LoweredLogicStatement`.

- [ ] **Step 2: Keep `loadPackage()` fallback behavior**

The loader should still tolerate missing `logic.lowered.json` for older local packages, but the normal path should deserialize the shared lowered contract directly.

- [ ] **Step 3: Update the current notes**

Make both current notes say the same thing:

- the package contract now carries structured lowered nodes for the IWanna-critical path
- raw fallback is transitional, not the intended steady state
- runtime consumption still belongs to the WASM-first path, not a parallel TS gameplay engine

- [ ] **Step 4: Run the frontend verification**

Run:

```bash
rtk npm --prefix runtime test
```

Expected:

```text
pass
```

## Task 4: Final Verification And Cutover Rule

**Files:** none

- [ ] **Step 1: Run workspace verification**

Run:

```bash
rtk cargo test
```

Expected:

```text
test result: ok
```

- [ ] **Step 2: Record the cutover rule**

Once this plan is complete:

- parser contract changes must be checkpointed before runtime extraction uses them
- runtime extraction should not wait for full parser generality
- any new parser shape should be justified by a gold-sample or runtime consumer need

## Self-Review

Coverage check:

- shared lowered contract: covered
- IWanna-critical structured lowering: covered
- browser loader alignment: covered
- parser notes sync: covered
- runtime semantic expansion: intentionally out of scope for this plan
