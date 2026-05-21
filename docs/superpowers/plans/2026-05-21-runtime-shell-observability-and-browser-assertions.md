# Runtime Shell Observability And Browser Assertions Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the WASM-first runtime shell expose stable room/player/tick state in the UI and verify that state through a real browser smoke test.

**Architecture:** Keep `runtime/` as the single browser entrypoint and do not reintroduce any TS gameplay engine. Add a small telemetry view in the shell that renders bridge snapshot state with stable selectors, then add a minimal browser automation path that loads local packages through Vite and asserts on visible runtime state instead of only on canvas output. Use `mashikaku` for motion/reset smoke and `kamilia` for a simpler boot regression.

**Tech Stack:** TypeScript, Vite, Vitest, Playwright, existing WASM bridge, local package fixtures under `runtime/public/packages/`

---

## File Structure

- `runtime/src/ui/shell.ts`
  Renders the new telemetry panel, keeps the existing package inspector and canvas flow, and exposes stable ids for browser assertions.
- `runtime/src/main.test.ts`
  Covers the shell DOM behavior with the existing fake DOM harness.
- `runtime/package.json`
  Adds a browser test script and Playwright dev dependency.
- `runtime/package-lock.json`
  Locks the browser test dependency update.
- `runtime/playwright.config.ts`
  Configures the browser test runner against the local Vite dev server.
- `runtime/tests/browser/runtime-shell.spec.ts`
  Runs the real browser smoke assertions against `mashikaku` and `kamilia`.
- `README.md`
  Documents the new browser verification path.

---

### Task 1: Add a stable runtime telemetry panel to the shell

**Files:**
- Modify: `runtime/src/ui/shell.ts`
- Modify: `runtime/src/main.test.ts`

- [ ] **Step 1: Write the failing shell test**

Add a new `main.test.ts` assertion that loads the WASM path and checks for a dedicated runtime telemetry panel with stable ids and readable text:

```ts
expect(collectText(doc.body)).toContain('Runtime');
expect(collectText(doc.body)).toContain('Execution path');
expect(doc.querySelector('#runtime-room')).not.toBeNull();
expect(doc.querySelector('#runtime-tick')).not.toBeNull();
expect(doc.querySelector('#runtime-player')).not.toBeNull();
```

- [ ] **Step 2: Run the narrow test and confirm it fails**

Run:

```powershell
rtk npm --prefix runtime exec vitest run src/main.test.ts
```

Expected:

- the new assertions fail because the shell does not yet render a runtime telemetry panel

- [ ] **Step 3: Implement the smallest shell change that exposes runtime state**

Add a compact telemetry block in `shell.ts` that renders from `bridge.snapshot()` on each draw. The panel should show:

- execution path
- current room id and room name
- current tick
- player coordinates and velocity when present
- a readable placeholder when player state is absent
- current diagnostics summary

Keep the existing static-room fallback untouched.

- [ ] **Step 4: Re-run the shell test**

Run:

```powershell
rtk npm --prefix runtime exec vitest run src/main.test.ts
```

Expected:

- the new telemetry assertions pass
- the existing static-room viewer assertions still pass

- [ ] **Step 5: Commit the shell observability slice**

Run:

```powershell
rtk git add runtime/src/ui/shell.ts runtime/src/main.test.ts
rtk git commit -m "feat(runtime): expose telemetry in the shell"
```

---

### Task 2: Add a real browser smoke harness

**Files:**
- Create: `runtime/playwright.config.ts`
- Create: `runtime/tests/browser/runtime-shell.spec.ts`
- Modify: `runtime/package.json`
- Modify: `runtime/package-lock.json`

- [ ] **Step 1: Write the failing browser smoke test**

Create a browser spec that opens the Vite dev server, loads `/packages/mashikaku`, and asserts the visible telemetry panel after boot:

```ts
await expect(page.locator('#runtime-status')).toContainText('WASM runtime active');
await expect(page.locator('#runtime-room')).toContainText('2: rInit');
await expect(page.locator('#runtime-player')).toContainText('x=');
```

Extend the same test to prove movement, reset, and room selection:

```ts
const playerBefore = await page.locator('#runtime-player').innerText();
await page.keyboard.press('ArrowRight');
await page.getByRole('button', { name: 'Pause' }).click();
await expect(page.locator('#runtime-player')).not.toHaveText(playerBefore);
await page.getByRole('button', { name: 'Reset' }).click();
await expect(page.locator('#runtime-player')).toHaveText(playerBefore);
await page.locator('select[name="roomSelect"]').selectOption('87');
await expect(page.locator('#runtime-room')).toContainText('87: rStage01');
```

- [ ] **Step 2: Add a second boot-only regression for `kamilia`**

Use the same harness to load `/packages/kamilia` and assert that the telemetry panel shows the WASM path and the default room after boot. This protects the shell against regressions on a different package shape without depending on motion semantics.

- [ ] **Step 3: Configure the browser runner**

Add:

- a `test:browser` script in `runtime/package.json`
- a Playwright config that points at `http://127.0.0.1:4173`
- any dependency entries needed for the browser runner

Use the local dev server already defined by Vite; do not invent a separate server stack.

- [ ] **Step 4: Run the browser test and confirm it fails before the harness exists**

Run:

```powershell
rtk npm --prefix runtime run test:browser
```

Expected:

- the command fails until the new Playwright files and selectors exist

- [ ] **Step 5: Implement the browser harness and selector stability**

Keep the shell selectors simple and stable so the browser test reads the runtime state directly from the page instead of inferring it from canvas pixels.

- [ ] **Step 6: Re-run the browser test**

Run:

```powershell
rtk npm --prefix runtime run test:browser
```

Expected:

- the `mashikaku` motion/reset test passes
- the `kamilia` boot regression passes

- [ ] **Step 7: Commit the browser harness slice**

Run:

```powershell
rtk git add runtime/package.json runtime/package-lock.json runtime/playwright.config.ts runtime/tests/browser/runtime-shell.spec.ts
rtk git commit -m "test(runtime): add browser smoke coverage"
```

---

### Task 3: Update the usage docs for the new verification path

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Document the new verification command**

Add the browser smoke command next to the existing `vitest` and build commands:

```powershell
rtk npm --prefix runtime run test:browser
```

Document that the shell now exposes runtime telemetry for room, tick, player, and diagnostics so browser failures are easier to interpret.

- [ ] **Step 2: Re-run the existing runtime checks**

Run:

```powershell
rtk npm --prefix runtime test
rtk npm --prefix runtime run build
```

Expected:

- the docs update does not disturb the shell build or existing Vitest suite

- [ ] **Step 3: Commit the docs update**

Run:

```powershell
rtk git add README.md
rtk git commit -m "docs: add browser smoke verification path"
```

