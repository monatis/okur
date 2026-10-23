//! `okur-lab-notify-probe.exe`: `okur-lab probe-notifications` with UI Access
//! (`manifests/okur-lab-notify-probe.manifest`). A plain client heard none of the notifications a
//! WinForms window raised on the reference machine, which JAWS, a UI Access program, spoke (820,
//! rounds 247 to 249); this tells whether UI Access is the difference. A harness tool, never
//! installed for users: signed with the development certificate and copied under Program
//! Files, as Windows requires for UI Access. Windows starts such a program only through
//! ShellExecute, whose output cannot be redirected, so the lines go to a file.
//!
//! ```text
//! okur-lab-notify-probe --window <title> [--millis <n>] --out <file>
//! ```

#[cfg(windows)]
fn main() {
    let mut window: Option<String> = None;
    let mut millis: u64 = 15000;
    let mut out: Option<String> = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--window" => window = args.next(),
            "--millis" => millis = args.next().and_then(|m| m.parse().ok()).unwrap_or(millis),
            "--out" => out = args.next(),
            _ => {}
        }
    }
    let (Some(window), Some(out)) = (window, out) else {
        eprintln!("usage: okur-lab-notify-probe --window <title> [--millis <n>] --out <file>");
        std::process::exit(2);
    };
    let lines = match okur_platform::uia_probe::notification_registrations(&window, millis) {
        Ok(lines) => lines,
        Err(e) => vec![format!("failed: {e}")],
    };
    let mut text = lines.join("\n");
    text.push('\n');
    if let Err(e) = std::fs::write(&out, text) {
        eprintln!("{out}: {e}");
        std::process::exit(1);
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("okur-lab-notify-probe runs on Windows only");
    std::process::exit(1);
}
