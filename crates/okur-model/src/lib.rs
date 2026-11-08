//! What every platform's model shares: the parts of turning a platform's accessibility data into
//! the engine's events that are the same on Windows, Linux and macOS
//! (docs/references/linux-and-structure-review-2026-11-08.md, the structure's recommendation 5).
//! A platform's model (okur-windows-model, okur-linux-model) depends on this crate and on no other
//! platform's model.

pub mod element;
pub mod event_model;
pub mod snapshot;
pub mod web;
