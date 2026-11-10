//! Windows' accessibility data as data, read the same way live and in replay: the trace
//! format and its conversion into the engine's events (`event_trace`), UI Automation and
//! IAccessible2 snapshots and their mapping to the AOM (`uia_snapshot`, `ia2_snapshot`), the
//! ids an AOM node keeps of its platform object (`node_ids`), what the harness keeps out of a
//! recording (`privacy`), and the window classes and control texts the conversion knows
//! (`classes`, `text`). Nothing here calls Windows; `okur-windows` reads the screen into these
//! types, and parity replays recordings of them (docs/references/crate-structure-proposal.md).

pub mod classes;
pub mod element_info;
pub mod event_trace;
pub mod ia2_snapshot;
pub mod node_ids;
pub mod privacy;
pub mod text;
pub mod uia_snapshot;
pub mod window_walk;
