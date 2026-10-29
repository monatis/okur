use okur_core::{AomNode, NodeStates, Role};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use xa11y::{App, AppExt, Element, Role as Xa11yRole, Toggled};

#[derive(Debug, thiserror::Error)]
pub enum PlatformError {
    #[error("xa11y error: {0}")]
    Xa11y(#[from] xa11y::Error),
    #[error("No accessible window found")]
    NoWindowFound,
    #[error("No accessible document found in application")]
    NoDocumentFound,
    #[error("Application not found: {0}")]
    AppNotFound(String),
    #[error("Keyboard hook error: {0}")]
    HookError(String),
    #[error("Event listener error: {0}")]
    EventListenerError(String),
    #[error("Audio error: {0}")]
    AudioError(String),
    #[error("Unsupported platform: {0}")]
    Unsupported(String),
    #[error("Recognition error: {0}")]
    Recognition(String),
}

static NEXT_NODE_ID: AtomicUsize = AtomicUsize::new(1);

fn alloc_node_id() -> usize {
    NEXT_NODE_ID.fetch_add(1, Ordering::Relaxed)
}

/// Bridge layer converting live OS accessibility trees into semantic AOM structures
/// and dispatching accessibility commands back to the operating system.
pub struct PlatformBridge;

impl PlatformBridge {
    pub fn new() -> Self {
        Self
    }

    /// Queries the application currently holding system foreground.
    pub fn get_foreground_app(&self, timeout: Duration) -> Result<App, PlatformError> {
        App::foreground(timeout).map_err(PlatformError::Xa11y)
    }

    /// Resolves a running application by process or executable name.
    pub fn get_app_by_name(&self, name: &str, timeout: Duration) -> Result<App, PlatformError> {
        App::by_name(name, timeout).map_err(PlatformError::Xa11y)
    }

    /// Retrieves the active document as an AOM node tree from a running application.
    pub fn get_active_document(&self, app_name: Option<&str>) -> Result<AomNode, PlatformError> {
        let timeout = Duration::from_millis(1500);
        let app = if let Some(name) = app_name {
            self.get_app_by_name(name, timeout)?
        } else {
            self.get_foreground_app(timeout)?
        };

        // Locate document or web area within the application
        let web_area = app.locator(r#"web_area"#).first().element();
        let target_element = match web_area {
            Ok(el) => el,
            Err(_) => return Err(PlatformError::NoDocumentFound),
        };

        self.element_to_aom(&target_element)
    }

    /// Recursively converts an `xa11y::Element` into an `okur_core::AomNode`.
    pub fn element_to_aom(&self, element: &Element) -> Result<AomNode, PlatformError> {
        let id = alloc_node_id();
        let role = map_role(element.role);
        let mut node = AomNode::new(id, role);

        node.name = element.name.clone();
        node.value = element.value.clone();

        // Map live accessibility states
        node.states = NodeStates {
            checked: match element.states.checked {
                Some(Toggled::On) => Some(true),
                Some(Toggled::Off) => Some(false),
                _ => None,
            },
            selected: if element.states.selected {
                Some(true)
            } else {
                None
            },
            expanded: None,
            focused: element.states.focused,
            disabled: !element.states.enabled,
            hidden: false,
            modal: false,
            focusable: Some(element.states.focusable),
            pressed: false,
        };

        // StaticText nodes surface text in their name property
        if element.role == Xa11yRole::StaticText {
            node.text = element.name.clone();
        }

        // Recursively query children through OS accessibility APIs
        match element.children() {
            Ok(children) => {
                for child in children {
                    if let Ok(child_node) = self.element_to_aom(&child) {
                        node.add_child(child_node);
                    }
                }
            }
            Err(e) => {
                tracing::debug!(
                    "Could not query children for element {:?}: {}",
                    element.role,
                    e
                );
            }
        }

        Ok(node)
    }

    /// Invokes default action on the live element via OS accessibility API.
    pub fn press(&self, element: &Element) -> Result<(), xa11y::Error> {
        element.press()
    }

    /// Sets keyboard focus to the live element via OS accessibility API.
    pub fn focus(&self, element: &Element) -> Result<(), xa11y::Error> {
        element.focus()
    }

    /// Toggles state of a checkbox or switch via OS accessibility API.
    pub fn toggle(&self, element: &Element) -> Result<(), xa11y::Error> {
        element.toggle()
    }

    /// Sets the value of an editable field via OS accessibility API.
    pub fn set_value(&self, element: &Element, val: &str) -> Result<(), xa11y::Error> {
        element.set_value(val)
    }

    /// Types text into the focused element via OS accessibility API.
    pub fn type_text(&self, element: &Element, text: &str) -> Result<(), xa11y::Error> {
        element.type_text(text)
    }
}

impl Default for PlatformBridge {
    fn default() -> Self {
        Self::new()
    }
}

fn map_role(role: Xa11yRole) -> Role {
    match role {
        Xa11yRole::WebArea | Xa11yRole::Window | Xa11yRole::Application => Role::Document,
        Xa11yRole::Heading => Role::Heading { level: 1 },
        Xa11yRole::StaticText => Role::Text,
        Xa11yRole::Link => Role::Link { href: None },
        Xa11yRole::Button => Role::Button,
        Xa11yRole::CheckBox => Role::CheckBox,
        Xa11yRole::TextField | Xa11yRole::TextArea => Role::Edit,
        Xa11yRole::Table => Role::Table,
        Xa11yRole::TableRow => Role::TableRow,
        Xa11yRole::TableCell => Role::TableCell,
        Xa11yRole::Tab => Role::Tab,
        Xa11yRole::TabGroup => Role::TabList,
        Xa11yRole::List => Role::List,
        Xa11yRole::ListItem => Role::ListItem,
        Xa11yRole::Navigation => Role::Landmark {
            landmark_type: "navigation".to_string(),
        },
        _ => Role::Generic,
    }
}
