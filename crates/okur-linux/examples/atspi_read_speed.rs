//! How fast a page is read whole, in order and in parallel (the code review of 2026-11-08,
//! finding 6): waits for Chrome's focus in the page whose address ends with the given end,
//! then reads its document `rounds` times each way, alternating, and says each read's time,
//! its objects, and whether both reads gave the same tree. Run by
//! `skills/linux-lab/scripts/probe-read-speed.sh`, started before Chrome:
//!
//!     atspi_read_speed <address end> [rounds]

#[cfg(target_os = "linux")]
fn main() {
    use std::time::{Duration, Instant};
    use xa11y_core::{EventKind, RecvStatus};
    use xa11y_linux::{AtspiObject, AtspiTree, LinuxProvider};

    fn count(tree: &AtspiTree) -> usize {
        1 + tree.children.iter().map(count).sum::<usize>()
    }

    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(address_end) = args.first() else {
        eprintln!("usage: atspi_read_speed <address end> [rounds]");
        std::process::exit(2);
    };
    let rounds: usize = args.get(1).and_then(|r| r.parse().ok()).unwrap_or(3);
    let provider = match LinuxProvider::new() {
        Ok(provider) => provider,
        Err(e) => {
            eprintln!("atspi_read_speed: the bus: {e}");
            std::process::exit(1);
        }
    };
    let subscription = match provider.subscribe_desktop_kinds(Box::new(|kind, _, _| {
        matches!(kind, EventKind::FocusChanged | EventKind::DocumentLoaded)
    })) {
        Ok(subscription) => subscription,
        Err(e) => {
            eprintln!("atspi_read_speed: the desktop's events: {e}");
            std::process::exit(1);
        }
    };
    let started = Instant::now();
    let mut document = None;
    while document.is_none() && started.elapsed() < Duration::from_secs(60) {
        let RecvStatus::Event(event) = subscription.recv_status(Duration::from_millis(200)) else {
            continue;
        };
        let Some(object) = event.target.as_ref().and_then(AtspiObject::of) else {
            continue;
        };
        let Some(found) = provider.atspi_ancestor(&object, "document web", 40) else {
            continue;
        };
        if provider
            .atspi_tree(&found, 1)
            .uri
            .is_some_and(|uri| uri.ends_with(address_end.as_str()))
        {
            document = Some(found);
        }
    }
    let Some(document) = document else {
        eprintln!("atspi_read_speed: no focus reached a page at …{address_end}");
        std::process::exit(1);
    };
    // Settled: the page's own load events over.
    std::thread::sleep(Duration::from_secs(2));
    // rayon's threads, one a processor.
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
    for round in 1..=rounds {
        let reading = Instant::now();
        let in_order = provider.atspi_tree(&document, 20_000);
        let in_order_ms = reading.elapsed().as_millis();
        let reading = Instant::now();
        let parallel = provider.atspi_tree_parallel(&document, 20_000);
        let parallel_ms = reading.elapsed().as_millis();
        println!(
            "{address_end} round {round}: in order {in_order_ms} ms, parallel {parallel_ms} ms \
             ({threads} threads), {} objects, same tree: {}",
            count(&in_order),
            in_order == parallel
        );
    }
}

#[cfg(not(target_os = "linux"))]
fn main() {}
