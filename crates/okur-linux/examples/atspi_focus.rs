//! Okur's Linux event source, live, for the Linux lab (`skills/linux-lab/scripts/probe-focus.sh`):
//! every focus event the desktop's applications send, as the engine would be given it, printed
//! as "focus <role> <name>". Runs for the seconds given (10 by default).

#[cfg(target_os = "linux")]
fn main() {
    use okur_core::event::AccessibilityEvent;
    use okur_runtime::events::{EventSink, EventSource};

    let seconds = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let mut source = okur_linux::events::AtspiEventSource::new();
    if let Err(error) = source.start(EventSink::new(tx)) {
        println!("the event source did not start: {error}");
        return;
    }
    println!("events started");
    let until = std::time::Instant::now() + std::time::Duration::from_secs(seconds);
    while std::time::Instant::now() < until {
        match rx.try_recv() {
            Ok(AccessibilityEvent::Focus(focus)) => println!(
                "focus {:?} {}",
                focus.role,
                focus.name.as_deref().unwrap_or("")
            ),
            Ok(_) => {}
            Err(_) => std::thread::sleep(std::time::Duration::from_millis(20)),
        }
    }
    let _ = source.stop();
    println!("events stopped");
}

#[cfg(not(target_os = "linux"))]
fn main() {}
