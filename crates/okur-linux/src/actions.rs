//! What acts on a page's elements for the engine on Linux: Enter or Space on a link, a button,
//! a check box activates it (its AT-SPI Action interface's first action, Chrome's "click"), and
//! focusing an element gives it the keyboard focus (its Component interface's GrabFocus). A page
//! node finds its object by its unique id, its place in the tree the event source read and
//! numbered ([`crate::events::PageObjects`]).
//!
//! The calls on the bus run on a thread of their own, in the order asked: the engine asks from
//! the keyboard hook's callback, with the key held by the X server until it answers, and a slow
//! application made the hook's watchdog replay the key, activating the element twice (the code
//! review of 2026-11-08). An action is taken when its object is known; a call that fails is
//! logged.

use std::sync::mpsc::{channel, Sender};

use okur_core::actions::ApplicationActions;
use okur_core::aom::{AomNode, UNIQUE_ID_ATTR};
use xa11y_linux::LinuxProvider;

use crate::events::PageObjects;

/// A call on the bus for the worker: an object's bus name and path.
enum Request {
    Focus(String, String),
    Activate(String, String),
}

pub struct LinuxActions {
    objects: PageObjects,
    /// The worker's queue; none when its thread could not start.
    worker: Option<Sender<Request>>,
}

impl LinuxActions {
    /// Actions on the objects of the pages `objects` holds, as the event source reads them.
    pub fn new(objects: PageObjects) -> Self {
        let (requests, received) = channel::<Request>();
        let worker = std::thread::Builder::new()
            .name("okur-linux-actions".to_owned())
            .spawn(move || {
                let provider = match LinuxProvider::new() {
                    Ok(provider) => provider,
                    Err(e) => {
                        tracing::warn!("Linux actions: no accessibility bus: {e}");
                        return;
                    }
                };
                for request in received {
                    match request {
                        Request::Focus(bus, path) => {
                            if let Err(e) = provider.atspi_grab_focus(&bus, &path) {
                                tracing::debug!("focus not given: {e}");
                            }
                        }
                        Request::Activate(bus, path) => {
                            if let Err(e) = provider.atspi_do_action(&bus, &path, 0) {
                                tracing::debug!("not activated: {e}");
                            }
                        }
                    }
                }
            })
            .map_err(|e| tracing::warn!("Linux actions: no worker thread: {e}"))
            .ok()
            .map(|_| requests);
        Self { objects, worker }
    }

    /// Asks the worker for `request` on `node`'s object: whether it was asked.
    fn ask(&self, node: &AomNode, request: fn(String, String) -> Request) -> bool {
        let (Some(worker), Some((bus, path))) = (self.worker.as_ref(), self.object(node)) else {
            return false;
        };
        worker.send(request(bus, path)).is_ok()
    }

    /// The object of `node`: its application's bus name and its path.
    fn object(&self, node: &AomNode) -> Option<(String, String)> {
        let id: usize = node.attributes.get(UNIQUE_ID_ATTR)?.parse().ok()?;
        let objects = self.objects.lock().ok()?;
        objects.get(id.checked_sub(1)?).cloned()
    }
}

impl ApplicationActions for LinuxActions {
    fn focus(&self, node: &AomNode) -> bool {
        self.ask(node, Request::Focus)
    }

    fn activate(&self, node: &AomNode) -> bool {
        self.ask(node, Request::Activate)
    }
}
