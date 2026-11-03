//! How Windows starts `okur-ui` ([`okur_runtime::ui_host::UiSpawner`]).

use std::path::Path;

use okur_runtime::ui_host::{spawn_plain, UiChild, UiProcess, UiSpawner};

use crate::lowered_process::LoweredChild;

/// Starts `okur-ui` below Okur's UI Access when Okur has it: with the lowered token, and let to
/// the front for each window it shows. Without UI Access (an Okur started as administrator
/// from a terminal), `okur-ui` runs at Okur's own level, as before.
#[derive(Debug, Clone, Copy)]
pub struct WindowsUiSpawner {
    lowered: bool,
}

impl WindowsUiSpawner {
    /// The spawner for this process: lowered only below UI Access.
    pub fn for_this_process() -> Self {
        Self {
            lowered: crate::elevation::has_ui_access(),
        }
    }
}

impl UiChild for LoweredChild {
    fn has_exited(&mut self) -> bool {
        LoweredChild::has_exited(self)
    }

    fn id(&self) -> u32 {
        LoweredChild::id(self)
    }
}

impl UiSpawner for WindowsUiSpawner {
    fn spawn(&self, program: &Path) -> Option<UiProcess> {
        if !self.lowered {
            return spawn_plain(program);
        }
        tracing::debug!("okur-ui started lowered");
        match crate::lowered_process::spawn(program) {
            Ok((child, pipes)) => Some(UiProcess {
                child: Box::new(child),
                input: Box::new(pipes.input),
                output: Box::new(pipes.output),
                error: Box::new(pipes.error),
            }),
            // Okur's windows still show, at Okur's own level.
            Err(e) => {
                tracing::warn!(error = %e, "okur-ui not started lowered; started as Okur");
                spawn_plain(program)
            }
        }
    }

    /// A lowered `okur-ui` may take the foreground only when Okur lets it (the wizard and the
    /// main window were left to the thread-attach trick alone: performance review 2026-10-25,
    /// finding 10).
    fn allow_foreground(&self, pid: u32) {
        if self.lowered {
            crate::process::allow_foreground(pid);
        }
    }
}
