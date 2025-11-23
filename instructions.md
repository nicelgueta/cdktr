# TUI Workflow Orchestrator — Specification

Status: Draft
Target platform: terminal (cross-platform)
Language: Rust (stable)
TUI library: ratatui
Event handling: crossterm (or similar)
Concurrency / async runtime: tokio (recommended)
Inter-thread comms: flume or tokio::sync channels

Overview
--------
Build a small terminal user interface (TUI) application for a workflow orchestration tool. The UI should let operators browse workflows, inspect a workflow's DAG/steps, view step logs & status, and control execution (start/pause/cancel/retry). The UI must reuse the project's existing panel code for panel design/layout and implement the application architecture following the "flux" architecture (single source of truth, unidirectional data flow, explicit Actions → Dispatcher → Stores → Views pattern).

The whole application should be implemented as a rust crate called cdktr-tui (under crates). You can reference and overwrite the code that already exists there.

Goals
-----
- Reuse existing panel implementation and styles (you can simplify or improve panel rendering where apprpriate).
- You can use the existing code for examples of certain widgets lik lists etc.
- Have a single immutable application state (Store) updated via Actions.
- Keep side-effects (I/O, remote API, long-running work) separated from state updates and executed by Effects modules that dispatch follow-up Actions.
- Provide a responsive, keyboard-driven TUI with clear layout and accessible keyboard shortcuts.
- Small, testable codebase organized into clear modules suitable for iterative improvements.

High-level user flows
---------------------
- Browse a list of workflows (left sidebar).
- Select a workflow to view the workflow graph/steps in the main panel.
- Select a step to view logs, metadata, and controls in the detail panel.
- Watch real-time status updates reflected in the UI (via background poll or push). These should be implemented using the PrincipalAPI in cdktr_api::principal::PrincipalAPI


Architecture — "flux" (unidirectional / flux-like)
---------------------------------------------------
Core concepts and responsibilities:

- Action (enum): Small typed messages representing user intents and system events (e.g., Action::SelectWorkflow(id), Action::StartWorkflow(id), Action::WorkflowStatusUpdated(WorkflowStatus)).
- Dispatcher: Central component that accepts Actions and synchronously forwards them to registered Stores and Effects. Should be lightweight; can be a thin wrapper around channels or direct function calls.
- Store(s): Hold slices of application state and implement reducers that react to Actions to update state. Example stores:
  - WorkflowsStore: list, metadata, selected id
  - UIStore: focused panel, selections, help toggles
  - LogsStore: step logs streaming buffer
- Effects: Side-effect handlers invoked by the Dispatcher when certain Actions are dispatched (e.g., StartWorkflow triggers an Effect that calls the orchestration backend, then dispatches WorkflowStarted or WorkflowStartFailed).
- View/UI: Ratatui-based renderers (panels) that render from Store state and dispatch user Actions via the Dispatcher.
- Single Source of Truth: All UI reads state from Stores. Views do NOT mutate stores directly. Views only dispatch Actions.

Concurrency and messaging
-------------------------
- Use a bounded async channel (tokio mpsc) between the UI thread and the background task runner.
- The UI event loop (main thread) will:
  - Poll for crossterm input events.
  - Tick timers (for redraws or heartbeat).
  - Read store updates dispatched from Effects (or use a subscription mechanism).
- Background Effects run on tokio tasks, call network or disk, and dispatch Actions back to the Dispatcher.

Integration with existing panel design
--------------------------------------
- Reuse the project's existing panel modules and components. Do not duplicate visual/layout code.
- The new application should import and compose those panel components.
- If the existing panels expose a trait or API (e.g., Panel::render(area, frame, &state)), use that API and pass the portion of store state the panel needs.
- If necessary, adapt panels to read data via function arguments rather than global state — prefer passing the Store view or DTOs.

Suggested crate/module layout
-----------------------------
- src/main.rs
  - sets up terminal, tokio runtime, Dispatcher, root app loop
- src/app.rs
  - App struct: holds references to Dispatcher, Stores, UI adapter
  - main event loop
- src/ui/
  - mod.rs (UI initialization)
  - panels/  (reuse existing panel files — DO NOT reimplement visuals)
    - sidebar.rs (imported from existing code)
    - main_graph.rs (imported)
    - detail.rs (imported)
  - keyboard.rs (key bindings)
- src/actions.rs
  - Action enum and lightweight payload types
- src/dispatcher.rs
  - Dispatcher implementation (sync and async interfaces)
- src/stores/
  - mod.rs
  - workflows_store.rs
  - ui_store.rs
  - logs_store.rs
  - reducers.rs (optional helpers)
- src/effects.rs
  - Effects that perform I/O and call backend APIs
- src/models/
  - workflow.rs, step.rs, status.rs
- src/api/
  - backend_client.rs (abstraction over orchestration backend)
- tests/
  - unit tests for reducers and effects (mock API)
  - UI integration smoke tests (non-interactive)

Action examples
---------------
- UI Actions:
  - SelectWorkflow(WorkflowId)
  - FocusPanel(PanelId)
  - ToggleHelp
- Command Actions:
  - StartWorkflow(WorkflowId)
  - PauseWorkflow(WorkflowId)
  - CancelWorkflow(WorkflowId)
  - RetryStep(WorkflowId, StepId)
- System/Effect Actions (usually emitted by Effects):
  - WorkflowListLoaded(Vec<WorkflowMeta>)
  - WorkflowStatusUpdated(WorkflowId, WorkflowStatus)
  - WorkflowActionFailed(Action, Error)
  - StepLogsAppended(WorkflowId, StepId, Vec<LogLine>)

Store design notes
------------------
- Stores expose read-only accessors (cloned snapshots or Arc<RwLock<T>>) for UI to render.
- Reducers are pure functions (Action × State -> State). Side effects do not happen inside reducers.
- Keep store state small and serializable (helpful for tests).

Effects and backend
-------------------
- Effects must be pure I/O layers: call backend_client, stream logs, handle reconnection, then dispatch result Actions.
- Design the backend_client as a trait to allow mocking in tests.
- For streaming logs/status, Effects can spawn tokio tasks that push StepLogsAppended and WorkflowStatusUpdated actions.

UI layout & panels (recommended)
-------------------------------
- Top: Header (app name, connection status, current workflow)
- Left: Sidebar (workflows list, filter/search) — reuse existing sidebar panel
- Center: Main panel (workflow graph or step list) — graph visualization or simple list
- Right / Bottom: Details panel (selected step metadata, logs, controls)
- Bottom: Footer (keyboard shortcuts / app status)
- Panels should be individually composable so the existing panel module is plugged into the layout.

Keyboard mapping (suggested)
----------------------------
- j / k — move selection in list
- h / l — switch focus panels
- Enter — open/select
- s — start workflow
- p — pause workflow
- c — cancel workflow
- r — retry step
- / — search
- ? — help

Testing
-------
- Unit test reducers: dispatch actions against initial state and assert expected state.
- Effects: use a mock backend_client to simulate success/failure and assert that proper Actions are dispatched.
- UI smoke test: run the TUI in headless mode (if possible) and assert no panics; otherwise test rendering helpers independently.

Acceptance criteria
-------------------
- Can list workflows and select one.
- Main panel shows workflow steps (graph or list).
- Detail panel shows step metadata and recent logs.
- User can start/pause/cancel a workflow from the UI and see status updates.
- UI reuses existing panel code for visuals/layout.
- The code follows the flux architecture: Actions -> Dispatcher -> Stores -> Views and Effects for side effects.
- Reducers are pure; Effects handle I/O and dispatch actions back.

Developer notes / Implementation guidance
----------------------------------------
- Start by wiring the Dispatcher, Action enum, and a minimal WorkflowsStore.
- Plug existing sidebar panel into the layout and make it dispatch SelectWorkflow actions.
- Implement a simple mock backend and Effects that return static WorkflowListLoaded for initial UI population.
- Iterate: once UI is working with static data, add real backend calls and streaming updates.
- Keep effects cancellable and robust to backend errors; dispatch explicit failure actions and surface messages in UIStore.
- Use structured logging (tracing) for visibility while developing.

Deliverables
------------
- SPEC.md (this doc)
- Minimal runnable TUI app that composes existing panels, supports browsing workflows and viewing details, and demonstrates flux flow through at least one command (e.g., start workflow -> effect -> status update).

Appendix: Example minimal Action enum (conceptual)
-------------------------------------------------
- enum Action {
    SelectWorkflow(WorkflowId),
    StartWorkflow(WorkflowId),
    WorkflowListLoaded(Vec<WorkflowMeta>),
    WorkflowStatusUpdated(WorkflowId, WorkflowStatus),
    StepLogsAppended(WorkflowId, StepId, Vec<LogLine>),
    ShowError(String),
    // ...
  }

Respect the existing panel API and adapt store/dispatch integration to pass the right DTOs to panel renderers instead of letting panels directly query Stores.

If anything in this spec seems mismatched with the existing panel APIs you mentioned, paste or point me to the panel module path(s) and I will adapt the spec to match those exact types and function names.