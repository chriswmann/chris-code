# Architecture Decision Record

A record of the key structural decisions made in the initial design of this
application, the alternatives considered, and the consequences of each choice.

---

## ADR-001 - Main thread owns the terminal and runs the event loop

**Status:** Accepted

**Context**

The application needs to handle three concurrent concerns: reading terminal
input, communicating with the LLM API, and rendering the UI. Ratatui's
`Terminal` struct is not `Send`, meaning it cannot be moved to a background
thread. Terminal signal handling (e.g. `SIGWINCH` for resize events) also
interacts most cleanly with the main thread on Unix systems.

**Decision**

The main thread owns the `Terminal`, runs the render loop, and also drives
the event loop via a blocking `recv()` call on the MPSC channel receiver.
Input handling and LLM communication are delegated to worker threads.

**Alternatives considered**

Splitting input handling and rendering into separate threads was considered.
This is unnecessary because Ratatui redraws the full terminal on every frame
anyway, and the event loop is non-blocking between frames. A single loop on
the main thread is simpler and avoids the `Terminal: !Send` problem entirely.

**Consequences**

The main thread must never block. Any operation that could take more than a
frame's time (API calls, file I/O, tool execution) must happen in a worker
thread and communicate results back via the channel. Violating this will cause
the UI to freeze.

---

## ADR-002 - MPSC channel as the single communication primitive

**Status:** Accepted

**Context**

Multiple threads need to send information to the main thread: keypress events
from the input thread, token and tool events from the LLM thread. The main
thread needs to receive from all of these without blocking on any one source.

**Decision**

A single `std::sync::mpsc` channel carries all inter-thread communication to
the main thread. Each worker thread holds a `Sender<AppEvent>` clone. The
main thread holds the sole `Receiver<AppEvent>` and calls `recv()` in the
event loop, blocking until any event arrives from any sender.

**Alternatives considered**

`Arc<Mutex<AppState>>` shared across threads was considered. This would allow
worker threads to mutate state directly without a channel. It was rejected
because it makes the flow of mutations implicit and hard to trace - any thread
can change anything at any time. The channel model makes data flow explicit
and unidirectional.

Tokio's async runtime with `tokio::sync::mpsc` was considered. Rejected for
the initial version to avoid the complexity of an async runtime on the main
thread interacting with Ratatui's synchronous rendering model. Can be
revisited if the LLM thread's blocking HTTP calls become a bottleneck.

**Consequences**

All events are serialised through one channel. The main thread processes them
one at a time, which simplifies state mutation (no locks needed on `AppState`)
but means that a slow event handler could delay rendering. Event handler
functions must remain fast. Any slow work must be offloaded before sending
the event.

---

## ADR-003 - AppState is owned exclusively by the main thread

**Status:** Accepted

**Context**

Application state needs to be readable by the render function and writable by
the event handler. Both of these run on the main thread.

**Decision**

`AppState` is a plain struct with no `Arc`, no `Mutex`, and no interior
mutability. It lives on the stack of the main thread's event loop. The event
handler receives `&mut AppState` and the render function receives `&AppState`.
Worker threads never hold a reference to `AppState`.

**Alternatives considered**

`Arc<Mutex<AppState>>` cloned into worker threads was considered, allowing
them to push tokens directly into the message list. Rejected because it
requires locking on every token (potentially hundreds per second during
streaming), and because it means UI state can change between the event handler
and the render call in ways that are hard to reason about.

**Consequences**

Worker threads must represent everything they want to communicate as an
`AppEvent` message rather than a direct state mutation. This is slightly more
verbose but makes the application's data flow completely legible: the only
place `AppState` changes is inside `events::handle()`.

---

## ADR-004 - Elm-like unidirectional data flow

**Status:** Accepted

**Context**

As the UI grows in complexity (new panels, modals, overlays), a model is
needed for how UI components interact with state. Without discipline, render
functions begin to contain logic, event handlers begin to contain layout
calculations, and the two concerns become entangled.

**Decision**

A strict separation is enforced: `events::handle()` is the only function
that mutates `AppState`. `ui::render()` is a pure function of `AppState` -
it reads state and draws, and does nothing else. UI components do not send
messages; they receive state. This is the Elm architecture applied to a TUI.

**Alternatives considered**

Component-local state (each UI component owns its own mutable state) was
considered. Rejected because it makes it harder to reason about what the
application will look like given a particular sequence of events - state is
scattered rather than centralised.

**Consequences**

Adding a new UI feature requires two things: a new field or variant in
`AppState` to represent the new state, and a new branch in `ui::render()` to
draw it. Adding new behaviour requires a new `AppEvent` variant and a new
branch in `events::handle()`. These are the only two places to look when
debugging or extending the application.

---

## ADR-005 - Streaming LLM responses held in a dedicated field

**Status:** Accepted

**Context**

The LLM API returns responses as a stream of tokens. A completed response
belongs in `state.messages` as a `Message::Agent`. But during streaming,
tokens arrive one at a time and the response is not yet complete. The render
loop runs on every event, including every `TokenReceived` event, so it must
have something to display at all times during a stream.

**Decision**

`AppState` contains a `streaming_response: Option<String>` field alongside
`messages`. When a stream starts, the field is initialised to `Some(String::new())`.
Each token is appended. When the stream completes, the string is moved into
`Message::Agent`, pushed to `messages`, and the field is reset to `None`.
The render function checks `streaming_response` and displays it as an
in-progress message when `Some`.

**Alternatives considered**

Pushing a `Message::Agent` with empty content at stream start and mutating it
in place (using an index into the `messages` vec) was considered. Rejected
because mutating a vec element by index is awkward in Rust and requires either
unsafe code or a `RefCell`. The dedicated field is cleaner.

**Consequences**

There is a clear distinction in state between "a message that is being
received" and "a message that is complete". The render function can style them
differently. The event handler has a clear lifecycle: `StreamStart` →
`TokenReceived` (many) → `StreamComplete` or `StreamError`.

---

## ADR-006 - Tool dispatch via match in a dedicated module

**Status:** Accepted

**Context**

The LLM can request tool calls (file reads, shell commands, etc.). These need
to be executed and their results returned to the API. As the number of tools
grows, the dispatch logic could become unwieldy if left inline in `llm.rs`.

**Decision**

Tool dispatch lives in a dedicated `tools.rs` module. Initially implemented
as a `match` on `tool_name`. The `execute()` function takes a `&ToolCall` and
returns a `ToolResponse`.

**Alternatives considered**

A trait object registry (`HashMap<String, Box<dyn Tool>>`) was considered for
extensibility. Deferred - it adds complexity before there are enough tools to
justify it. The `match` approach is simpler to understand and refactor later.

**Consequences**

Adding a new tool means adding a variant to the `match` in `tools.rs` and
implementing a private function. This is a single, obvious place to look.
When the number of tools grows large enough that the `match` becomes unwieldy,
migration to a registry is straightforward.
