//! Okur as Windows' Ease of Access knows it: its assistive-technology key, and the
//! comma-separated `Configuration` lists that make Windows start it
//! (`docs/references/startup-and-secure-screens-proposal.md`, §2.2). `okur doctor` reads
//! them; `okur-setup` writes the key.
//!
//! Windows keeps names lowercase in the per-user and per-session lists where the key and the
//! machine list are mixed case (JAWS's, on the dev box), and NVDA compares them with case:
//! Okur's name is lowercase, and names are compared without regard to case.

use crate::registry::{Hive, RegValue, Registry};

/// Okur's assistive-technology key name, `CompanyName_ProductName_v#`.
pub const AT_NAME: &str = "okur_okur_v1";

/// Where Windows keeps its accessibility settings.
pub const ACCESSIBILITY_KEY: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Accessibility";

/// The value listing the assistive technologies to start: in HKLM on the sign-in screen, in
/// HKCU after this user signs in.
pub const CONFIGURATION: &str = "Configuration";

/// Okur's assistive-technology key.
pub fn at_key() -> String {
    format!(r"{ACCESSIBILITY_KEY}\ATs\{AT_NAME}")
}

/// The names in a `Configuration` list, in order, without blanks.
pub fn names(list: &str) -> Vec<&str> {
    list.split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .collect()
}

/// Whether a `Configuration` list names `name`, in any case.
pub fn lists(list: &str, name: &str) -> bool {
    names(list).iter().any(|n| n.eq_ignore_ascii_case(name))
}

/// The list with `name` at its end, unless it is there already; every other name kept, in
/// order (JAWS's name is in it on the owner's machines).
pub fn with_name(list: &str, name: &str) -> String {
    let mut kept = names(list);
    if !kept.iter().any(|n| n.eq_ignore_ascii_case(name)) {
        kept.push(name);
    }
    kept.join(",")
}

/// The list without `name`, in any case; every other name kept, in order.
pub fn without_name(list: &str, name: &str) -> String {
    names(list)
        .into_iter()
        .filter(|n| !n.eq_ignore_ascii_case(name))
        .collect::<Vec<_>>()
        .join(",")
}

/// What the registry says of Okur's registration.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AtRegistration {
    /// The assistive-technology key's values; `None` when the key does not exist.
    pub at_values: Option<Vec<(String, RegValue)>>,
    /// HKLM's `Configuration` (the sign-in screen), when set.
    pub machine_configuration: Option<String>,
    /// HKCU's `Configuration` (after this user signs in), when set.
    pub user_configuration: Option<String>,
}

impl AtRegistration {
    /// Reads it from `registry`.
    pub fn read(registry: &dyn Registry) -> Self {
        Self {
            at_values: registry.values(Hive::LocalMachine, &at_key()),
            machine_configuration: registry.string(
                Hive::LocalMachine,
                ACCESSIBILITY_KEY,
                CONFIGURATION,
            ),
            user_configuration: registry.string(
                Hive::CurrentUser,
                ACCESSIBILITY_KEY,
                CONFIGURATION,
            ),
        }
    }

    /// Whether Windows starts Okur on the sign-in screen.
    pub fn on_sign_in_screen(&self) -> bool {
        self.machine_configuration
            .as_deref()
            .is_some_and(|list| lists(list, AT_NAME))
    }

    /// Whether Windows starts Okur after this user signs in.
    pub fn after_sign_in(&self) -> bool {
        self.user_configuration
            .as_deref()
            .is_some_and(|list| lists(list, AT_NAME))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::MemoryRegistry;

    #[test]
    fn names_are_added_and_removed_without_losing_others() {
        let jaws = "FreedomScientific_JAWS_v2025";
        assert_eq!(
            with_name(jaws, AT_NAME),
            "FreedomScientific_JAWS_v2025,okur_okur_v1"
        );
        assert_eq!(with_name("", AT_NAME), "okur_okur_v1");
        // Already there, in another case: kept as it is.
        assert_eq!(with_name("Okur_Okur_V1,osk", AT_NAME), "Okur_Okur_V1,osk");
        assert_eq!(
            without_name("freedomscientific_jaws_v2025, OKUR_OKUR_V1 ,osk", AT_NAME),
            "freedomscientific_jaws_v2025,osk"
        );
        assert_eq!(without_name(AT_NAME, AT_NAME), "");
        assert!(lists("a, Okur_okur_v1", AT_NAME));
        assert!(!lists("okur_okur_v10", AT_NAME));
    }

    #[test]
    fn the_registration_is_read_from_both_lists_and_the_key() {
        let mut registry = MemoryRegistry::new();
        assert_eq!(AtRegistration::read(&registry), AtRegistration::default());
        let _ = registry.set(
            Hive::LocalMachine,
            &at_key(),
            "StartExe",
            &RegValue::String(r"C:\Program Files\Okur\okur-sr.exe".into()),
        );
        let _ = registry.set(
            Hive::LocalMachine,
            ACCESSIBILITY_KEY,
            CONFIGURATION,
            &RegValue::String("FreedomScientific_JAWS_v2025".into()),
        );
        let _ = registry.set(
            Hive::CurrentUser,
            ACCESSIBILITY_KEY,
            CONFIGURATION,
            &RegValue::String("freedomscientific_jaws_v2025,okur_okur_v1".into()),
        );
        let read = AtRegistration::read(&registry);
        assert_eq!(read.at_values.as_ref().map(Vec::len), Some(1));
        assert!(!read.on_sign_in_screen());
        assert!(read.after_sign_in());
    }
}
