//! Okur's speech on Linux, live, for the Linux lab (`skills/linux-lab/scripts/probe-ssip.sh`):
//! three parts of a reading given as Say All gives them (marked, one behind the other), each
//! mark printed as speech-dispatcher reaches it, then whether it is still speaking.

#[cfg(target_os = "linux")]
fn main() {
    use okur_runtime::synthesizer::TtsEngine;
    use std::time::{Duration, Instant};

    let mut engine = match okur_linux::speech::SsipEngine::new() {
        Ok(engine) => engine,
        Err(error) => {
            println!("speech did not start: {error}");
            return;
        }
    };
    println!("voices: {}", engine.get_voices().len());
    let parts = [
        "The first part of the reading.",
        "Then the second <part> & its symbols.",
        "And the third.",
    ];
    for (mark, part) in (1u64..).zip(parts) {
        if let Err(error) = engine.speak_marked(part, mark, 0, false) {
            println!("part {mark} failed: {error}");
        }
    }
    let until = Instant::now() + Duration::from_secs(20);
    let mut last = 0;
    while Instant::now() < until && last < 3 {
        if let Some(mark) = engine.reached() {
            println!("reached {mark}");
            last = mark;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let _ = engine.wait_until_done(10_000);
    println!("speaking after the last: {}", engine.is_speaking());
}

#[cfg(not(target_os = "linux"))]
fn main() {}
