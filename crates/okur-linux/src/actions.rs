//! What acts on a page's elements for the engine on Linux: Enter or Space on a link, a button,
//! a check box activates it (its AT-SPI Action interface's first action, Chrome's "click"), and
//! focusing an element gives it the keyboard focus (its Component interface's GrabFocus). A page
//! node finds its object by its unique id, its place in the tree the event source read and
//! numbered ([`crate::events::PageObjects`]).

use okur_core::actions::ApplicationActions;
use okur_core::aom::{AomNode, UNIQUE_ID_ATTR};
use xa11y_linux::LinuxProvider;

use crate::events::PageObjects;

pub struct LinuxActions {
    provider: Option<LinuxProvider>,
    objects: PageObjects,
}

impl LinuxActions {
    /// Actions on the objects of the pages `objects` holds, as the event source reads them.
    pub fn new(objects: PageObjects) -> Self {
        let provider = LinuxProvider::new()
            .map_err(|e| tracing::warn!("Linux actions: no accessibility bus: {e}"))
            .ok();
        Self { provider, objects }
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
        let (Some(provider), Some((bus, path))) = (self.provider.as_ref(), self.object(node))
        else {
            return false;
        };
        provider
            .atspi_grab_focus(&bus, &path)
            .map_err(|e| tracing::debug!("focus not given: {e}"))
            .unwrap_or(false)
    }

    fn activate(&self, node: &AomNode) -> bool {
        let (Some(provider), Some((bus, path))) = (self.provider.as_ref(), self.object(node))
        else {
            return false;
        };
        provider
            .atspi_do_action(&bus, &path, 0)
            .map_err(|e| tracing::debug!("not activated: {e}"))
            .is_ok()
    }
}
