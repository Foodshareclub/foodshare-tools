# Foodshare Tools - Phasal Execution Plan

## Overview
This plan systematically transforms the foodshare-tools monorepo by:
1. Setting up Sentry for full error tracking and PR integration
2. Deeply componentizing and modulizing the codebase
3. Implementing bleeding-edge practices throughout
4. Executing CI/CD pipelines

---

## Phase 0: Preparation & Analysis ✅
### Completed
- Analyzed monorepo structure (Rust + Bun workspaces)
- Identified existing Sentry dependencies in bun.lock (v10.68.0/10.70.0)
- Mapped crates: web, app, telemetry, core, hooks, cli
- Found no actual Next.js app or Sentry configuration exists

### Codebase Map
- **Rust Workspace**: 27 crates, 7 bin targets
- **Bun Workspace**: packages/, Sentry deps in bun.lock but unused
- **Key existing modules**: nextjs_security.rs, accessibility.rs, bundle_size.rs
- **Missing**: Next.js app, Sentry integration, proper component structure

---

## Phase 1: Set Up Next.js Application with Sentry 🚧
### Objective: Create proper Next.js app and fully configure Sentry

#### 1.1 Create Next.js App Directory Structure
**Files to create:**
- `crates/web/src/app/layout.tsx` - Root layout with Sentry error boundary
- `crates/web/src/app/page.tsx` - Home page
- `crates/web/next.config.ts` - Next.js configuration with Sentry
- `crates/web/tsconfig.json` - TypeScript config
- `crates/web/package.json` - Bun package deps

** bleeding-edge practices:**
- App Router (RSC) pattern
- `export const dynamic = 'force-static'`
- `export const revalidate = 3600`
- Tailwind CSS v4 bleeding edge
- Full TypeScript typings

#### 1.2 Configure Sentry for Next.js
**Files to modify/create:**
- `crates/web/sentry.client.config.ts` - Client-side Sentry config
- `crates/web/sentry.server.config.ts` - Server-side Sentry config
- `@sentry/nextjs` version 10.68.0 (already in bun.lock)

**Sentry configuration:**
```typescript
// sentry.server.config.ts
import * as Sentry from '@sentry/nextjs';

Sentry.init({
  dsn: process.env.SENTRY_DSN,
  environment: process.env.NODE_ENV,
  tracesSampleRate: 0.5,
  attachOnErrorAllNative: true,
  debug: process.env.NEXT_PUBLIC_SENTRY_DEBUG === 'true',
});
```

```typescript
// sentry.client.config.ts
import * as Sentry from '@sentry/react';

Sentry.init({
  dsn: process.env.NEXT_PUBLIC_SENTRY_DSN,
  environment: process.env.NODE_ENV,
  integrations: [
    Sentry.replayIntegration(),
    Sentry.feedbackIntegration(),
  ],
  tracesSampleRate: 0.5,
  replaysSampleRate: 0.3,
});
```

#### 1.3 Enable Sentry Issue/PR Integration
**Features to add:**
- Sentry PR comments on new issues
- GitHub integration for automatic issue creation
- `sentry-cli` for releasing management
- Source map upload for source attribution

**Configuration:**
```bash
# Initialize Sentry CLI
npx @sentry/cli init

# Add to package.json scripts
"sentry:init": "sentry-cli init",
"sentry:release": "sentry-cli releases",
"sentry:debug": "sentry-cli keys deploy",
```

#### 1.4 Create Sentry Error Boundary Component
**Files to create:**
- `crates/web/src/components/SentryErrorBoundary.tsx`
- `crates/web/src/components/ReplayButton.tsx`
- `crates/web/src/components/FeedbackWidget.tsx`

---

## Phase 2: Deep Componentization & Modulization 🧩
### Objective: Break down monolithic structures into focused modules

#### 2.1 Restructure `crates/web` Modules
**Current**: 4 modules in `lib.rs` (accessibility, bundle_size, nextjs_security)

**New structure:**
- `crates/web/src/lib.rs` - Re-exports only
- `crates/web/src/a11y/` - Accessibility checking submodules
  - `lib.rs` - Public API
  - `patterns.rs` - Regex pattern definitions
  - `checker.rs` - File checking logic
  - `reporter.rs` - Result formatting
- `crates/web/src/bundle/` - Bundle analysis submodules
  - `lib.rs` - Public API
  - `analyzer.rs` - Next.js build analysis
  - `formatter.rs` - Size formatting
  - `reporter.rs` - Analysis output
- `crates/web/src/security/` - Security scanning submodules
  - `lib.rs` - Public API
  - `patterns.rs` - OWASP pattern definitions
  - `scanner.rs` - File scanning logic
  - `reporter.rs` - Finding formatting
- `crates/web/src/types.rs` - Shared type definitions

#### 2.2 Create Shared UI Components (packages/ui)
**Enhance `@foodshare/ui` package:**
- Convert README to actual component library
- Build with Radix UI + Tailwind CSS v4
- Component categories: buttons, forms, inputs, modals, etc.
- Add Storybook stories for each component
- Export from `packages/ui/src/lib.ts`

**Component categories to create:**
- `packages/ui/src/button/` - Button variants
- `packages/ui/src/input/` - Input types
- `packages/ui/src/form/` - Form components
- `packages/ui/src/layout/` - Page layout helpers
- `packages/ui/src/feedback/` - Feedback widgets

#### 2.3 Modulize `crates/app` Binary
**Current**: `bins/fs-app/src/main.rs` is 1000+ lines

**New structure:**
- `bins/fs-app/src/commands/` - Command modules
  - `simulator.rs` - Simulator operations
  - `doctor.rs` - Environment checks
  - `verify.rs` - Setup verification
  - `protect.rs` - Code protection
  - `supabase.rs` - Supabase operations
- `bins/fs-app/src/lib.rs` - Shared utilities
- `bins/fs-app/src/status.rs` - Status printing logic
- `bins/fs-app/src/errors.rs` - Error handling

#### 2.4 Create Crate-Specific Type Definitions
**New type crates:**
- `crates/types/` - Shared TypeScript/Rust types
- `crates/web/types/` - Next.js specific types
- `crates/app/types/` - App-specific types

---

## Phase 3: Enhance Functionality with Bleeding-Edge Practices 🌟
### Objective: Improve and add missing capabilities

#### 3.1 Enhance Accessibility Checking
**Add new a11y patterns:**
- `aria-invalid` pattern
- `label-association` pattern  
- `focus-visible` pattern
- Color contrast checks

**Improve existing:**
- Support for TSX/JSX with TypeScript parsing
- JSX pragma detection
- Component prop checking

#### 3.2 Advanced Bundle Size Analysis
**Add:**
- Gzip size computation (use `zlib`)
- Chunk graph analysis
- Import hierarchy mapping
- Real-time bundle profiling during dev

** bleeding-edge:**
- Use Next.js `next/bundle-analyzer` integration
- `ANALYZE=true` support
- Interactive bundle graph

#### 3.3 Sentry-Fullstack Integration
**Add:**
- Browser monitoring (RUM)
- Performance metric tracking
- Custom transactions for foodshare operations
- User feedback integration

**Sentry Replays:**
- Automatic replay on errors
- Privacy filtering (PII redaction)
- Session replay with user consent

#### 3.4 Component Documentation & Discovery
**Add:**
- Component API documentation
- Prop type charts
- Usage examples per component
- Dark mode variants
- Responsive breakpoints

---

## Phase 4: CI/CD Pipeline Integration 🔄
### Objective: Ensure automated testing and deployment

#### 4.1 Turbo Build Pipeline
**Enhance `turbo.json`:**
```json
{
  "$schema": "https://turbo.build/schema.json",
  "tasks": {
    "build": { "dependsOn": ["^build"], "outputs": ["dist/**", "target/**", "pkg/**"] },
    "dev": { "cache": false, "persistent": true },
    "test": { "dependsOn": ["build"], "outputs": ["coverage/**"] },
    "lint": {},
    "type-check": { "dependsOn": ["^build"] },
    "build:wasm": "bun tools/build-wasm.ts",
    "release": { "dependsOn": ["build", "test", "lint", "type-check"] }
  }
}
```

#### 4.2 Sentry CI/CD Integration
**Add to GitHub Actions:**
```yaml
# .github/workflows/sentry.yml
name: Sentry Release

on:
  push:
    branches: [main, develop]
  pull_request:

jobs:
  release:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: '24'
      - uses: sentry/cli/action@v2
        with:
          authToken: ${{ secrets.SENTRY_AUTH_TOKEN }}
          org: foodshare
          project: foodshare-web
      - run: sentry-cli releases propose-version
      - run: sentry-cli releases greet
```

#### 4.3 Rust Test Enhancement
**Add to `Cargo.toml` testing:**
- `cargo test --workspace --all-features`
- Property-based testing with proptest
- Benchmark with criterion
- Snapshot testing with insta

#### 4.4 Bun/Lint/Type Check
**Enhance scripts:**
- `bun run lint` - ESLint with bleeding-edge config
- `bun run type-check` - TypeScript 5.7 strict mode
- `bun run test` - Vitest + Playwright
- `bun run test:rust` - Full Rust test suite

---

## Phase 5: Execution & Verification ✅
### Objective: Commit, push, and watch CI/CD

#### 5.1 Commit Strategy
**Atomic commits per phase:**
```
Phase 1: "feat: setup next.js app with Sentry integration"
Phase 2: "refactor: modularize crates/web into submodules"
Phase 3: "feat: enhance a11y, bundle, security checks"
Phase 4: "chore: CI/CD pipeline enhancements"
Phase 5: "ci: execute pipeline and verify"
```

#### 5.2 Push and CI/CD Execution
**Commands:**
```bash
git add .
git commit -m "phase: initial setup"
git push origin main
# Watch CI/CD execute automatically
```

#### 5.3 Verification Steps
- [ ] Turbo build completes: `bun run build`
- [ ] Type check passes: `bun run type-check`
- [ ] Lint passes: `bun run lint`
- [ ] Rust tests pass: `bun run test:rust`
- [ ] Sentry issues visible in dashboard
- [ ] PR template works correctly
- [ ] All new modules compile without errors

#### 5.4 Success Metrics
- ✅ Next.js app runs without errors
- ✅ Sentry captures errors and creates issues
- ✅ All crates compile: `cargo check --workspace`
- ✅ All Bun types check: `bun run type-check`
- ✅ CI/CD pipeline green on push
- ✅ Component library exports work
- ✅ Accessibility checks functional
- ✅ Bundle size analysis functional
- ✅ Security scanning functional