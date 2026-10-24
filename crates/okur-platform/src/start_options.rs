//! Where Okur's start after sign-in is chosen (`docs/references/startup-and-secure-screens-proposal.md`
//! §5.4, §5.5): the user's choice in `HKCU\SOFTWARE\Okur\StartAfterSignIn` (`always`, `never`,
//! `all-users`; none is JAWS's default, Always), which the Startup Wizard changes; and the
//! machine's "for all users", `HKLM\SOFTWARE\Okur\StartAfterSignInAllUsers`, which setup
//! writes. The sign-in start reads both before anything else ([`UserStart::starts_after_sign_in`]).

use std::sync::Mutex;

use okur_core::settings::{StartOptions, UserStart};

use crate::registry::{Hive, RegValue, Registry, SystemRegistry};

/// Okur's key, the machine's and the user's.
pub const KEY: &str = r"SOFTWARE\Okur";
/// The user's choice.
pub const USER_VALUE: &str = "StartAfterSignIn";
/// The machine's "for all users": a DWORD, 1 on.
pub const ALL_USERS_VALUE: &str = "StartAfterSignInAllUsers";

/// The start options in a registry: Windows' for Okur, a memory one for tests.
pub struct RegistryStartOptions {
    registry: Mutex<Box<dyn Registry + Send>>,
}

impl RegistryStartOptions {
    pub fn new(registry: Box<dyn Registry + Send>) -> Self {
        Self {
            registry: Mutex::new(registry),
        }
    }

    /// Windows' registry.
    pub fn system() -> Self {
        Self::new(Box::new(SystemRegistry))
    }

    /// Whether the machine starts Okur after every user's sign-in.
    pub fn all_users(&self) -> bool {
        self.registry
            .lock()
            .ok()
            .and_then(|r| r.value(Hive::LocalMachine, KEY, ALL_USERS_VALUE))
            .and_then(|v| v.as_dword())
            .is_some_and(|on| on != 0)
    }

    /// Whether a start at this user's sign-in goes on.
    pub fn starts_after_sign_in(&self) -> bool {
        UserStart::starts_after_sign_in(self.user_start(), self.all_users())
    }
}

impl StartOptions for RegistryStartOptions {
    fn user_start(&self) -> Option<UserStart> {
        self.registry
            .lock()
            .ok()
            .and_then(|r| r.string(Hive::CurrentUser, KEY, USER_VALUE))
            .and_then(|word| UserStart::from_word(&word))
    }

    fn set_user_start(&self, start: UserStart) {
        let written = self.registry.lock().map(|mut r| {
            r.set(
                Hive::CurrentUser,
                KEY,
                USER_VALUE,
                &RegValue::String(start.word().to_owned()),
            )
        });
        if let Ok(Err(e)) | Err(e) = written.map_err(|e| e.to_string()) {
            tracing::warn!(error = %e, "the start after sign-in was not kept");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::MemoryRegistry;

    /// The resolver's table: the user's choice over the machine's, none being Always; the
    /// choice kept as its word.
    #[test]
    fn the_user_choice_decides_the_sign_in_start() {
        let mut machine = MemoryRegistry::new();
        let _ = machine.set(
            Hive::LocalMachine,
            KEY,
            ALL_USERS_VALUE,
            &RegValue::Dword(0),
        );
        let options = RegistryStartOptions::new(Box::new(machine));
        assert_eq!(options.user_start(), None);
        assert!(options.starts_after_sign_in());
        for (start, starts) in [
            (UserStart::Never, false),
            (UserStart::AllUsers, false),
            (UserStart::Always, true),
        ] {
            options.set_user_start(start);
            assert_eq!(options.user_start(), Some(start));
            assert_eq!(options.starts_after_sign_in(), starts, "{start:?}");
        }
        let mut machine = MemoryRegistry::new();
        let _ = machine.set(
            Hive::LocalMachine,
            KEY,
            ALL_USERS_VALUE,
            &RegValue::Dword(1),
        );
        let _ = machine.set(
            Hive::CurrentUser,
            KEY,
            USER_VALUE,
            &RegValue::String("all-users".into()),
        );
        assert!(RegistryStartOptions::new(Box::new(machine)).starts_after_sign_in());
    }
}
