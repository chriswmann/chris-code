# Module map

A reference for translating the architecture diagram into code structure.
Each section describes one Rust module: what it owns, what it exposes publicly,
and how it relates to other modules.

---

## `main.rs`

The entry point. Responsible for wiring everything together and running the
main event loop. Owns nothing persistently; it constructs the pieces and
hands off ownership.

### Responsibilities

- Initialise the terminal (crossterm alternate screen, raw mode)
- Construct `AppState`
- Create the MPSC channel
- Spawn worker threads, passing each a `Sender` clone
- Run the event loop
- Restore the terminal on exit (even on panic - use a guard)

### Pseudocode shape

```
fn main()
    terminal = init_terminal()
    state = AppState::new()
    (tx, rx) = mpsc::channel::<AppEvent>()

    spawn input_thread(tx.clone())
    spawn llm_thread(tx.clone())

    loop
        event = rx.recv()
        events::handle(&mut state, event)
        terminal.draw(|f| ui::render(f, &state))
        if state.mode == Mode::Exiting { break }

    restore_terminal(terminal)
```

---

## `state.rs`

Owns the canonical definition of `AppState` and all types that live inside it.
No logic - only data structures. Every other module that needs to read or
write application state imports from here.

### Public types

```
pub struct AppState
    messages: Vec<Message>
    streaming_response: Option<String>
    mode: Mode

pub enum Mode
    Running
    Exiting

pub enum Message
    User(String)
    Agent(String)
    ToolCall(ToolCall)
    ToolResponse(ToolResponse)

pub struct ToolCall
    tool_call_id: String
    tool_name: String
    tool_input: serde_json::Value

pub struct ToolResponse
    tool_call_id: String
    tool_name: String
    content: serde_json::Value

impl AppState
    pub fn new() -> Self
```

---

## `events.rs`

Defines the `AppEvent` type hierarchy - the complete set of things that can
happen in the application - and the `handle` function that maps events onto
state mutations.

No I/O, no rendering. Pure logic: given an event and mutable state, produce
updated state.

### Public types

```
pub enum AppEvent
    Input(InputEvent)
    Llm(LlmEvent)
    App(AppCommand)

pub enum InputEvent
    Key(KeyEvent)          // re-exported from crossterm
    Resize(u16, u16)

pub enum LlmEvent
    StreamStart
    TokenReceived(String)
    StreamComplete
    StreamError(String)
    ToolCallRequested(ToolCall)
    ToolResponseReady(ToolResponse)

pub enum AppCommand
    Quit
```

### Public functions

```
pub fn handle(state: &mut AppState, event: AppEvent)
    // pattern matches on AppEvent variant
    // delegates to private helpers, e.g.:
    //   handle_input(state, event)
    //   handle_llm(state, event)
    //   handle_app(state, event)
```

### Key behaviours to implement per variant

- `LlmEvent::StreamStart` - initialise `state.streaming_response = Some(String::new())`
- `LlmEvent::TokenReceived(t)` - append `t` to `state.streaming_response`
- `LlmEvent::StreamComplete` - move `streaming_response` into `Message::Agent`, push to `messages`, set `streaming_response = None`
- `LlmEvent::StreamError(e)` - clear `streaming_response`, optionally push an error message
- `InputEvent::Key(k)` - match on key; `Ctrl+C` / `q` sends `AppCommand::Quit`
- `AppCommand::Quit` - set `state.mode = Mode::Exiting`

---

## `ui.rs`

Contains the render function. Reads `AppState` immutably and draws to the
Ratatui frame. No mutation, no I/O beyond the terminal write that Ratatui
manages.

### Public functions

```
pub fn render(frame: &mut Frame, state: &AppState)
    // computes layout from frame.area()
    // delegates to component functions
```

### Private helpers (one per UI component)

```
fn render_messages(frame: &mut Frame, area: Rect, state: &AppState)
    // iterates state.messages
    // renders streaming_response as in-progress message if Some

fn render_input_bar(frame: &mut Frame, area: Rect, state: &AppState)
    // renders the user input area at the bottom
```

### Layout sketch

```
┌─────────────────────────┐
│                         │  <- messages area (most of screen)
│  render_messages()      │
│                         │
├─────────────────────────┤
│  render_input_bar()     │  <- fixed height at bottom
└─────────────────────────┘
```

---

## `input.rs`

The input worker thread. Reads raw crossterm events in a blocking loop and
forwards them as `AppEvent::Input` variants into the channel. Contains no
application logic - it is a translator between crossterm and the event system.

### Public functions

```
pub fn run(tx: Sender<AppEvent>)
    loop
        event = crossterm::event::read()
        tx.send(AppEvent::Input(event.into()))
        // thread exits when tx.send returns Err (receiver dropped)
```

---

## `llm.rs`

The LLM worker thread. Manages the HTTP connection to the Anthropic API,
drives the streaming response loop, and dispatches tool calls. Sends
`AppEvent::Llm` variants into the channel as events occur.

This module will grow as tool support expands. Keep the API call logic and
tool dispatch logic in separate private functions from the start.

### Public functions

```
pub fn run(tx: Sender<AppEvent>, /* api config */)
    loop
        // wait for a trigger (e.g. a oneshot channel from the input thread
        // signalling that the user submitted a message)
        // call API, stream response
        // for each token: tx.send(AppEvent::Llm(LlmEvent::TokenReceived(t)))
        // on tool call: tx.send(AppEvent::Llm(LlmEvent::ToolCallRequested(tc)))
        //               execute tool
        //               tx.send(AppEvent::Llm(LlmEvent::ToolResponseReady(tr)))
        // on complete:  tx.send(AppEvent::Llm(LlmEvent::StreamComplete))
        // on error:     tx.send(AppEvent::Llm(LlmEvent::StreamError(e)))
```

### Note on triggering LLM calls

The LLM thread needs to know when the user has submitted a message.
One clean approach: a second channel (a `std::sync::mpsc` oneshot-style, or simply another `Sender<String>`) that the event handler writes to when it processes a `Return` keypress with a non-empty input buffer.
    The LLM thread blocks on this channel between calls.

---

## `tools.rs`

Defines the tool dispatch logic. Called by `llm.rs` when a `ToolCall` is received. Each tool is a function matching a common signature.

Start with a simple match on `tool_name`. Graduate to a trait or registry when the number of tools justifies it.

### Pseudocode shape

```
pub fn execute(call: &ToolCall) -> ToolResponse
    match call.tool_name.as_str()
        "read_file"    => tools::read_file(call)
        "write_file"   => tools::write_file(call)
        "shell"        => tools::shell(call)
        _              => ToolResponse::error("unknown tool")
```

---

## Dependency graph

```
main
 ├── state        (data only, no deps on other app modules)
 ├── events       (depends on state)
 ├── ui           (depends on state)
 ├── input        (depends on events)
 ├── llm          (depends on events, tools)
 └── tools        (depends on state)
```

`state.rs` sits at the base with no internal dependencies. Everything else depends on it, but nothing except `events.rs` mutates it.
