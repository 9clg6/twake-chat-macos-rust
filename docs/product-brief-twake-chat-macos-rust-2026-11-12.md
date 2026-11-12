# Product Brief: Twake Chat (native desktop, Rust + GPUI)

**Date:** 2026-11-12
**Author:** clement
**Version:** 1.0
**Project Type:** desktop-app
**Project Level:** 4 (large: many features, 3 operating systems)

---

## Executive Summary

A native desktop client for Twake Chat, a Matrix client of the Twake suite, written in Rust with GPUI (Zed's UI framework) instead of Electron. It reproduces the current React app (same features, same design as `@linagora/twake-mui`) for people who use Twake Chat all day. macOS comes first; Windows and Linux are designed in from day one. The point is a lighter, faster and more secure client than a web view in a shell.

---

## Problem Statement

### The Problem

Chat clients wrapped in Electron carry a full browser per app: high memory use, slow start, slow long timelines. Twake Chat today only exists as a web app (React) and a Flutter app; there is no native desktop client.

### Why Now?

- Discord CTO announced on 2026-11-04 a rewrite of its desktop client in Rust on GPUI, with an early build around 290 MB of RAM (single data point, not a shipped benchmark). Sources: ghacks.net 2026-11-06, invenglobal.com article 28182.
- `gpui` 0.2.2 is on crates.io (2026-10-30) and `matrix-sdk` / `matrix-sdk-ui` 0.19.1 (2026-09-18) cover sliding sync (MSC4186), Rust crypto and SSO login.
- Three open-source GPUI Matrix clients exist (Rivet, theGrid, twayblade): the stack is feasible.

### Impact if Unsolved

Desktop users keep a browser tab or a heavy shell; the Twake suite has no native presence on the desktop.

---

## Target Audience

### Primary Users

People who use Twake Chat daily at work on a desktop (macOS first): long conversations, many rooms, several accounts, calls, assistants.

### Secondary Users

- Windows and Linux users of Twake Chat.
- Developers who want a fast Matrix client built on matrix-rust-sdk.

### User Needs

1. A chat that opens fast and stays light with thousands of messages and rooms.
2. Everything the web app does (rooms, threads, media, calls, encryption), with the same look.
3. Safe handling of keys and sessions (system keychain, signed and notarized builds).

---

## Solution Overview

### Proposed Solution

A Cargo workspace of small crates: domain, application (use cases and ports), Matrix adapter, design-system crates, UI on GPUI, platform glue per OS. The UI is a 1:1 port of the React app, driven by the feature inventory and the design-system export (`docs/spec/`).

### Key Features

- Sign in with the Synapse SSO; session in the system keychain.
- Room list, filters, folders, spaces; timeline; composer; threads; reactions; media; search.
- End-to-end encryption, recovery key, verification.
- Calls (MatrixRTC + LiveKit), notifications, multi-account.
- Light and dark themes from twake-mui tokens; accessibility; same languages as the web app.

Full list with priorities: `docs/spec/feature-inventory.md`.

### Value Proposition

Same product, native: lower memory, faster start and scroll, no embedded browser, one codebase for three operating systems.

---

## Business Objectives

### Goals

- Reach feature parity with the React app on the P0 and P1 areas, macOS first.
- Keep the design identical by generating UI tokens from twake-mui, not copying values by hand.
- Ship a signed, notarized macOS build, then Windows and Linux builds.

### Success Metrics (proposed targets, to be confirmed in the PRD)

- Idle memory with a logged-in account and 500 rooms: lower than the React app in a browser tab, measured on the same account.
- Cold start to room list; scroll of a 10,000-message timeline without dropped frames: measured by benchmarks in CI.
- Zero `unsafe` in application crates; `cargo-deny` and `cargo-audit` clean on every PR.

### Business Value

A native presence for the Twake suite on the desktop and a reusable Rust/GPUI design-system layer for other Twake apps.

---

## Scope

### In Scope

- Everything in P0 and P1 of the feature inventory, on macOS.
- Cross-platform architecture, CI on macOS, Windows and Linux from the start.
- A `twake-tokens` crate generated from twake-mui and a component crate aligned on it.

### Out of Scope

- Mobile apps.
- Embedding the Meet iframe (not available natively; calls use LiveKit directly).
- Web-only mechanisms (service worker, Web Locks, CSP), replaced by native equivalents.

### Future Considerations

- Windows and Linux release builds once GPUI platform support is verified (accessibility, Vulkan on Linux).
- Auto-update channel; Twake Space integration.

---

## Key Stakeholders

- **Clement (owner, developer)** - High influence. Defines scope and design parity.
- **Twake Chat team (React app owners)** - Medium influence. Source of truth for behaviour (DECISION.md, e2e tests); not yet consulted.

---

## Constraints and Assumptions

### Constraints

- GPUI on crates.io (0.2.2) has no AccessKit and lacks some platform crates; a facade crate must isolate the choice.
- `matrix-sdk-ui` is marked experimental: pin exact versions, one adapter crate.
- Follow the Matrix spec; custom names under `app.twake.*`.
- Public repository: nothing secret in it.

### Assumptions

- The Twake homeserver, ToM, side service and LiveKit endpoints behave as the React app expects.
- Inter font can be shipped with the app (to confirm).
- A spike will confirm GPUI source choice (crates.io vs pinned Zed git).

---

## Success Criteria

- A new user signs in, sees rooms, reads and sends messages, in an encrypted room, on macOS.
- Parity checklist (feature inventory P0 and P1) green with acceptance tests.
- Architecture rules in `ARCHITECTURE.md` enforced by CI, not by review.

---

## Timeline and Milestones

### Target Launch

No date set. Milestones are ordered by dependency, not by calendar.

### Key Milestones

1. Spec: brief, PRD, `ARCHITECTURE.md`.
2. Workspace skeleton + CI on 3 OS + GPUI spike.
3. Sign-in, room list, timeline read-only.
4. Composer, send, E2EE.
5. Threads, media, search, notifications.
6. Calls, multi-account.
7. Signed macOS beta; Windows and Linux builds.

---

## Risks and Mitigation

- **Risk:** GPUI public crate lags Zed and has gaps (accessibility, platform crates).
  - **Likelihood:** High
  - **Mitigation:** `twake-gpui` facade; spike comparing three sources.
- **Risk:** matrix-sdk-ui API churn.
  - **Likelihood:** Medium
  - **Mitigation:** exact pins, single adapter crate, contract tests.
- **Risk:** Pixel parity with MUI is costly (sizes live in JSX, not the theme).
  - **Likelihood:** High
  - **Mitigation:** token export script; per-component visual checks against the web app.
- **Risk:** Calls on three OSes (native WebRTC).
  - **Likelihood:** Medium
  - **Mitigation:** optional crate, late milestone, spike first.

---

## Next Steps

1. PRD (`/prd`), then `ARCHITECTURE.md` (`/architecture`).
2. Sprint planning after both.

**This document was created using BMAD Method v6 - Phase 1 (Analysis)**
