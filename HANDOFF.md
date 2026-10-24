# Handoff de ratakit

Estado del proyecto a 24 de octubre de 2026 y lo que queda hasta la 0.1.0. El diseño completo y el roadmap están en [`docs/DISENO.md`](docs/DISENO.md); la API del módulo de agente está en su §7.4 y el roadmap en su §14.

## Estado actual

Los hitos 0 a 3 están terminados y mergeados en `main`. El CI está verde.

| Hito | Contenido | PRs |
|---|---|---|
| 0 | Workspace, CI, `ratakit-core`, `ratakit-testing` | #1 |
| 1 | G1–G6: `TextArea`, `VirtualList`, `Highlighter`/`CodeBlock`, `Spinner`/`StatusBar`/`KeyHints`/`Toasts`, `Modal`/`Tabs`/`Table`, `FuzzyFilter` | #2–#7 |
| 2 | G7–G12: `Markdown`, `DiffView`, `FuzzyPicker`, `CompletionMenu`, `Tree`/`ScrollView`, `FrameScheduler`/`event_stream`/`enter_terminal` + ejemplo `gallery` | #8–#13 |
| 3 | A1–A8: `agent::state`, `Transcript`, `ToolCallCard`, `Composer`, `PermissionPrompt`, `ModelPicker`, `UsageBar`/`PlanView`, `AgentLayout` + ejemplo `mock-agent` | #14–#21 |

`crates/ratakit-acp` sigue siendo un esqueleto vacío (solo una constante `STUB`).

### Cómo probarlo

```sh
cargo run -p gallery       # todos los componentes genéricos, por pestañas
cargo run -p mock-agent    # el layout de agente reproduciendo guiones .jsonl
RATAKIT_INLINE=1 cargo run -p mock-agent   # modo inline (opt-in)
```

## Tareas pendientes

Cada fila es un PR. Lo que no depende entre sí puede ir en paralelo.

### Hito 4: ACP y ejemplos reales

- [ ] **C1. `ratakit-acp`: `AcpBridge`.** Depende de A1 y A5 (ya mergeados).
  - Traducir `SessionUpdate` del Agent Client Protocol a `AgentEvent` (mensajes, thinking, tool calls con su estado y diff, plan, uso).
  - Gestionar las peticiones de permiso: `PermissionRequest` hacia la UI y la decisión del usuario de vuelta al agente.
  - Enviar prompts, cancelaciones y cambios de modelo desde `AgentLayoutIntent`.
  - Tests con un agente ACP falso (proceso o transporte en memoria), sin red.
  - Mantener el crate sin dependencia de ratatui, para que lo pueda reutilizar el cliente de review (revu).
- [ ] **C2. `examples/claude-acp` y `examples/opencode-acp`.** Depende de C1 y A8.
  - Una app común de unas 100 líneas que lance el agente por stdio y lo pinte con `AgentLayout`; cada ejemplo solo cambia el comando que lanza.
  - En CI no se pueden ejecutar los agentes reales; basta con que compilen.
- [ ] **C3. Guiones grabados y benchmarks.** Depende de C2.
  - Grabar sesiones reales a `.jsonl`, convertirlas en snapshots y reproducirlas en `mock-agent`.
  - Benchmarks con `criterion` (render del transcript largo, streaming de markdown, diff grande).

### Hito 5: pulido y 0.1

- [ ] **R1. Accesibilidad.** Modo lector de pantalla, alto contraste y auditoría de `MotionMode` (sin animaciones cuando esté desactivado). Depende de A8.
- [ ] **R2. Modo inline completo.** Enviar el historial al scrollback de la terminal (idea de `insert_history.rs` de Codex). Ahora el modo inline existe como opt-in en `enter_terminal`, pero el transcript no vuelca al scrollback. Depende de A2.
- [ ] **R3. Documentación.** Guía de uso, ejemplos en `docs.rs`, README y tabla de compatibilidad con versiones de ratatui. Depende de todo lo anterior.
- [ ] **R4. Release 0.1.0 en crates.io.** Reservar los nombres de los crates y publicar. **Solo con la aprobación de Jordi.** Depende de R1–R3.

## Decisiones tomadas

- Pantalla alternativa (fullscreen) por defecto; inline es opt-in.
- Solo syntect para el resaltado en la v1; nada de tree-sitter.
- "Fullscreen" se refiere solo al viewport de la terminal (`TerminalViewport`, `enter_terminal()`). Ningún enum de widget debe tener una variante `Fullscreen`; se usan nombres como `Fill` o `SingleLine`.

## Preguntas abiertas

1. ¿El adaptador ACP expone las capacidades `fs` y `terminal` del cliente (por ejemplo, para revisar diffs antes de escribir)? Propuesta: después de la 0.1. Hay que decidirlo antes o durante C1.
2. ¿Integración con revu? `ratakit-agent` sin la feature `widgets` y `ratakit-acp` no dependen de ratatui y se podrían compartir. Si hace falta, `agent::state` puede salir a su propio crate (`ratakit-agent-state`).

## Convenciones y lecciones

- **CI (9 checks):** `fmt`, `clippy` (con `-D warnings`), `test`, `msrv` (1.88), `deny`, `docs`, `hello-headless`, `gallery-headless`, `mock-agent-headless`. No renombrar los existentes; añadir jobs nuevos está bien.
- **Merge:** squash-merge cuando el CI esté verde y la rama no vaya por detrás de `main`.
- **PRs en paralelo:** se pisan en `widgets/mod.rs`, `lib.rs`, `state/mod.rs` y `Cargo.toml`. Después de cada merge, las ramas abiertas deben hacer merge de `main` y conservar todos los módulos y exports existentes. Si un PR tiene conflictos, GitHub no ejecuta el CI de `pull_request`; la solución es hacer merge de `main`, no pedir aprobación.
- **Módulos que se enchufan entre sí:** cada componente expone funciones de render o traits desde su propio módulo (por ejemplo `render_tool_call_card`, `render_plan_cell`), para que otros PRs los usen sin editar sus ficheros.
- **Snapshots:** compactos (unas 40 líneas, terminales pequeñas). Los snapshots de miles de líneas no se aceptan.
- **Modo headless:** cada ejemplo tiene una variable (`RATAKIT_GALLERY_HEADLESS=1`, `RATAKIT_MOCK_HEADLESS=1`) que pinta unos frames en un `TestBackend` y sale con código 0; los ejemplos nuevos deben seguir el mismo patrón.
- **Capas:** `ratakit-core` y `ratakit-agent::state` no dependen de ratatui; los widgets de agente van detrás de la feature `widgets`.
