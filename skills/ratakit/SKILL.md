---
name: ratakit
description: Ratakit: Use when building or modifying a Rust TUI with this library, integrating ratakit into an application, choosing or wiring its widgets and state, or adding a reusable ratakit component.
---

# Ratakit

## Workflow

1. **Find the current contract.** Read `docs/USER_GUIDE.md` first. For widget choices and behavior,
   read `docs/COMPONENTS.md`; for event-loop integration, read `docs/COOKBOOK.md`. Inspect the
   relevant crate source and `crates/ratakit/Cargo.toml` before relying on an API or feature flag.
   **Done when** you know the exact exported type, required feature, and existing application seam.

2. **Choose the application or library path.** Prefer an existing widget and its state API over a
   parallel implementation. The `ratakit` facade exposes `core`, `widgets`, and optional feature
   groups; check its manifest before changing dependencies. This project is not published on
   crates.io yet, so follow the in-repository path dependency examples rather than inventing a
   published version. **Done when** the chosen API, dependency, and enabled features cover the
   request without adding a duplicate implementation.

3. **Keep ownership at the application seam.** In an app, keep the event loop, terminal session,
   runtime, domain state, focus policy, key bindings, and hit testing app-owned. Store `Theme` and
   widget state in app state, render widgets each frame, and pass normalized `ratakit::core::Event`s
   to `Component::handle`. Translate returned `Outcome`s and intents into application side effects
   in the event loop. **Done when** rendering and interaction work while application policy and
   side effects remain in the app.

4. **When extending ratakit, keep state and rendering separate.** Put ratatui-independent interaction
   state and intent types in `ratakit-core`; implement `Component` for event-driven state. Put themed
   rendering in `ratakit-widgets`, using `Theme` and `Widget` or `StatefulWidget`. Keep terminal I/O,
   runtimes, application policy, and side effects out of both crates. A visual-only primitive does
   not need core state. **Done when** state can be tested without ratatui and the widget only renders
   that state.

5. **Preserve component contracts.** `Button` is presentational: applications own activation and hit
   testing; its focus setting only changes appearance. Use `CommandPalette` as the existing
   `FuzzyPicker` alias instead of implementing a duplicate picker. Keep keyboard behavior, disabled
   state, palette colors, and the theme's ASCII/screen-reader profile consistent with neighboring
   widgets. **Done when** these contracts are preserved and every new interaction remains reachable
   through the app's keyboard/focus policy.

6. **Prove the change at the right seam.** Test interaction state without a terminal. Test rendering
   with `ratakit-testing` helpers such as `render_widget` or `render_stateful`, including narrow
   areas and relevant theme profiles. Run the smallest focused tests first. For repository-wide
   changes, use the checks listed in `docs/USER_GUIDE.md`; verify Rust 1.88 compatibility when the
   change relies on language or dependency behavior. **Done when** relevant tests and checks pass,
   or any unrun check and its reason are reported.

## Examples to follow

- `examples/gallery/src/main.rs` demonstrates composing and rendering generic widgets.
- `examples/acp-app/src/lib.rs` demonstrates application-owned event-loop and agent integration.
- `docs/COOKBOOK.md` explains how to route events and intents without moving app policy into ratakit.
