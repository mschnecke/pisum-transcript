# Implementation Plan: CI Code Quality Pipeline

> Generated from: `docs/PRD-13-ci-code-quality.md`
> Date: 2026-04-01

## 1. Overview

Add automated code quality checks (ESLint, Prettier, Clippy, rustfmt) to the CI pipeline so that every PR to `main` is gated on formatting and lint compliance. This brings the project in line with the `github-global-hotkey` reference repo's proven CI patterns.

The feature touches three areas: new frontend tooling configs, CI workflow additions, and existing code fixes to pass the new checks. No new Rust crates or frontend features are introduced — this is purely developer infrastructure.

## 2. Architecture & Design

### CI Job Graph (after changes)

```
pull_request → main
  ├── lint-and-check (ubuntu-latest)
  │     ├── ESLint
  │     ├── Prettier --check
  │     └── svelte-check
  ├── rust-check (ubuntu-latest)
  │     ├── cargo fmt --check
  │     └── cargo clippy -D warnings
  └── build (needs: [lint-and-check, rust-check])
        ├── windows build
        └── macos build
```

`lint-and-check` and `rust-check` run in parallel. The existing matrix `build` job gains a `needs` clause so it only runs after both quality jobs pass.

### New Config Files

| File               | Purpose                                      |
| ------------------ | -------------------------------------------- |
| `eslint.config.js` | Flat-config ESLint for Svelte 5 + TypeScript |
| `.prettierrc`      | Prettier options (plugin ordering, Svelte)   |
| `.prettierignore`  | Excludes build artifacts, Rust target, etc.  |

### Modified Files

| File                       | Change                              |
| -------------------------- | ----------------------------------- |
| `package.json`             | Add devDependencies + scripts       |
| `.github/workflows/ci.yml` | Add two new jobs, gate build        |
| `src/**/*.{svelte,ts}`     | Fix lint/format violations          |
| `src-tauri/src/**/*.rs`    | Fix `cargo fmt` / Clippy violations |

## 3. Phases & Milestones

### Phase 1: Frontend Tooling Setup

**Goal:** ESLint and Prettier installed, configured, and runnable locally.
**Deliverable:** `npm run lint` and `npm run format:check` work against the codebase.

### Phase 2: Fix Existing Violations

**Goal:** All frontend and Rust code passes the new quality checks.
**Deliverable:** `npm run lint`, `npm run format:check`, `npm run check`, `cargo fmt --check`, and `cargo clippy -- -D warnings` all exit 0.

### Phase 3: CI Workflow Integration

**Goal:** Quality checks run automatically on PRs and gate the build.
**Deliverable:** Updated `ci.yml` with `lint-and-check`, `rust-check` jobs and build gating.

### Phase 4: Verification

**Goal:** Confirm the full pipeline works end-to-end on a test PR.
**Deliverable:** A PR where all three jobs (lint-and-check, rust-check, build) pass.

## 4. Files Overview

### Files to Create

| File Path          | Purpose                                            |
| ------------------ | -------------------------------------------------- |
| `eslint.config.js` | Flat-config ESLint setup for Svelte 5 + TypeScript |
| `.prettierrc`      | Prettier formatting options                        |
| `.prettierignore`  | Paths excluded from Prettier                       |

### Files to Modify

| File Path                  | What Changes                                             |
| -------------------------- | -------------------------------------------------------- |
| `package.json`             | Add 7 devDependencies, 4 scripts                         |
| `.github/workflows/ci.yml` | Add `lint-and-check` and `rust-check` jobs; gate `build` |
| `src/**/*.svelte`          | Fix formatting/lint violations (up to 12 files)          |
| `src/**/*.ts`              | Fix formatting/lint violations (up to 3 files)           |
| `src-tauri/src/**/*.rs`    | Fix `cargo fmt` and Clippy violations (up to 28 files)   |

## 5. Task Breakdown

### Phase 1: Frontend Tooling Setup

#### Task 1.1: Install npm devDependencies

- **Files to modify:**
  - `package.json` — add devDependencies
- **Implementation details:**
  ```bash
  npm install -D eslint @eslint/js typescript-eslint eslint-plugin-svelte \
    prettier prettier-plugin-svelte prettier-plugin-tailwindcss
  ```
- **Dependencies:** None
- **Acceptance criteria:** `npm ls eslint prettier` shows all packages installed without errors

#### Task 1.2: Create ESLint configuration

- **Files to create:**
  - `eslint.config.js` — flat config for Svelte 5 + TypeScript
- **Implementation details:**

  ```js
  import js from '@eslint/js';
  import ts from 'typescript-eslint';
  import svelte from 'eslint-plugin-svelte';
  import svelteConfig from './svelte.config.js';

  export default ts.config(
  	js.configs.recommended,
  	...ts.configs.recommended,
  	...svelte.configs['flat/recommended'],
  	{
  		files: ['**/*.svelte', '**/*.svelte.ts'],
  		languageOptions: {
  			parserOptions: {
  				svelteConfig,
  			},
  		},
  	},
  	{
  		ignores: ['build/', 'dist/', '.svelte-kit/', 'src-tauri/', 'node_modules/'],
  	},
  );
  ```

  - The `ignores` block ensures Rust code and build artifacts are excluded.
  - Use the Svelte config to support Svelte 5 runes properly.
  - Refer to the reference repo's config but adapt for this project's Svelte 5 + TS setup.

- **Dependencies:** Task 1.1
- **Acceptance criteria:** `npm run lint` executes without configuration errors (violations are OK at this point)

#### Task 1.3: Create Prettier configuration

- **Files to create:**
  - `.prettierrc` — formatting options
  - `.prettierignore` — paths to exclude
- **Implementation details:**
  `.prettierrc`:

  ```json
  {
  	"useTabs": true,
  	"singleQuote": true,
  	"trailingComma": "all",
  	"printWidth": 100,
  	"plugins": ["prettier-plugin-svelte", "prettier-plugin-tailwindcss"],
  	"overrides": [
  		{
  			"files": "*.svelte",
  			"options": {
  				"parser": "svelte"
  			}
  		}
  	]
  }
  ```

  Note: Review existing code style (tabs vs spaces, quote style) and match the `.prettierrc` to what the codebase already uses to minimize diff churn. The above is a starting point — adjust based on what `src/` files actually use.

  `.prettierignore`:

  ```
  build/
  dist/
  .svelte-kit/
  src-tauri/target/
  src-tauri/gen/
  node_modules/
  package-lock.json
  ```

- **Dependencies:** Task 1.1
- **Acceptance criteria:** `npm run format:check` executes without configuration errors

#### Task 1.4: Add npm scripts to package.json

- **Files to modify:**
  - `package.json` — add scripts
- **Implementation details:**
  Add these scripts (per PRD):
  ```json
  {
  	"lint": "eslint .",
  	"lint:fix": "eslint . --fix",
  	"format:check": "prettier --check .",
  	"format": "prettier --write ."
  }
  ```
- **Dependencies:** Tasks 1.1, 1.2, 1.3
- **Acceptance criteria:** All four scripts run without errors (lint/format violations are expected)

### Phase 2: Fix Existing Violations

#### Task 2.1: Auto-fix frontend formatting with Prettier

- **Files to modify:**
  - All `.svelte`, `.ts`, `.js`, `.css`, `.json` files under `src/` and root config files
- **Implementation details:**
  ```bash
  npm run format
  ```
  Run Prettier's auto-fix across the codebase. Review the diff to ensure no logic changes — only whitespace/formatting. Commit the result as a standalone commit for clean git history.
- **Dependencies:** Phase 1 complete
- **Acceptance criteria:** `npm run format:check` exits 0

#### Task 2.2: Fix frontend lint violations

- **Files to modify:**
  - `.svelte` and `.ts` files under `src/` that have ESLint violations
- **Implementation details:**
  1. Run `npm run lint` to identify violations
  2. Run `npm run lint:fix` for auto-fixable issues
  3. Manually fix remaining violations (unused variables, type issues, etc.)
  4. If certain rules produce false positives with Svelte 5 runes (`$state`, `$derived`, etc.), add targeted rule overrides in `eslint.config.js` rather than disabling broadly
- **Dependencies:** Task 2.1 (format first so lint diffs are clean)
- **Acceptance criteria:** `npm run lint` exits 0

#### Task 2.3: Fix svelte-check violations (if any)

- **Files to modify:**
  - `.svelte` and `.ts` files under `src/` with type errors
- **Implementation details:**
  Run `npm run check` and fix any reported type errors. The `check` script already exists and runs `svelte-check`.
- **Dependencies:** Task 2.2
- **Acceptance criteria:** `npm run check` exits 0

#### Task 2.4: Fix Rust formatting violations

- **Files to modify:**
  - `.rs` files under `src-tauri/src/` with formatting issues
- **Implementation details:**
  ```bash
  cargo fmt --manifest-path src-tauri/Cargo.toml
  ```
  Auto-format all Rust files. Review the diff — should be whitespace-only changes.
- **Dependencies:** None (parallel with Tasks 2.1–2.3)
- **Acceptance criteria:** `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` exits 0

#### Task 2.5: Fix Clippy warnings

- **Files to modify:**
  - `.rs` files under `src-tauri/src/` with Clippy warnings
- **Implementation details:**
  ```bash
  cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
  ```
  Fix all warnings. Common Clippy fixes: unnecessary clones, unused imports, redundant closures, `unwrap()` replacements. Some warnings may require manual judgment (e.g., `allow` attributes for intentional patterns).
- **Dependencies:** Task 2.4 (format first)
- **Acceptance criteria:** `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings` exits 0

### Phase 3: CI Workflow Integration

#### Task 3.1: Add `lint-and-check` job to CI

- **Files to modify:**
  - `.github/workflows/ci.yml` — add new job
- **Implementation details:**
  Add the following job before the `build` job:

  ```yaml
  lint-and-check:
    name: Lint & Check
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version-file: '.nvmrc'
          cache: 'npm'

      - name: Install dependencies
        run: npm ci

      - name: ESLint
        run: npm run lint

      - name: Prettier
        run: npm run format:check

      - name: Svelte Check
        run: npm run check
  ```

  - Use `node-version-file: '.nvmrc'` (not hardcoded version) per PRD.
  - The `cache: 'npm'` setting provides npm dependency caching for performance.

- **Dependencies:** Phase 2 complete (code must pass checks)
- **Acceptance criteria:** Job definition is valid YAML and references correct script names

#### Task 3.2: Add `rust-check` job to CI

- **Files to modify:**
  - `.github/workflows/ci.yml` — add new job
- **Implementation details:**
  Add the following job:

  ```yaml
  rust-check:
    name: Rust Check
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install system dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y \
            libwebkit2gtk-4.1-dev \
            libappindicator3-dev \
            librsvg2-dev \
            patchelf \
            libasound2-dev

      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy, rustfmt

      - name: Cache Rust dependencies
        uses: Swatinem/rust-cache@v2
        with:
          workspaces: src-tauri

      - name: Check formatting
        run: cargo fmt --manifest-path src-tauri/Cargo.toml -- --check

      - name: Clippy
        run: cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
  ```

  - Per the resolved open question, `libopus-dev` is NOT needed (macOS only).
  - Rust cache is scoped to `src-tauri` workspace.

- **Dependencies:** Phase 2 complete
- **Acceptance criteria:** Job definition is valid YAML; system deps match Tauri 2 requirements on Ubuntu

#### Task 3.3: Gate build job on quality checks

- **Files to modify:**
  - `.github/workflows/ci.yml` — modify existing `build` job
- **Implementation details:**
  Add `needs` clause to the existing `build` job:
  ```yaml
  build:
    needs: [lint-and-check, rust-check]
    # ... rest of existing build config unchanged
  ```
- **Dependencies:** Tasks 3.1, 3.2
- **Acceptance criteria:** Build job YAML includes `needs: [lint-and-check, rust-check]`

### Phase 4: Verification

#### Task 4.1: Verify full pipeline on a test PR

- **Implementation details:**
  1. Push the branch with all changes
  2. Open a PR to `main`
  3. Verify all three jobs appear in the checks UI
  4. Verify `lint-and-check` and `rust-check` run in parallel
  5. Verify `build` waits for both quality jobs to pass
  6. Confirm all jobs pass green
- **Dependencies:** Phase 3 complete
- **Acceptance criteria:** All CI jobs pass; build is gated correctly

## 6. Data Model Changes

No data model changes. This feature is CI/tooling infrastructure only.

## 7. API Changes

No API changes. This feature is CI/tooling infrastructure only.

## 8. Dependencies & Risks

### External Dependencies

| Package                       | Version | Purpose                          |
| ----------------------------- | ------- | -------------------------------- |
| `eslint`                      | latest  | JavaScript/TypeScript linting    |
| `@eslint/js`                  | latest  | ESLint recommended rules         |
| `typescript-eslint`           | latest  | TypeScript ESLint parser + rules |
| `eslint-plugin-svelte`        | latest  | Svelte-specific ESLint rules     |
| `prettier`                    | latest  | Code formatting                  |
| `prettier-plugin-svelte`      | latest  | Svelte file formatting           |
| `prettier-plugin-tailwindcss` | latest  | Tailwind class sorting           |

### GitHub Actions Dependencies

| Action                   | Version | Purpose                 |
| ------------------------ | ------- | ----------------------- |
| `actions/checkout`       | v4      | Already used            |
| `actions/setup-node`     | v4      | Already used            |
| `dtolnay/rust-toolchain` | stable  | Already used            |
| `Swatinem/rust-cache`    | v2      | Rust dependency caching |

### Risks

| Risk                                            | Likelihood | Mitigation                                                    |
| ----------------------------------------------- | ---------- | ------------------------------------------------------------- |
| ESLint/Svelte 5 runes incompatibility           | Medium     | Test locally first; add targeted rule overrides if needed     |
| Large formatting diff obscures real changes     | Low        | Separate formatting commit from config changes                |
| Clippy warnings require non-trivial refactoring | Low        | Use `#[allow()]` for intentional patterns with comments       |
| Ubuntu system deps don't match Tauri 2 needs    | Low        | Reference existing CI build job's macOS deps for completeness |

### Assumptions

- The existing `npm run check` (svelte-check) script works on Ubuntu without Tauri-specific system deps
- Prettier's Tailwind plugin handles Svelte component class attributes correctly
- No Clippy warnings in dependencies (only project code is checked)

## 9. Testing Strategy

This feature is CI infrastructure — there are no unit tests to write. Verification is done through:

- **Local validation:** Run all five commands locally and confirm exit 0:
  - `npm run lint`
  - `npm run format:check`
  - `npm run check`
  - `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`
  - `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings`
- **CI validation:** Open a PR and confirm all jobs pass
- **Negative test:** Intentionally introduce a lint violation on a branch, push, and confirm CI catches it (optional but recommended)

### Edge Cases

- Svelte 5 runes (`$state`, `$derived`, `$effect`) may trigger ESLint "unused variable" false positives — handle with rule overrides
- Files with mixed line endings — Prettier should normalize these
- Large generated files (if any) — ensure `.prettierignore` covers them

## 10. Requirement Traceability

### Functional Requirements

| PRD Ref | Requirement Summary          | Task(s)       | Notes                                     |
| ------- | ---------------------------- | ------------- | ----------------------------------------- |
| FR-1    | Add ESLint configuration     | 1.1, 1.2, 1.4 | Flat config, Svelte 5 + TS                |
| FR-2    | Add Prettier configuration   | 1.1, 1.3, 1.4 | Includes Svelte + Tailwind plugins        |
| FR-3    | Add `lint-and-check` CI job  | 3.1           | Ubuntu, Node from .nvmrc, npm cache       |
| FR-4    | Add `rust-check` CI job      | 3.2           | System deps, clippy + rustfmt, Rust cache |
| FR-5    | Gate build on quality checks | 3.3           | `needs: [lint-and-check, rust-check]`     |
| FR-6    | CI trigger configuration     | 3.1, 3.2      | PR to main only (no change to triggers)   |

### Non-Functional Requirements

| PRD Ref | Requirement Summary                              | How Addressed                                                                |
| ------- | ------------------------------------------------ | ---------------------------------------------------------------------------- |
| NFR-1   | CI performance (<3 min, caching, parallel)       | npm cache in lint job, Rust cache in rust-check, jobs run in parallel        |
| NFR-2   | Developer experience (local scripts)             | All checks available as npm scripts (lint, format:check, etc.)               |
| NFR-3   | Compatibility (.nvmrc, stable Rust, Ubuntu deps) | Node from `.nvmrc`, `dtolnay/rust-toolchain@stable`, explicit `apt-get` deps |

### User Stories

| PRD Ref | User Story Summary                 | Implementing Tasks          | Fully Covered? |
| ------- | ---------------------------------- | --------------------------- | -------------- |
| US-1    | CI catches issues on PRs           | 3.1, 3.2, 3.3               | Yes            |
| US-2    | Reviewer focus on logic, not style | 3.1, 3.2 (automated checks) | Yes            |
| US-3    | Consistent code style              | 1.2, 1.3, 2.1–2.5           | Yes            |

### Success Metrics

| Metric                                                 | How the Plan Addresses It         |
| ------------------------------------------------------ | --------------------------------- |
| ESLint, Prettier, svelte-check run on every PR         | Task 3.1: `lint-and-check` job    |
| `cargo fmt --check` and `cargo clippy` run on every PR | Task 3.2: `rust-check` job        |
| Build only runs after lint/check jobs pass             | Task 3.3: `needs` clause on build |
| Local scripts available for pre-PR validation          | Task 1.4: npm scripts added       |
