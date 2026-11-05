//! The key Okur binds to each JAWS script named, in the scenarios' notation: how the Linux lab's
//! runner (`skills/linux-lab/scripts/run-scenario-linux.py`) plays a scenario's `jaws_script`
//! step live, as the user would press it. One line per name: the name, a tab, the chord, or
//! nothing after the tab when Okur binds no key to it.
//!
//!     cargo run -q -p okur-core --example script_key -- MoveToNextHeading SayLine

fn main() {
    let bound = okur_core::command::bound_commands();
    for name in std::env::args().skip(1) {
        let chord = okur_core::command::Command::from_name(&name)
            .and_then(|command| bound.iter().find(|(c, _)| *c == command))
            .map(|(_, key)| key.chord())
            .unwrap_or_default();
        println!("{name}\t{chord}");
    }
}
