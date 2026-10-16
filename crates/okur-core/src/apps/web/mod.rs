//! Web applications: known by the address of the page in front ([`super::AppModule::owns_page`]),
//! in any browser on any platform.

pub mod google_docs;

use super::AppModule;

/// The web application modules.
pub fn modules() -> Vec<Box<dyn AppModule>> {
    vec![Box::new(google_docs::GoogleDocs)]
}
