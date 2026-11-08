//! A page's AT-SPI snapshot for the corpus (`corpus/snapshots/<fixture>.chrome.atspi.json`, the
//! plan's §2.3): waits for Chrome's focus in the page whose address ends with the given end,
//! reads it whole as Okur reads a page, and writes it with its provenance, for
//! `okur-lab parity --api atspi` to replay on every OS. Run by
//! `skills/linux-lab/scripts/capture-atspi-snapshots.sh`, started before Chrome:
//!
//!     atspi_snapshot <address end> <fixture sha256> <Chrome's version> <out.json>

#[cfg(target_os = "linux")]
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [address_end, fixture_sha256, chrome, out] = args.as_slice() else {
        eprintln!(
            "usage: atspi_snapshot <address end> <fixture sha256> <Chrome's version> <out.json>"
        );
        std::process::exit(2);
    };
    let started = std::time::Instant::now();
    let root =
        match okur_linux::events::capture_page(address_end, std::time::Duration::from_secs(60)) {
            Ok(root) => root,
            Err(error) => {
                eprintln!("atspi_snapshot: {error}");
                std::process::exit(1);
            }
        };
    let capture_ms = started.elapsed().as_millis();
    let node_count = count(&root);
    let read = |path: &str| std::fs::read_to_string(path).unwrap_or_default();
    let os_version = read("/etc/os-release")
        .lines()
        .find_map(|line| line.strip_prefix("PRETTY_NAME="))
        .map(|name| name.trim_matches('"').to_owned())
        .unwrap_or_default();
    let snapshot = okur_linux_model::page::AtspiSnapshot {
        provenance: okur_linux_model::page::SnapshotProvenance {
            format_version: 1,
            tool_version: format!("okur-linux {}", env!("CARGO_PKG_VERSION")),
            host: read("/etc/hostname").trim().to_owned(),
            os_version,
            window_title: root.name.clone(),
            window_class: String::new(),
            process_version: Some(chrome.clone()),
            fixture_sha256: Some(fixture_sha256.clone()),
            captured_at_unix_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or_default(),
            node_count,
            capture_ms,
        },
        root,
    };
    let written = serde_json::to_string_pretty(&snapshot)
        .map_err(|e| e.to_string())
        .and_then(|json| std::fs::write(out, json + "\n").map_err(|e| e.to_string()));
    match written {
        Ok(()) => println!("atspi_snapshot: {node_count} objects in {capture_ms} ms to {out}"),
        Err(error) => {
            eprintln!("atspi_snapshot: {out}: {error}");
            std::process::exit(1);
        }
    }
}

/// The objects in `node`'s tree, itself included.
#[cfg(target_os = "linux")]
fn count(node: &okur_linux_model::page::AtspiNode) -> usize {
    1 + node.children.iter().map(count).sum::<usize>()
}

#[cfg(not(target_os = "linux"))]
fn main() {}
