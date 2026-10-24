//! Whether a starting Okur goes on, and how it takes its desktop from a copy already there
//! (`docs/references/startup-and-secure-screens-proposal.md`, §5.3, §5.5 and §5.6).
//!
//! - An **automatic start** (Windows' Ease of Access, `--ease-of-access`; the sign-in start,
//!   `--sign-in`) ends silently when both Shift keys are held, JAWS's documented escape, or
//!   when a copy already runs on the desktop (NVDA's `--ease-of-access` rule).
//! - A **manual start** (`okur run`, `okur-sr.exe` from Win+R) asks the copy already running
//!   to quit and takes over, as NVDA does; one that does not go is ended.
//! - No copy runs on the secure desktop until its secure mode is built (the proposal's M6): a
//!   process there runs as SYSTEM, reachable by anyone at the keyboard.

use std::time::Duration;

/// How Okur was started.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartKind {
    /// By the user: `okur run`, `okur-sr.exe` from the Run box.
    Manual,
    /// By Windows' Assistive Technology Manager (`--ease-of-access`).
    EaseOfAccess,
    /// By the sign-in start (`--sign-in`, the Run key).
    SignIn,
}

impl StartKind {
    /// Whether Windows or the sign-in started Okur, not the user.
    pub fn is_automatic(self) -> bool {
        self != StartKind::Manual
    }
}

/// What a starting copy knows of where it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StartFacts {
    pub kind: StartKind,
    /// Both Shift keys are down.
    pub both_shifts_held: bool,
    /// The process's desktop is the secure one (Winlogon).
    pub secure_desktop: bool,
    /// The process runs as SYSTEM.
    pub system_account: bool,
}

/// Why a starting copy ends before making a sound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SilentExit {
    /// Both Shift keys were held at an automatic start.
    ShiftsHeld,
    /// The secure desktop: no secure mode yet.
    SecureDesktop,
    /// Started by Ease of Access as SYSTEM, but not on the secure desktop: never Okur's case.
    SystemOffSecureDesktop,
    /// An automatic start found a copy already running on its desktop.
    AlreadyRunning,
    /// The sign-in start, where this user's choice (or the machine's, which it follows) is
    /// not to start (the proposal's Run-key resolver).
    NotAfterSignIn,
    /// The copy running would not let the desktop go.
    NotReplaced,
}

impl SilentExit {
    /// A line for the log.
    pub fn describe(self) -> &'static str {
        match self {
            SilentExit::ShiftsHeld => "both Shift keys held at an automatic start",
            SilentExit::SecureDesktop => "the secure desktop has no Okur until its secure mode",
            SilentExit::SystemOffSecureDesktop => {
                "started as SYSTEM by Ease of Access off the secure desktop"
            }
            SilentExit::AlreadyRunning => "a copy already runs on this desktop",
            SilentExit::NotAfterSignIn => "this user's start after sign-in is off",
            SilentExit::NotReplaced => "the copy running on this desktop did not quit",
        }
    }
}

/// Whether a start goes on, before any copy already running is looked at.
pub fn preflight(facts: &StartFacts) -> Result<(), SilentExit> {
    if facts.kind.is_automatic() && facts.both_shifts_held {
        return Err(SilentExit::ShiftsHeld);
    }
    if facts.secure_desktop {
        return Err(SilentExit::SecureDesktop);
    }
    if facts.kind == StartKind::EaseOfAccess && facts.system_account {
        return Err(SilentExit::SystemOffSecureDesktop);
    }
    Ok(())
}

/// The copies of Okur on this desktop, as a starting copy deals with them.
pub trait Instances {
    /// What holds the desktop: the lock, let go when dropped.
    type Lock;

    /// Takes the desktop's lock, waiting up to `wait` for a copy that is exiting.
    fn try_lock(&mut self, wait: Duration) -> Option<Self::Lock>;
    /// Asks the copy running to quit; its process id, when it said.
    fn ask_to_quit(&mut self) -> Option<u32>;
    /// Ends a copy that did not quit when asked.
    fn terminate(&mut self, pid: u32);
}

/// How long an automatic start waits for a copy that is exiting (NVDA waits 2 s).
pub const EXITING_WAIT: Duration = Duration::from_secs(2);
/// How long a manual start waits for the copy it asked to quit (NVDA waits 4 s).
pub const QUIT_WAIT: Duration = Duration::from_secs(4);

/// Takes this desktop for a starting copy: at once when no copy runs; an automatic start
/// steps aside for one that does; a manual start asks it to quit, waits for it, and ends it
/// if it stays.
pub fn take_desktop<I: Instances>(
    kind: StartKind,
    instances: &mut I,
) -> Result<I::Lock, SilentExit> {
    let first_wait = if kind.is_automatic() {
        EXITING_WAIT
    } else {
        Duration::from_millis(100)
    };
    if let Some(lock) = instances.try_lock(first_wait) {
        return Ok(lock);
    }
    if kind.is_automatic() {
        return Err(SilentExit::AlreadyRunning);
    }
    let pid = instances.ask_to_quit();
    if let Some(lock) = instances.try_lock(QUIT_WAIT) {
        return Ok(lock);
    }
    if let Some(pid) = pid {
        instances.terminate(pid);
        if let Some(lock) = instances.try_lock(EXITING_WAIT) {
            return Ok(lock);
        }
    }
    Err(SilentExit::NotReplaced)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(kind: StartKind) -> StartFacts {
        StartFacts {
            kind,
            both_shifts_held: false,
            secure_desktop: false,
            system_account: false,
        }
    }

    #[test]
    fn both_shifts_stop_an_automatic_start_only() {
        for kind in [StartKind::EaseOfAccess, StartKind::SignIn] {
            let held = StartFacts {
                both_shifts_held: true,
                ..facts(kind)
            };
            assert_eq!(preflight(&held), Err(SilentExit::ShiftsHeld), "{kind:?}");
        }
        let manual = StartFacts {
            both_shifts_held: true,
            ..facts(StartKind::Manual)
        };
        assert_eq!(preflight(&manual), Ok(()));
        assert_eq!(preflight(&facts(StartKind::SignIn)), Ok(()));
    }

    #[test]
    fn no_copy_starts_on_the_secure_desktop_yet() {
        let secure = StartFacts {
            secure_desktop: true,
            system_account: true,
            ..facts(StartKind::EaseOfAccess)
        };
        assert_eq!(preflight(&secure), Err(SilentExit::SecureDesktop));
        let system_on_default = StartFacts {
            system_account: true,
            ..facts(StartKind::EaseOfAccess)
        };
        assert_eq!(
            preflight(&system_on_default),
            Err(SilentExit::SystemOffSecureDesktop)
        );
    }

    /// The desktop's copies as a test sets them: whether the lock is free, and whether the
    /// copy running quits when asked; what was done to it is kept.
    struct Desktop {
        held: bool,
        quits: bool,
        killable: bool,
        asked: u32,
        terminated: Vec<u32>,
    }

    impl Desktop {
        fn new(held: bool, quits: bool, killable: bool) -> Self {
            Self {
                held,
                quits,
                killable,
                asked: 0,
                terminated: Vec::new(),
            }
        }
    }

    impl Instances for Desktop {
        type Lock = &'static str;

        fn try_lock(&mut self, _: Duration) -> Option<&'static str> {
            (!self.held).then_some("lock")
        }

        fn ask_to_quit(&mut self) -> Option<u32> {
            self.asked += 1;
            if self.quits {
                self.held = false;
            }
            Some(4321)
        }

        fn terminate(&mut self, pid: u32) {
            self.terminated.push(pid);
            if self.killable {
                self.held = false;
            }
        }
    }

    #[test]
    fn a_free_desktop_is_taken_by_any_start() {
        for kind in [
            StartKind::Manual,
            StartKind::EaseOfAccess,
            StartKind::SignIn,
        ] {
            let mut desktop = Desktop::new(false, false, false);
            assert_eq!(take_desktop(kind, &mut desktop), Ok("lock"));
            assert_eq!(desktop.asked, 0);
        }
    }

    /// `--ease-of-access` and `--sign-in` with a copy running end without touching it.
    #[test]
    fn an_automatic_start_steps_aside_for_a_running_copy() {
        for kind in [StartKind::EaseOfAccess, StartKind::SignIn] {
            let mut desktop = Desktop::new(true, true, true);
            assert_eq!(
                take_desktop(kind, &mut desktop),
                Err(SilentExit::AlreadyRunning)
            );
            assert_eq!(desktop.asked, 0, "{kind:?}");
            assert!(desktop.terminated.is_empty());
        }
    }

    /// A manual start replaces the running copy: asked to quit, and ended if it stays.
    #[test]
    fn a_manual_start_replaces_the_running_copy() {
        let mut quits = Desktop::new(true, true, true);
        assert_eq!(take_desktop(StartKind::Manual, &mut quits), Ok("lock"));
        assert_eq!((quits.asked, quits.terminated.len()), (1, 0));

        let mut stays = Desktop::new(true, false, true);
        assert_eq!(take_desktop(StartKind::Manual, &mut stays), Ok("lock"));
        assert_eq!(stays.terminated, vec![4321]);

        let mut stuck = Desktop::new(true, false, false);
        assert_eq!(
            take_desktop(StartKind::Manual, &mut stuck),
            Err(SilentExit::NotReplaced)
        );
    }
}
