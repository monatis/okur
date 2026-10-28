//! Windows applications and the Windows shell.

pub mod desktop;
pub mod explorer;
pub mod search;
pub mod settings;
pub mod task_switcher;
pub mod taskbar;
pub mod terminal;
pub mod widgets;
pub mod word;
pub mod word_dialog;
pub mod word_ribbon;

use super::AppModule;

/// The Windows modules, in the order focus events are offered to them: Widgets' background
/// updates are dropped before anything, the task switcher sees every other focus (it starts
/// over when one is not its own), and the desktop is told from File Explorer before either
/// claims a list.
pub fn modules() -> Vec<Box<dyn AppModule>> {
    vec![
        Box::new(widgets::Widgets),
        Box::new(task_switcher::TaskSwitcher::default()),
        Box::new(desktop::Desktop::default()),
        Box::new(explorer::Explorer::default()),
        Box::new(taskbar::Taskbar),
        Box::new(terminal::Terminal),
        Box::new(search::Search),
        Box::new(settings::Settings),
        Box::new(word_ribbon::WordRibbon::default()),
        Box::new(word_dialog::WordDialog::default()),
        Box::new(word::WordDocument),
    ]
}
