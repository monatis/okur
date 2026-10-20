//! `okur-lab-keys.exe`: the lab's key sender with UI Access (`manifests/okur-lab-keys.manifest`),
//! for keys into windows that refuse a normal program's (JAWS's own dialogs, which run with
//! UI Access: its Jump To Line dialog closed when a JAWS function typed into it, 559/s2).
//! A harness tool, never installed for users: on the reference machine only, signed with the
//! development certificate and copied under Program Files once, as Windows requires for UI
//! Access.
//!
//! ```text
//! okur-lab-keys --allow jfw[,chrome] [--title <text>] <step>...
//! ```
//!
//! The steps and the checks are `okur keys`'s: each chord is sent only while one of the
//! allowed processes is in front.

#[cfg(windows)]
fn main() {
    let mut allow: Vec<String> = Vec::new();
    let mut title: Option<String> = None;
    let mut steps = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--allow" => allow.extend(
                args.next()
                    .unwrap_or_default()
                    .split(',')
                    .map(str::trim)
                    .filter(|a| !a.is_empty())
                    .map(str::to_owned),
            ),
            "--title" => title = args.next(),
            _ => steps.push(arg),
        }
    }
    if allow.is_empty() || steps.is_empty() {
        eprintln!("usage: okur-lab-keys --allow <process>[,<process>] [--title <text>] <step>...");
        std::process::exit(2);
    }
    let result = steps
        .iter()
        .map(|s| okur_platform::harness::parse_step(s))
        .collect::<Result<Vec<_>, _>>()
        .and_then(|steps| okur_platform::harness::send_steps(&steps, &allow, title.as_deref()));
    if let Err(e) = result {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("okur-lab-keys runs on Windows only");
    std::process::exit(1);
}
