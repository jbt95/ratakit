# ratakit Component Catalog

ratakit is crate-first: interaction state lives in `ratakit-core`, themed
rendering lives in `ratakit-widgets`, and applications own event loops, terminal
sessions, focus policy, key bindings, and hit testing. Widgets render into
caller-provided areas and do not create a runtime.

Run `cargo run -p gallery` to explore the components. The **UI** page shows
controls and visual primitives; the **Command** page shows the fuzzy picker
under its command-palette name.

## In this catalog

- [Form controls](#form-controls)
- [Visual primitives](#visual-primitives)
- [Generic widgets](#generic-widgets)
- [Agent widgets](#agent-widgets)
- [Composition and ownership](#composition-and-ownership)
- [Theming and accessibility](#theming-and-accessibility)

## Form controls

| Widget | State | Interaction |
|---|---|---|
| `Checkbox` | `CheckboxState` (`ToggleState` alias) | Space or Enter toggles through `Component::handle`; `set_disabled` prevents activation. |
| `Switch` | `SwitchState` (`ToggleState` alias) | Same state and keyboard contract as `Checkbox`, with a switch-style marker. |
| `RadioGroup` | `RadioGroupState` | Up/Down or Left/Right selects and wraps; Home/End selects the first/last option. |
| `Select` | `SelectState` | Enter, Space, or Down opens; arrows move the highlight; Enter commits; Escape closes without changing the selected value. |

`RadioGroup` and `Select` use option indices in core state. Keep the option
ordering stable between renders, or update it with `set_option_count` and map
the reported selection to the application's domain value.

```rust,ignore
let mut notifications = CheckboxState::new(false);
let checkbox = Checkbox::new(&theme, "Email notifications");
frame.render_stateful_widget(checkbox, area, &mut notifications);

if let Outcome::Intent(ToggleIntent::Changed(checked)) =
    notifications.handle(&event, &keymap)
{
    app.notifications_enabled = checked;
}
```

The application decides when a control receives input. Render state does not
automatically establish focus, activate a hit target, or dispatch events.

## Visual primitives

| Widget | Purpose |
|---|---|
| `Button` | Single-line action label with default, secondary, destructive, outline, ghost, and link variants. |
| `Badge` | Compact label using the same semantic variants as `Button`. |
| `Card` | Bordered surface; render child widgets inside `Card::inner(area)`. |
| `Separator` | Horizontal or vertical divider. |
| `Progress` | Determinate bar with an optional percentage. Values are clamped to 0–100%; non-finite input renders as zero. |
| `Skeleton` | Static placeholder fill. It does not schedule animation. |

`Button` is presentational: `.focused(true)` styles it but does not create a
focus target or event handler. Applications own activation and hit testing.

## Generic widgets

| Area | Widgets | Feature |
|---|---|---|
| Dialogs and overlays | `Dialog`, `Modal`, `ConfirmDialog` | `widgets` |
| Editing and completion | `TextArea`, `CompletionMenu` | `widgets`; completion menu also needs `fuzzy` |
| Selection and navigation | `Tabs`, `Table`, `Tree`, `VirtualList`, `ScrollView`, `FuzzyPicker` | `widgets`; fuzzy picker also needs `fuzzy` |
| Text and review | `CodeBlock`, `Markdown`, `DiffView` | `widgets`; Markdown needs `markdown`, diff view needs `diff`; Syntect highlighting is optional |
| Status and feedback | `StatusBar`, `KeyHints`, `Spinner`, `Toasts` | `widgets` |
| Terminal integration | Crossterm conversion and `TerminalSession` helpers | `crossterm` |
| Async helpers | Event stream and `FrameScheduler` | `tokio` |

`CommandPalette` is an alias for the existing `FuzzyPicker`, gated by `fuzzy`;
it does not introduce a second command-menu implementation.

## Agent widgets

The optional `ratakit-agent` crate provides protocol-neutral transcript,
composer, permission, model-picker, plan, and usage state. Its `widgets` feature
adds the corresponding Ratatui views and `AgentLayout`. The umbrella `agent`
feature enables this default widget feature. See the [User Guide](USER_GUIDE.md)
for feature selection and the [Integration Cookbook](COOKBOOK.md) for wiring an
agent to an application.

## Composition and ownership

Keep a `Theme` and each widget's state in application state; render the widget
each frame using `Widget` or `StatefulWidget`. Route normalized
`ratakit_core::Event` values to focused `Component` state and translate
`Outcome` intents into application behavior. Keep terminal I/O, async work, and
domain side effects in the application.

For patterns and testing seams, see the [Integration Cookbook](COOKBOOK.md).
For dependency setup and feature details, see the [User Guide](USER_GUIDE.md).

## Theming and accessibility

Widgets borrow `Theme` and use its semantic palette and glyph profile. Use the
high-contrast theme constructors when their tested palettes fit the app, and
`Theme::enable_screen_reader()` for text-first labels, ASCII markers, and
reduced motion. Keep functionality available from the application's keyboard
policy; a visual focus marker alone is not an interaction model.
