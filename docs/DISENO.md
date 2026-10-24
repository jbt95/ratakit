# ratakit: documento de diseño

> **Nombre definitivo:** **ratakit** (`ratakit`, `ratakit-core`, `ratakit-widgets`, etc.). En crates.io existe un crate antiguo homónimo (0.5.0, sin mantenimiento desde 2022); la publicación del workspace se hará cuando el mantenimiento lo apruebe, sin mezclar con ese paquete.

- **Estado:** borrador 0.1, 24 de octubre de 2026
- **Autor:** Jordi Bermejo Tornero (@jbt95)
- **Licencia prevista:** Apache-2.0
- **Lenguaje:** Rust (edición 2024; la MSRV será la que exija ratatui 0.30)

---

## 1. Contexto

Quien quiere construir un agente de código en terminal como Claude Code, opencode, pi o Codex se encuentra con este panorama:

| Ecosistema | Qué hay | Problema |
|---|---|---|
| TypeScript | **OpenTUI** (núcleo en Zig, usado por opencode) y **pi-tui** (`@earendil-works/pi-tui`) | Son kits bastante completos, pero en TS. Los bindings de OpenTUI para Rust y Go son experimentales. |
| Go | Bubble Tea v2, Bubbles v2, Lip Gloss, Glamour y Huh | Son piezas genéricas sin nada específico para agentes. La interfaz de Crush es interna y tiene licencia FSL. |
| Rust | ratatui 0.30, ratatui-textarea, tui-markdown, tui-tree-widget, tui-scrollview y throbber-widgets-tui | Son piezas sueltas que no comparten modelo de estado, temas ni foco. |
| Rust, agentes | phi-tui, cornichon y agent-air | Son muy recientes y casi nadie los usa; agent-air además impone su propio runtime de agente. |
| Referencia | **TUI de Codex** (`openai/codex`, `codex-rs/tui`, Apache-2.0) | Es la interfaz de agente en Rust más probada en uso real, pero no se publica como librería y está muy acoplada a los protocolos de Codex. |

**El hueco:** no existe en Rust una librería coherente de componentes TUI que sirva para cualquier aplicación y que, además, traiga los componentes propios de un agente de código, ya montados y conectables a cualquier agente mediante ACP (Agent Client Protocol).

## 2. Objetivos y no objetivos

### Objetivos

1. **Dos capas claramente separadas:**
   - **Capa genérica:** componentes útiles para cualquier TUI, como editores, listas, markdown, diffs o selectores.
   - **Capa de agente:** componentes de agente de código construidos *solo* sobre la capa genérica y su API pública.
2. **Estado separado del dibujo.** Todo el estado y la lógica (edición, scroll, filtrado o streaming) vive en un crate sin dependencia de ratatui y se puede probar sin terminal.
3. **Streaming de primera clase.** El texto que llega token a token se dibuja de forma incremental, sin volver a parsear todo el mensaje en cada frame.
4. **Independiente del agente.** La capa de agente usa un modelo de datos propio y neutral. ACP es un adaptador opcional, no una dependencia obligatoria.
5. **Un cliente mínimo en ~100 líneas.** Con el adaptador ACP, un cliente funcional para Claude Code u opencode cabe en un ejemplo corto.
6. **Compatible con el patrón de ratatui.** Usa `Widget`, `StatefulWidget` y `Frame::render_stateful_widget`; no inventa un framework con su propio bucle.
7. **Funciona con o sin tokio.** El núcleo es síncrono; la integración async va detrás de una feature.
8. **Accesible y rápido.** Se puede reducir el movimiento, hay un modo para lectores de pantalla, se puede usar solo con teclado y el redibujado está acotado (sección 9).

### No objetivos

- **No es un framework de aplicación.** No impone arquitectura (Elm, componentes con mensajes, etc.) ni posee el bucle de eventos.
- **No es un runtime de agente.** No llama a LLMs, no ejecuta herramientas y no gestiona permisos reales; solo los *muestra* y devuelve la decisión del usuario.
- **No es un backend de terminal.** Usamos crossterm a través de ratatui; termion y termwiz podrían funcionar, pero no los probamos en la v1.
- **No es un emulador de terminal embebido** en la v1, aunque podría añadirse más adelante con `vt100`.
- **No es un layout de tipo flexbox o CSS.** Usamos el `Layout` de ratatui.
- **No busca compatibilidad binaria con OpenTUI ni pi-tui.**

## 3. Arquitectura general

```
┌──────────────────────────────────────────────────────────────┐
│ Aplicación del usuario (su bucle, su runtime, su agente)     │
├───────────────────────────────┬──────────────────────────────┤
│ ratakit-acp (opcional)         │   ejemplos/                  │
│ SessionUpdate → AgentEvent    │   claude-acp, opencode-acp,  │
│ permisos ACP ↔ PermissionPrompt│  galería, mock-agent        │
├───────────────────────────────┴──────────────────────────────┤
│ ratakit-agent        widgets de agente (transcript, composer, │
│                     permisos, modelo, uso, plan)             │
│   └─ agent::state   modelo de agente sin ratatui              │
├──────────────────────────────────────────────────────────────┤
│ ratakit-widgets      widgets genéricos sobre ratatui-core     │
├──────────────────────────────────────────────────────────────┤
│ ratakit-core         estado, eventos, teclas, foco, temas     │
│                     (sin ratatui, testeable sin terminal)    │
└──────────────────────────────────────────────────────────────┘
```

**Regla de dependencias:** cada capa solo conoce las de abajo. `ratakit-agent` no puede usar internos de `ratakit-widgets` (solo su API pública), y así comprobamos que la capa genérica es suficiente. El CI lo comprueba: `cargo-deny` vigila las dependencias entre crates y `ratakit-agent` compila contra la API pública de `ratakit-widgets`, sin features internas.

## 4. Workspace de crates

```
ratakit/
├── Cargo.toml                 # workspace, versiones centralizadas
├── crates/
│   ├── ratakit-core/           # estado + eventos + keymap + foco + tema (sin ratatui)
│   ├── ratakit-text/           # markdown en streaming, resaltado y diff, como modelo (sin ratatui)
│   ├── ratakit-widgets/        # widgets genéricos (ratatui-core + ratatui-widgets)
│   ├── ratakit-agent/          # estado de agente (sin ratatui) + widgets de agente
│   ├── ratakit-acp/            # adaptador ACP opcional (agent-client-protocol 3.5)
│   ├── ratakit-testing/        # helpers de snapshot, KeySeq, guiones de eventos
│   └── ratakit/                # crate fachada: reexporta por features
├── examples/
│   ├── gallery/               # todos los widgets genéricos, con pestañas
│   ├── mock-agent/            # agente simulado, sin red (demo y tests)
│   ├── claude-acp/            # Claude Code via @agentclientprotocol/claude-agent-acp
│   └── opencode-acp/          # opencode via `opencode acp`
├── NOTICE                     # atribuciones (Codex, ratatui)
└── THIRD_PARTY.md
```

### Responsabilidades

| Crate | Depende de | Contenido |
|---|---|---|
| `ratakit-core` | `unicode-width`, `unicode-segmentation`, `nucleo-matcher` (feature) | `TextBuffer` (rope + cursor + selección + undo), `ScrollState` con *follow tail*, `ListState` virtualizada, `FuzzyFilter`, `Keymap`/`KeyChord`, `FocusRing`, `Theme`/`Palette` (con colores propios, no los de ratatui), `Event` normalizado, `Tick`/`Motion` |
| `ratakit-text` | `pulldown-cmark`, `syntect` + `two-face` o `tree-sitter-highlight`, `similar`, `textwrap` | `MarkdownStream` (parseo incremental por bloques), `StyledLine`/`StyledSpan` (texto con estilo independiente de ratatui), `Highlighter`, `DiffModel` (hunks, intra-línea), wrapping con caché por ancho |
| `ratakit-widgets` | `ratakit-core`, `ratakit-text`, `ratatui-core`, `ratatui-widgets`, `crossterm` (feature) | Los widgets genéricos de la sección 6.1 y la conversión `StyledLine → ratatui::text::Line` |
| `ratakit-agent` | `ratakit-core`, `ratakit-text`; `ratakit-widgets` (API pública) solo con la feature `widgets` | `agent::state` (modelo de agente) y `agent::widgets`. La feature `widgets` va activada por defecto; con `default-features = false` solo queda `state`, **sin ratatui**, para usarlo desde otras interfaces (por ejemplo GPUI) |
| `ratakit-acp` | `ratakit-agent::state`, `agent-client-protocol` 3.5, `tokio` | `AcpBridge`: proceso hijo, conexión, traducción de `SessionUpdate` y permisos |
| `ratakit-testing` | `ratatui` (TestBackend), `insta` | `render_to_string`, `assert_widget_snapshot!`, `KeySeq::parse("ctrl+a h e y enter")` |
| `ratakit` | todos (opcionales) | fachada con `pub use` |

Separar `ratakit-text` de `ratakit-core` permite que quien solo quiera el editor o la lista no arrastre syntect ni pulldown-cmark.

> **Por qué `ratatui-core` y no `ratatui`:** desde la 0.30, ratatui está dividido en `ratatui-core` (0.1.x), `ratatui-widgets` (0.3.x) y los backends, y recomienda que las librerías de widgets dependan de `ratatui-core`, que cambia de API con menos frecuencia. Así reducimos roturas (sección 12). Solo los ejemplos y `ratakit-testing` usan la fachada `ratatui`.

### Features de cargo (crate fachada `ratakit`)

| Feature | Por defecto | Activa |
|---|---|---|
| `widgets` | sí | `ratakit-widgets` |
| `markdown` | sí | `ratakit-text/markdown` (pulldown-cmark) |
| `highlight-syntect` | sí | syntect + two-face, unos 250 lenguajes, regex `fancy-regex` (puro Rust) |
| `highlight-syntect-onig` | no | lo mismo, con Oniguruma (más rápido, necesita compilador C) |
| `highlight-treesitter` | no | `tree-sitter-highlight`; las gramáticas las aporta el usuario |
| `diff` | sí | `similar` |
| `fuzzy` | sí | `nucleo-matcher` (selector, @ficheros, /comandos) |
| `fuzzy-async` | no | `nucleo` (matcher en hilos para listas enormes, por ejemplo un repo con 1M de ficheros) |
| `crossterm` | sí | conversión `crossterm::event::Event → ratakit::Event` y helpers de terminal |
| `tokio` | no | `EventStream` async, `FrameScheduler` sobre `tokio::time` |
| `agent` | no | `ratakit-agent` |
| `acp` | no | `ratakit-acp` (implica `agent` + `tokio`) |
| `serde` | no | `Serialize`/`Deserialize` para temas, keymaps y el estado del transcript |
| `unstable-acp` | no | reexporta `agent-client-protocol/unstable` (planes por ID, subagentes y uso al final del turno) |

## 5. Modelo de estado y eventos

### 5.1 Principios

1. **El estado es un dato plano en `ratakit-core` o `ratakit-agent::state`.** No guarda `Rect`, `Buffer` ni estilos de ratatui; se puede crear, mutar y comparar en tests sin terminal.
2. **La vista es un `StatefulWidget` efímero** que se construye en cada frame (`TextArea::new(&theme)`), recibe `&mut State` y solo guarda en el estado datos de *layout* (último ancho, offset visible, posición del cursor en pantalla). Es el patrón estándar de ratatui.
3. **Los eventos entran por un único método:** `state.handle(&Event, &Keymap) -> Outcome`. `Outcome` dice si el evento se consumió y qué *intención* produjo (enviar, cancelar, abrir menú...). La aplicación decide qué hacer con esa intención.
4. **No hay callbacks ni `Rc<RefCell>`.** Las intenciones se devuelven como valores. Es fácil de probar y de usar desde tokio.
5. **Revisión monotónica.** Cada estado expone `revision() -> u64`, que aumenta con cada cambio visible. Las cachés de layout y de líneas renderizadas se invalidan por `(revision, width, theme_revision)`, la misma idea que el *active-cell cache key* de Codex.

### 5.2 Eventos

```rust
pub enum Event {
    Key(KeyEvent),            // normalizado (kitty keyboard protocol si existe)
    Paste(String),            // bracketed paste
    Mouse(MouseEvent),
    Resize { width: u16, height: u16 },
    FocusGained, FocusLost,
    Tick(Instant),            // animaciones (spinner, shimmer), opcional
}

pub enum Outcome<I = ()> {
    Ignored,                  // no era para mí: propágalo
    Consumed,                 // lo usé, redibuja si revision cambió
    Intent(I),                // lo usé y produje una intención de alto nivel
}
```

Hay un `PasteBurst` (idea tomada de `codex-rs/tui/src/bottom_pane/paste_burst.rs`) que detecta pegados en terminales sin bracketed paste, como algunos de Windows, para que un Enter dentro del pegado no envíe el mensaje.

### 5.3 Foco y teclas

- **`FocusRing`:** una lista ordenada de `FocusId`. Tab y Shift+Tab rotan; un modal abierto captura el foco (pila de *traps*).
- **`Keymap`:** se resuelve por capas `Modal > Focused > Global`, con acciones con nombre (`"composer.submit"`) y combinaciones configurables, que se pueden serializar con la feature `serde`. Incluye presets `default`, `emacs` y `vim-lite` (solo modo normal e inserción en el composer).
- **`KeyHints`:** se genera a partir del keymap activo, para la barra de estado y la ayuda con `?`, de modo que la ayuda nunca miente.

### 5.4 Streaming incremental

Es la parte más delicada; la diseñamos a partir de `markdown_stream.rs` y `render/highlight_streaming.rs` de Codex.

```
deltas de texto ──▶ MarkdownStream ──▶ bloques cerrados (inmutables, cacheados)
                         │
                         └─▶ cola abierta (último bloque, se re-renderiza)
```

- El buffer acumula el texto en crudo. Solo se **confirma** (*commit*) hasta el último límite de bloque estable: una línea en blanco fuera de un bloque de código, o el cierre de una valla ```.
- Los bloques confirmados se parsean una vez y sus `StyledLine` envueltas se guardan en caché por ancho. Solo la **cola abierta** se vuelve a parsear con cada delta, así que el coste por delta es O(tamaño de la cola), no O(tamaño del mensaje).
- Los bloques de código abiertos se resaltan línea a línea con el estado de syntect que se conserva entre deltas (`ParseState` + `HighlightState` por línea).
- Al cambiar el ancho se reenvuelven las líneas desde los bloques guardados sin volver a parsear. Al cambiar el tema (`theme_revision`) se vuelven a resaltar.
- **Limitación de frames:** el `FrameScheduler` agrupa cambios y pide como mucho N frames por segundo (60 por defecto, 10 con movimiento reducido). Los deltas que llegan entre frames solo mutan el estado.

### 5.5 Integración con tokio

El núcleo no usa async. Con la feature `tokio`:

```rust
pub struct FrameScheduler { /* coalesce + rate limit */ }
impl FrameScheduler {
    pub fn request(&self);                       // Send + Sync, se llama desde cualquier task
    pub async fn next_frame(&mut self) -> Instant;
}

pub fn event_stream() -> impl Stream<Item = io::Result<Event>>; // crossterm EventStream normalizado
```

El patrón recomendado es un `tokio::select!` sobre `event_stream()`, el canal del agente y `scheduler.next_frame()`. El ejemplo `mock-agent` muestra el bucle completo en unas 60 líneas.

## 6. Catálogo de componentes

### 6.1 Capa genérica (`ratakit-widgets`)

| Componente | Estado (core) | Base / inspiración | Notas |
|---|---|---|---|
| `TextArea` multilínea | `TextBuffer` | Codex `bottom_pane/textarea.rs`, ratatui-textarea | Selección, undo/redo, wrap suave, bloques pegados reemplazables por marcadores `[Pasted 120 lines]` y elementos atómicos (por ejemplo una mención `@file`) |
| `VirtualList` | `ListState`, `ScrollState` | Codex `Renderable::desired_height` + `render_scrolled` | Alturas variables y cacheadas por ancho; *follow tail* (sigue el final mientras estás abajo y se suelta si subes); scrollbar |
| `Markdown` | `MarkdownStream` / `MarkdownDoc` | Codex `markdown_render.rs`, tui-markdown | Encabezados, listas, citas, tablas, enlaces OSC 8, código con resaltado |
| `CodeBlock` | `Highlighter` | Codex `render/highlight.rs` (syntect + two-face) | Números de línea, rango resaltado y lenguaje detectado por extensión o shebang |
| `DiffView` | `DiffModel` | `similar` (Codex usa `diffy`) | Modos unificado y en paralelo (*split*); resaltado intra-línea; hunks plegables; resaltado de sintaxis por lado |
| `Tree` | `TreeState<Id>` | tui-tree-widget (API inspirada, implementación propia sobre `VirtualList`) | Carga perezosa de hijos y filtrado difuso |
| `FuzzyPicker` | `PickerState<T>` + `FuzzyFilter` | nucleo-matcher, Codex `file_search_popup.rs` | Resalta los caracteres que coinciden; secciones; previsualización opcional |
| `CompletionMenu` | `CompletionState` | Codex `command_popup.rs` | Anclado al cursor de un `TextArea`; disparadores configurables (`@`, `/`, `#`) |
| `Modal` / `Dialog` | `DialogState` | — | Captura el foco; botones; variante de confirmación; centrado y con tamaño adaptable |
| `Tabs` | `TabsState` | ratatui-widgets `Tabs` | Con badge (contador) e indicador de actividad |
| `Table` | `TableState` | ratatui-widgets `Table` | Virtualizada, ordenable y con columnas auto-ajustables |
| `Spinner` | `Motion` | throbber-widgets-tui (estilo), Codex `motion.rs` | Respeta `MotionMode::Reduced` (sin animación, solo texto) |
| `StatusBar` | — | Codex `bottom_pane/footer.rs` | Segmentos izquierda, centro y derecha; truncado por prioridad |
| `Toasts` | `ToastQueue` | — | Apilables, con tiempo de vida y niveles; no roban el foco |
| `ScrollView` | `ScrollState` | tui-scrollview | Contenido arbitrario más alto que el área |
| `KeyHints` | `Keymap` | Codex `key_hint.rs` | Se genera del keymap |
| `Theme` | `Theme`, `Palette` | Codex `color.rs`, `styles.md` | Detecta fondo claro u oscuro (OSC 11); 16 colores, 256 o truecolor con degradado; temas de syntect asociados |

### 6.2 Capa de agente (`ratakit-agent`)

| Componente | Construido sobre | Notas |
|---|---|---|
| `Transcript` | `VirtualList` + celdas | Celdas: `User`, `Assistant` (en streaming), `Thinking` (plegable, plegado por defecto al terminar), `ToolCall` (estado, entrada, salida truncada, diff), `Plan`, `Notice`, `Error`. El modelo de celdas sigue a `history_cell/` de Codex |
| `ToolCallCard` | `CodeBlock`, `DiffView`, `Spinner` | Estados `Pending → InProgress → Completed / Failed / Cancelled`; salida plegable con las primeras y últimas N líneas; tipos: lectura, edición, ejecución, búsqueda, fetch u otro |
| `Composer` | `TextArea` + `CompletionMenu` + `FuzzyPicker` | `@ficheros` (con un `FileIndex` aportado por la app), `/comandos` con argumentos, historial (↑/↓ con prefijo), pegados grandes como bloque atómico, adjuntos (imágenes como marcador), cola de mensajes mientras el agente trabaja |
| `PermissionPrompt` | `Dialog` | Opciones al estilo ACP (`allow_once`, `allow_always`, `reject_once`, `reject_always`) con atajos 1–4; muestra el comando o diff afectado |
| `ModelPicker` | `FuzzyPicker` | Modelos agrupados por proveedor y esfuerzo de razonamiento (`low`, `medium`, `high`, …) como segundo eje |
| `UsageBar` | `StatusBar` | Tokens de entrada, salida y caché, % de contexto, coste, modo (plan, auto, ask) y rama de git |
| `PlanView` | `VirtualList` | Lista de tareas con estados `pending`, `in_progress` y `completed`; se puede mostrar en una celda del transcript o en un panel lateral |
| `AgentLayout` | todo lo anterior | Layout de referencia: transcript arriba, composer abajo, barra de uso y overlays (permisos, selector). Es opcional: cada pieza se puede usar por separado |

## 7. API pública de ejemplo

> Son firmas orientativas; los nombres pueden cambiar durante la implementación.

### 7.1 Núcleo

```rust
// ratakit-core
pub struct TextBuffer { /* rope, cursores, undo */ }
impl TextBuffer {
    pub fn new() -> Self;
    pub fn text(&self) -> String;
    pub fn set_text(&mut self, s: &str);
    pub fn insert_str(&mut self, s: &str);
    pub fn insert_atom(&mut self, atom: Atom);          // @mención, bloque pegado
    pub fn cursor(&self) -> Position;
    pub fn selection(&self) -> Option<Range<Position>>;
    pub fn undo(&mut self) -> bool;
    pub fn redo(&mut self) -> bool;
    pub fn revision(&self) -> u64;
}

pub struct ScrollState { offset: usize, follow_tail: bool, /* ... */ }
impl ScrollState {
    pub fn follow_tail(&self) -> bool;
    pub fn scroll_by(&mut self, delta: isize);          // al subir, se suelta del final
    pub fn scroll_to_end(&mut self);                    // vuelve a seguir el final
}

pub trait ItemHeights {                                 // lo implementa quien aporta los items
    fn len(&self) -> usize;
    fn height(&self, index: usize, width: u16) -> u16;  // cacheado por (rev, width)
}

pub struct Keymap { /* capas → mapa KeyChord → ActionId */ }
impl Keymap {
    pub fn bind(&mut self, scope: Scope, chord: KeyChord, action: ActionId);
    pub fn resolve(&self, scopes: &[Scope], key: &KeyEvent) -> Option<&ActionId>;
}

pub struct FocusRing { /* ids + pila de traps */ }
impl FocusRing {
    pub fn focused(&self) -> Option<FocusId>;
    pub fn next(&mut self);
    pub fn prev(&mut self);
    pub fn push_trap(&mut self, id: FocusId);           // modal
    pub fn pop_trap(&mut self);
}

pub struct Theme {
    pub palette: Palette,                               // colores semánticos
    pub syntax: SyntaxThemeId,
    pub motion: MotionMode,                             // Full | Reduced
    pub glyphs: GlyphSet,                               // Unicode | Ascii
}

pub trait Component {
    type Intent;
    fn handle(&mut self, ev: &Event, keys: &Keymap) -> Outcome<Self::Intent>;
    fn revision(&self) -> u64;
}
```

### 7.2 Texto

```rust
// ratakit-text
pub struct MarkdownStream { /* buffer, bloques cerrados, cola */ }
impl MarkdownStream {
    pub fn new(opts: MarkdownOptions) -> Self;
    pub fn push(&mut self, delta: &str);                // O(cola)
    pub fn finish(&mut self);
    pub fn lines(&mut self, width: u16, theme: &Theme) -> &[StyledLine]; // con caché
    pub fn source(&self) -> &str;                       // para copiar el markdown original
}

pub struct DiffModel { pub files: Vec<FileDiff> }
impl DiffModel {
    pub fn from_texts(path: &str, old: &str, new: &str) -> Self;   // similar
    pub fn from_unified(patch: &str) -> Result<Self, DiffParseError>;
}

pub trait Highlighter: Send + Sync {
    fn highlight_line(&self, state: &mut HlState, line: &str) -> Vec<StyledSpan>;
    fn language_for_path(&self, path: &Path) -> Option<LangId>;
}
```

### 7.3 Widgets genéricos

```rust
// ratakit-widgets: patrón StatefulWidget
pub struct TextArea<'a> { theme: &'a Theme, placeholder: Option<&'a str>, max_height: u16 }
impl StatefulWidget for TextArea<'_> { type State = TextAreaState; /* ... */ }

pub struct VirtualList<'a, R: RowRenderer> { rows: &'a R, theme: &'a Theme, scrollbar: bool }
pub trait RowRenderer: ItemHeights {
    fn render_row(&self, index: usize, area: Rect, buf: &mut Buffer, skip_top: u16);
}

pub struct DiffView<'a> { model: &'a DiffModel, mode: DiffMode, theme: &'a Theme }
pub enum DiffMode { Unified, SideBySide, Auto { min_width: u16 } }

pub struct FuzzyPicker<'a, T: PickerItem> { title: &'a str, theme: &'a Theme, _t: PhantomData<T> }
pub trait PickerItem { fn label(&self) -> Cow<'_, str>; fn detail(&self) -> Option<Cow<'_, str>>; }
pub enum PickerIntent<T> { Selected(T), Cancelled }

// Uso típico en el bucle de la app
frame.render_stateful_widget(TextArea::new(&theme), area, &mut app.input);
if let Outcome::Intent(TextIntent::Submit(text)) = app.input.handle(&ev, &keymap) { /* ... */ }
```

### 7.4 Capa de agente

```rust
// ratakit-agent::state (sin ratatui): modelo neutral, no atado a ACP
pub enum AgentEvent {
    UserMessage { id: MessageId, content: Vec<ContentPart> },
    AssistantDelta { id: MessageId, text: String },
    ThinkingDelta { id: MessageId, text: String },
    MessageDone { id: MessageId },
    ToolCallStarted(ToolCall),
    ToolCallUpdated { id: ToolCallId, patch: ToolCallPatch },
    PlanUpdated(Plan),
    Usage(Usage),
    ModeChanged(ModeId),
    CommandsAvailable(Vec<SlashCommand>),
    Notice { level: Level, text: String },
    Error(String),
    TurnEnded { reason: StopReason },
}

pub struct TranscriptState { /* Vec<Cell>, celda activa, revision */ }
impl TranscriptState {
    pub fn apply(&mut self, ev: AgentEvent);           // único punto de entrada
    pub fn cells(&self) -> &[Cell];
    pub fn toggle_fold(&mut self, cell: CellId);
    pub fn is_busy(&self) -> bool;
}

pub enum Cell {
    User(UserCell),
    Assistant(AssistantCell),          // contiene MarkdownStream
    Thinking(ThinkingCell),            // folded: bool
    ToolCall(ToolCallCell),            // status, kind, input, output, diff: Option<DiffModel>
    Plan(Plan),
    Notice(NoticeCell),
    Error(ErrorCell),
}

pub struct ToolCall {
    pub id: ToolCallId, pub title: String, pub kind: ToolKind,
    pub status: ToolStatus, pub locations: Vec<PathBuf>, pub content: Vec<ToolContent>,
}
pub enum ToolStatus { Pending, InProgress, Completed, Failed, Cancelled }

pub struct ComposerState { /* TextBuffer + historial + menús */ }
pub enum ComposerIntent {
    Submit { text: String, mentions: Vec<PathBuf>, attachments: Vec<Attachment> },
    SlashCommand { name: String, args: String },
    Cancel,                            // Esc con el agente ocupado = interrumpir
    OpenModelPicker,
}
pub trait FileIndex: Send + Sync {     // lo aporta la app (ignore, git ls-files...)
    fn query(&self, pattern: &str, limit: usize) -> Vec<PathBuf>;
}

pub struct PermissionRequest { pub tool_call: ToolCall, pub options: Vec<PermissionOption> }
pub enum PermissionIntent { Chose(PermissionOptionId), Dismissed }

pub struct ModelChoice { pub provider: String, pub model: String, pub effort: Option<Effort> }
pub struct Usage {
    pub input_tokens: u64, pub output_tokens: u64, pub cached_tokens: u64,
    pub context_used: Option<f32>, pub cost_usd: Option<f64>,
}

// ratakit-agent::widgets
pub struct Transcript<'a> { theme: &'a Theme }       // StatefulWidget<State = TranscriptState>
pub struct Composer<'a> { theme: &'a Theme, files: &'a dyn FileIndex }
pub struct PermissionPrompt<'a> { req: &'a PermissionRequest, theme: &'a Theme }
pub struct ModelPicker<'a> { models: &'a [ModelChoice], theme: &'a Theme }
pub struct UsageBar<'a> { usage: &'a Usage, mode: &'a str, theme: &'a Theme }
pub struct PlanView<'a> { plan: &'a Plan, theme: &'a Theme }
```

### 7.5 Adaptador ACP (`ratakit-acp`)

Usa `agent-client-protocol` 3.5.0 (esquema `agent-client-protocol-schema` 1.12, protocolo v1). La traducción de `SessionUpdate` es la siguiente:

| `SessionUpdate` (ACP v1) | `AgentEvent` |
|---|---|
| `UserMessageChunk` | `UserMessage` (se agrupa por turno) |
| `AgentMessageChunk` | `AssistantDelta` |
| `AgentThoughtChunk` | `ThinkingDelta` |
| `ToolCall` | `ToolCallStarted` |
| `ToolCallUpdate` | `ToolCallUpdated` (con diff si el contenido trae `Diff { path, old_text, new_text }`) |
| `Plan` | `PlanUpdated` |
| `AvailableCommandsUpdate` | `CommandsAvailable` (alimenta el menú `/` del composer) |
| `CurrentModeUpdate` | `ModeChanged` |
| `ConfigOptionUpdate` | actualiza `ModelChoice` y el esfuerzo si el agente los expone como opciones |
| `UsageUpdate` | `Usage` |
| `SessionInfoUpdate` | título de la sesión |
| `Notice` | `Notice` |
| `CompactionUpdate` / `CompactionSummaryChunk` | celda `Notice` "contexto compactado" |
| `PlanUpdate` / `PlanRemoved` / `SubagentUpdate` (inestables) | solo con `unstable-acp` |
| fin de `session/prompt` (`StopReason`) | `TurnEnded` |

`session/request_permission` se convierte en un `PermissionRequest` que se envía a la app por canal; la respuesta (`PermissionIntent`) vuelve como `RequestPermissionOutcome`.

```rust
pub struct AcpBridge { /* conexión, proceso hijo, canales */ }

pub struct AgentCommand { pub program: String, pub args: Vec<String>, pub env: Vec<(String, String)> }
impl AgentCommand {
    pub fn claude_code() -> Self;   // npx -y @agentclientprotocol/claude-agent-acp
    pub fn opencode() -> Self;      // opencode acp
}

impl AcpBridge {
    pub async fn spawn(cmd: AgentCommand, cwd: &Path, caps: ClientCaps) -> Result<Self, AcpError>;
    pub async fn new_session(&mut self) -> Result<SessionId, AcpError>;
    pub async fn prompt(&self, text: &str, mentions: &[PathBuf]) -> Result<(), AcpError>;
    pub async fn cancel(&self) -> Result<(), AcpError>;
    pub async fn set_mode(&self, mode: &ModeId) -> Result<(), AcpError>;
    pub fn events(&mut self) -> &mut mpsc::Receiver<BridgeEvent>;
    pub fn respond_permission(&self, id: RequestId, choice: PermissionIntent);
}

pub enum BridgeEvent { Agent(AgentEvent), Permission(RequestId, PermissionRequest), Exited(ExitStatus) }
```

Por defecto, `ClientCaps` **no** declara `fs` ni `terminal`. El agente usa sus propias herramientas y la TUI solo muestra. Exponer el sistema de ficheros del cliente es opcional y queda fuera del MVP.

Ejemplos:
- `examples/claude-acp`: necesita `npm i -g @agentclientprotocol/claude-agent-acp` (o `npx`) y la autenticación de Claude Code ya hecha.
- `examples/opencode-acp`: necesita `opencode` instalado (`opencode acp`).

Los dos comparten `examples/common/app.rs`; solo cambia el `AgentCommand`. El objetivo es que `app.rs` tenga unas 100 líneas.

## 8. Cómo se prueba

1. **Tests unitarios del estado (`ratakit-core`, `ratakit-agent::state`), sin terminal.** Por ejemplo: edición, undo, follow-tail, filtrado difuso, resolución del keymap, aplicar `AgentEvent`s y el orden de las celdas. Se escriben con `KeySeq` de `ratakit-testing`.
2. **Snapshots de render con `ratatui::backend::TestBackend` + `insta` 1.49** (el mismo enfoque que Codex, que tiene cientos de `snapshots/*.snap`):
   ```rust
   #[test]
   fn tool_call_card_failed_80x12() {
       let state = fixtures::tool_call(ToolStatus::Failed);
       assert_widget_snapshot!(ToolCallCard::new(&Theme::dark()), state, 80, 12);
   }
   ```
   `assert_widget_snapshot!` renderiza en un `Buffer`, lo serializa como texto con un apartado de estilos (colores y modificadores por celda) y llama a `insta::assert_snapshot!`. Cada widget tiene snapshots a 40, 80 y 120 columnas, en tema claro y oscuro, y en ASCII.
3. **Tests de streaming:** propiedad que comprueba que trocear un markdown en deltas aleatorios da *exactamente* las mismas líneas que renderizarlo entero (`proptest`). Es el test más importante de `MarkdownStream`.
4. **Guiones de eventos:** ficheros `.jsonl` con `AgentEvent`s grabados de sesiones reales de Claude Code y opencode a través de ACP, que se reproducen en snapshots. Sirven también como benchmarks.
5. **Tests del adaptador ACP:** con un agente falso en proceso (los traits `Agent` del propio crate) que emite un guion de `SessionUpdate`s. Las pruebas contra Claude u opencode reales son manuales o nocturnas, nunca en CI por PR.
6. **Benchmarks (`criterion`):** mide el coste por delta de `MarkdownStream` y el render de un transcript de 10 000 celdas, y falla el CI si empeora más de un 20 %.
7. **CI:** `cargo test --all-features`, `cargo test --no-default-features`, `cargo hack --feature-powerset` (acotado), `clippy -D warnings`, `cargo deny` (licencias y advisories), MSRV, y Linux, macOS y Windows.

## 9. Accesibilidad y rendimiento

### Accesibilidad

- **Movimiento reducido:** `MotionMode::Reduced` desactiva spinners animados, shimmer y transiciones, y los sustituye por texto estático ("trabajando…"). Se activa por configuración, con la variable `RATAKIT_REDUCED_MOTION`, o si se detecta un lector de pantalla (idea de `codex-rs/tui/src/screen_reader.rs`).
- **Modo lector de pantalla:** salida lineal sin cajas ni redibujados en sitio. Las celdas cerradas se escriben una vez al scrollback, que es lo que hace Codex con `insert_history.rs`, y solo la zona del composer se mantiene viva.
- **Sin depender del color:** se respeta `NO_COLOR`, y cada estado (éxito, error, pendiente) lleva un glifo y una etiqueta, no solo color. Los temas incluyen variantes de alto contraste y aptas para daltonismo, y se valida el contraste mínimo del tema en tests.
- **ASCII:** `GlyphSet::Ascii` para terminales sin Unicode o con fuentes limitadas.
- **Teclado completo:** todo se puede hacer sin ratón; el ratón es un añadido opcional. La ayuda con `?` se genera del keymap.
- **Ancho de caracteres correcto:** `unicode-width` y graphemes (emoji, CJK) en el wrapping y en el cursor.

### Rendimiento

- **Objetivo:** menos de 2 ms por frame con un transcript de 10 000 celdas y un mensaje en streaming de 50 KB, en un portátil medio, con el crate compilado en release.
- **Virtualización:** solo se renderizan las filas visibles, con las alturas cacheadas por `(revision, width)`.
- **Streaming O(cola):** ver la sección 5.4.
- **Recorte de salidas:** las salidas de herramientas se truncan al dibujar (primeras y últimas N líneas, como `TOOL_CALL_MAX_LINES` en Codex); el texto completo se queda en el estado y se puede ver en un pager.
- **Inicialización perezosa:** `SyntaxSet` y temas de two-face se cargan una vez (`OnceLock`) y solo si se usa resaltado.
- **Frames limitados:** `FrameScheduler`, a 60 fps como máximo.
- **Modo inline opcional:** como en Codex, el historial cerrado puede ir al scrollback del terminal (`Viewport::Inline`) en lugar de a pantalla alternativa, lo que reduce a casi cero el coste del historial largo y permite usar la búsqueda y la selección nativas del terminal.

## 10. Licencia y atribución a Codex

- **Licencia del proyecto:** Apache-2.0, con `LICENSE` y `NOTICE` en la raíz. Cada crate declara `license = "Apache-2.0"`.
- **Código tomado de Codex** (`openai/codex`, Apache-2.0, "Copyright 2025 OpenAI"):
  1. Se conserva la cabecera de copyright original en cada fichero derivado y se añade una línea propia:
     ```rust
     // Derived from openai/codex (codex-rs/tui/src/bottom_pane/paste_burst.rs, commit <sha>).
     // Copyright 2025 OpenAI. Licensed under the Apache License, Version 2.0.
     // Modifications Copyright 2026 The ratakit Authors.
     ```
  2. Por la sección 4(b) de Apache-2.0, se indica en el fichero que ha sido modificado.
  3. Se copia el contenido relevante del `NOTICE` de Codex al nuestro. Ese NOTICE ya incluye la atribución MIT a ratatui (Florian Dehau y The Ratatui Developers), porque `custom_terminal.rs` deriva de `ratatui::Terminal`. Si tomamos ese fichero, debemos mantener también el texto MIT completo de su cabecera.
  4. En `THIRD_PARTY.md` se lista cada fichero derivado con su ruta de origen y el commit de Codex.
- **Qué tomar de Codex** (candidatos, para adaptar quitando dependencias de `codex_*`): `paste_burst.rs`, la máquina de estados del composer (`chat_composer.rs`, `textarea.rs`), `markdown_stream.rs` y `markdown_render.rs`, `render/highlight.rs` y `highlight_streaming.rs`, `render/renderable.rs`, `wrapping.rs`/`live_wrap.rs`, `insert_history.rs`, `motion.rs`/`shimmer.rs`, `key_hint.rs` y los patrones de `history_cell/`.
- **Dependencias:** todas tienen licencias compatibles (MIT, Apache-2.0, Zlib). **`nucleo` y `nucleo-matcher` son MPL-2.0:** se pueden usar como dependencia sin problema (copyleft débil, por fichero), pero no hay que copiar su código a nuestros ficheros. Para eso las ponemos detrás de la feature `fuzzy`. `cargo deny` tendrá una lista de licencias permitidas.
- **Código de phi-tui, cornichon y agent-air:** no se copia. Como mucho sirven de referencia de diseño.

## 11. Dependencias

Versiones comprobadas en crates.io el 24 de octubre de 2026.

| Crate | Versión | Licencia | Uso | Feature |
|---|---|---|---|---|
| `ratatui-core` | 0.1.2 | MIT | Tipos base de widgets (`Buffer`, `Rect`, `Widget`) | siempre (widgets) |
| `ratatui-widgets` | 0.3.2 | MIT | `Block`, `Paragraph`, `Tabs`, `Table`, `Scrollbar` | siempre (widgets) |
| `ratatui` | 0.30.2 | MIT | Fachada, solo en ejemplos y en `ratakit-testing` (`TestBackend`) | dev / testing |
| `ratatui-crossterm` | 0.1.2 | MIT | Backend crossterm | `crossterm` |
| `crossterm` | 0.29.0 | MIT | Eventos y terminal | `crossterm` |
| `unicode-width` | 0.2.2 | MIT OR Apache-2.0 | Ancho de caracteres | siempre |
| `unicode-segmentation` | 1.13.3 | MIT OR Apache-2.0 | Graphemes | siempre |
| `textwrap` | 0.16.4 | MIT | Wrapping | `markdown` |
| `pulldown-cmark` | 0.13.5 | MIT | Parser de markdown | `markdown` |
| `syntect` | 5.3.0 | MIT | Resaltado de sintaxis | `highlight-syntect` |
| `two-face` | 0.5.2 (+bat-0.26.1) | MIT OR Apache-2.0 | Gramáticas y temas extra (los de bat) | `highlight-syntect` |
| `tree-sitter` | 0.27.1 | MIT | Alternativa de resaltado | `highlight-treesitter` |
| `tree-sitter-highlight` | 0.27.1 | MIT | Alternativa de resaltado | `highlight-treesitter` |
| `similar` | 3.2.1 | Apache-2.0 | Diffs (líneas y palabras) | `diff` |
| `nucleo-matcher` | 0.3.1 | **MPL-2.0** | Matching difuso | `fuzzy` |
| `nucleo` | 0.5.0 | **MPL-2.0** | Matcher en hilos para listas enormes | `fuzzy-async` |
| `tokio` | 1.54.1 | MIT | Async (scheduler, ACP) | `tokio`, `acp` |
| `futures` | 0.3.34 | MIT OR Apache-2.0 | `Stream` | `tokio` |
| `agent-client-protocol` | 3.5.0 | Apache-2.0 | Cliente ACP | `acp` |
| `serde` / `serde_json` | 1.0.229 / 1.0.154 | MIT OR Apache-2.0 | Temas, keymaps y guiones | `serde` |
| `thiserror` | 2.0.21 | MIT OR Apache-2.0 | Errores | siempre |
| `insta` | 1.49.0 | Apache-2.0 | Snapshots | dev |
| `throbber-widgets-tui` | 0.11.1 | Zlib | *Solo referencia de estilos*; implementamos `Spinner` nosotros para controlar el movimiento reducido | — |
| `ratatui-textarea` | 0.9.3 | MIT | *Referencia*; el `TextArea` parte del de Codex, que soporta átomos y pegados | — |
| `tui-markdown` | 0.3.12 | MIT OR Apache-2.0 | *Referencia*; no soporta streaming incremental | — |
| `tui-tree-widget` | 0.24.1 | MIT | *Referencia* de API | — |
| `tui-scrollview` | 0.6.8 | MIT OR Apache-2.0 | Se puede usar directamente o como referencia para `ScrollView` | opcional |

> **Nota:** Codex usa `diffy` para los diffs; nosotros usamos `similar` (Apache-2.0, del mismo autor que `insta`) porque da diffs por palabra y por carácter para el resaltado intra-línea.

## 12. Riesgos

| Riesgo | Impacto | Mitigación |
|---|---|---|
| **Cambios de API de ratatui** (0.x; la 0.30 dividió el crate) | Alto | Depender de `ratatui-core`, que es más estable; aislar las conversiones en un módulo `compat`; probar en CI también contra el `main` de ratatui de forma no bloqueante; versionar ratakit 0.x en sincronía, con una tabla de compatibilidad en el README |
| **ACP cambia rápido** (3.x, con protocolo v2 en el esquema y features inestables) | Medio | El adaptador está en un crate aparte; la capa de agente usa su propio `AgentEvent`; las features inestables van detrás de `unstable-acp`; se fija una versión menor |
| **Streaming incorrecto** (markdown que se rompe al trocearlo) | Alto | Test de propiedad "trocear = entero"; *commit* solo en límites estables |
| **Diferencias entre terminales** (Windows, tmux, kitty o iTerm: paste, teclas, OSC 8 y OSC 11) | Medio | `PasteBurst`; detectar capacidades y degradar; CI en Windows; una matriz de terminales probada a mano antes de cada release |
| **Alcance excesivo** (acabar haciendo un framework) | Medio | No objetivos explícitos; cada componente en su PR y con su snapshot; no tener bucle de aplicación propio |
| **Derivar demasiado de Codex** (deuda de atribución, código acoplado) | Bajo/medio | Tomar ficheros concretos y registrados en `THIRD_PARTY.md`; reescribir en lugar de copiar cuando el acoplamiento con `codex_*` sea alto |
| **Rendimiento con syntect** (regex onig vs fancy) | Medio | `fancy-regex` por defecto (sin C) y `onig` opcional; caché de resaltado por línea; resaltar solo lo visible |
| **MPL-2.0 de nucleo** | Bajo | Solo como dependencia, detrás de una feature; documentado en `cargo deny` |
| **Homónimo en crates.io** (crate `ratakit` 0.5.0 abandonado) | Bajo | Nombre del proyecto fijado en **ratakit**; coordinar publicación y nombres de crates con el mantenimiento antes del primer release |
| **Pocos mantenedores** | Medio | Una buena documentación y la galería de ejemplos para atraer contribuidores; issues "good first issue" por componente |

## 13. Nombre del proyecto

**Decisión:** el proyecto se llama **ratakit** (crates `ratakit`, `ratakit-core`, `ratakit-widgets`, `ratakit-agent`, `ratakit-acp`, `ratakit-testing`).

En crates.io hay un crate homónimo antiguo (`ratakit` 0.5.0, sin cambios desde 2022). No confundir con este workspace. La publicación en crates.io queda para el hito 5 (R4), con aprobación del mantenimiento.

Alternativas que se barajaron antes de fijar el nombre: `urdimbre`, `bastidor`, `tuiforge` (véase historial del documento).

## 14. Roadmap por hitos

Cada fila es **un PR**. "Depende de" indica qué PRs tienen que estar mergeados antes. Lo que no tiene dependencias entre sí puede ir en paralelo.

### Hito 0: base (secuencial, ~2 días)

| PR | Contenido | Depende de |
|---|---|---|
| P0.1 | Workspace, CI (test, clippy, deny, MSRV, 3 SO), `LICENSE`, `NOTICE`, `THIRD_PARTY.md`, plantilla de PR | — |
| P0.2 | `ratakit-core`: `Event`, `Outcome`, `Component`, `Keymap`, `FocusRing`, `Theme`/`Palette`/`GlyphSet`/`MotionMode`, `revision` | P0.1 |
| P0.3 | `ratakit-testing`: `assert_widget_snapshot!`, `KeySeq`, serializador de `Buffer` con estilos | P0.1 |

### Hito 1: genéricos básicos (en paralelo, ~1 semana)

| PR | Componente | Depende de |
|---|---|---|
| G1 | `TextBuffer` + `TextArea` (+ `PasteBurst`, átomos) | P0.2, P0.3 |
| G2 | `ScrollState` + `VirtualList` (follow tail, alturas cacheadas, scrollbar) | P0.2, P0.3 |
| G3 | `StyledLine` + wrapping + `Highlighter` (syntect/two-face) + `CodeBlock` | P0.2, P0.3 |
| G4 | `Spinner` + `StatusBar` + `KeyHints` + `Toasts` | P0.2, P0.3 |
| G5 | `Modal`/`Dialog` + `Tabs` + `Table` | P0.2, P0.3 |
| G6 | `FuzzyFilter` (nucleo-matcher) | P0.2 |

### Hito 2: genéricos avanzados (en paralelo, ~1 semana)

| PR | Componente | Depende de |
|---|---|---|
| G7 | `MarkdownStream` + widget `Markdown` (streaming O(cola), test de propiedad) | G3 |
| G8 | `DiffModel` (similar) + `DiffView` unificado y en paralelo | G3, G2 |
| G9 | `FuzzyPicker` | G6, G2, G5 |
| G10 | `CompletionMenu` anclado a `TextArea` | G6, G1 |
| G11 | `Tree` + `ScrollView` | G2 |
| G12 | `FrameScheduler` + `event_stream` (feature `tokio`) + ejemplo `gallery` | G1, G2, G4 |

### Hito 3: módulo de agente (en paralelo, ~1 semana)

| PR | Componente | Depende de |
|---|---|---|
| A1 | `agent::state`: `AgentEvent`, `TranscriptState`, `Cell`s, `apply()` (sin widgets) | P0.2, G7 (`MarkdownStream`), G8 (`DiffModel`) |
| A2 | Widget `Transcript` + celdas `User`/`Assistant`/`Thinking`/`Error`/`Notice` | A1, G2, G7 |
| A3 | `ToolCallCard` (estado, salida truncada, diff) | A1, G8, G3, G4 |
| A4 | `Composer` (@ficheros, /comandos, historial, pegados, cola) | G1, G10, G9 |
| A5 | `PermissionPrompt` | A1, G5 |
| A6 | `ModelPicker` (modelo + esfuerzo) | G9 |
| A7 | `UsageBar` + `PlanView` | A1, G4, G2 |
| A8 | `AgentLayout` + ejemplo `mock-agent` (guiones `.jsonl`) | A2–A7 |

### Hito 4: ACP y ejemplos reales (~4–5 días)

| PR | Contenido | Depende de |
|---|---|---|
| C1 | `ratakit-acp`: `AcpBridge`, traducción de `SessionUpdate`, permisos, tests con un agente falso | A1, A5 |
| C2 | `examples/claude-acp` + `examples/opencode-acp` (app común de ~100 líneas) | C1, A8 |
| C3 | Guiones grabados de sesiones reales → snapshots y benchmarks `criterion` | C2 |

### Hito 5: pulido y 0.1 (~1 semana)

| PR | Contenido | Depende de |
|---|---|---|
| R1 | Accesibilidad: modo lector de pantalla, alto contraste, auditoría de `MotionMode` | A8 |
| R2 | Modo inline (historial al scrollback, idea de `insert_history.rs`) | A2 |
| R3 | Documentación: guía, `docs.rs` con ejemplos, README, tabla de compatibilidad con ratatui | todo |
| R4 | Reservar crates en crates.io y release 0.1.0 (solo con la aprobación de Jordi) | R1–R3 |

**Grafo resumido:**

```
P0.1 → P0.2, P0.3 → {G1, G2, G3, G4, G5, G6}
G3 → G7, G8      G6 → G9, G10      G2 → G11      {G1,G2,G4} → G12
{G7, G8} → A1 → {A2, A3, A5, A7}      {G1,G9,G10} → A4      G9 → A6
A2..A7 → A8      {A1, A5} → C1 → C2 → C3      A8 → R1, R2 → R3 → R4
```

Con 4–6 agentes en paralelo en los hitos 1–3, la estimación total es de **3–4 semanas** hasta la 0.1.0.

## 15. Preguntas abiertas

1. ¿El modo inline (scrollback) o la pantalla alternativa por defecto? Codex usa inline; opencode y pi usan pantalla alternativa.
3. ¿Ofrecemos tree-sitter desde la v1 o solo syntect? Propuesta: solo syntect en la v1.
4. ¿Exponemos `fs` y `terminal` del cliente en el adaptador ACP (por ejemplo para revisar diffs antes de escribir)? Propuesta: después de la 0.1.
5. ¿Integración con el cliente de review "revu" (GPUI)? Podría compartir `ratakit-agent` sin la feature `widgets` y `ratakit-acp`, que no dependen de ratatui. Es un argumento más para mantener el estado sin ratatui. Si hace falta, `agent::state` se puede separar en su propio crate (`ratakit-agent-state`).
