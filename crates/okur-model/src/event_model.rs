//! A platform's events as the engine's, from what its driver recorded for each one. The live
//! driver and the corpus's replay convert through the same model, so a recorded run replays as
//! it ran. Windows' `okur_windows_model::event_trace::EventConverter` converts its trace events;
//! Linux's `okur_linux_model::convert::Converter` converts what its driver read for each AT-SPI
//! event; macOS's model will be a third.

use okur_core::event::AccessibilityEvent;

/// A platform's event model: its state carried from one record to the next (the focus as the
/// engine was told of it, the page read last), each record turned into the engine's events.
pub trait EventModel: Default {
    /// What the platform's driver records for one event.
    type Record;

    /// The engine's events for one record, in the order the engine takes them; none for a
    /// record the model passes over (a toolkit's repeat).
    fn convert(&mut self, record: &Self::Record) -> Vec<AccessibilityEvent>;
}
