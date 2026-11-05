//! Okur's X11 keyboard hook, live, for the Linux lab (`skills/linux-lab/scripts/probe-hook.sh`):
//! it keeps H, answers W only after half a second (so the watchdog lets it go first), passes
//! every other key, and prints each key it is told of with what it decided. Runs for the seconds
//! given (10 by default), then stops the hook.

#[cfg(target_os = "linux")]
fn main() {
    use okur_core::input::{Key, KeyAction};
    use okur_runtime::hook::KeyboardHook;

    let seconds = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);
    let mut hook = okur_linux::keyboard::X11KeyboardHook::new();
    let started = hook.start(Box::new(|event| {
        let action = match event.key {
            Key::Alpha('H') if event.is_down => KeyAction::Consume,
            Key::Alpha('W') if event.is_down => {
                std::thread::sleep(std::time::Duration::from_millis(500));
                KeyAction::Consume
            }
            _ => KeyAction::PassThrough,
        };
        println!(
            "{} {:?} {:?} {:?}",
            if event.is_down { "down" } else { "up" },
            event.key,
            event.text,
            action
        );
        action
    }));
    if let Err(error) = started {
        println!("the hook did not start: {error}");
        return;
    }
    println!("hook started");
    std::thread::sleep(std::time::Duration::from_secs(seconds));
    let _ = hook.stop();
    println!("hook stopped");
}

#[cfg(not(target_os = "linux"))]
fn main() {}
