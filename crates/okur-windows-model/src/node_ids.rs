//! The ids an AOM node keeps of the platform object it was read from, so an action can find
//! the object again: a page read through IA2 keeps each object's `uniqueID`
//! (`okur_core::aom::UNIQUE_ID_ATTR`), one read through UI Automation its RuntimeId.

use okur_core::aom::UNIQUE_ID_ATTR;

/// AOM attribute holding a node's UIA RuntimeId (comma-separated), for a page read through
/// UIA, whose nodes have no IA2 `uniqueID`.
pub const UIA_RUNTIME_ID_ATTR: &str = "uia_runtime_id";

/// The UIA RuntimeId recorded on a node, if any.
pub fn runtime_id(node: &okur_core::AomNode) -> Option<Vec<i32>> {
    node.attributes
        .get(UIA_RUNTIME_ID_ATTR)?
        .split(',')
        .map(|part| part.parse().ok())
        .collect()
}

/// The IA2 `uniqueID` recorded on a node, if any.
pub fn unique_id(node: &okur_core::AomNode) -> Option<i32> {
    node.attributes.get(UNIQUE_ID_ATTR)?.parse().ok()
}
