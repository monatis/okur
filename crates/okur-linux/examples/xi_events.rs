//! XInput 2 through x11rb, alone, for the Linux lab: the version queries and raw key events
//! selected on the root, then every event printed for the seconds given (5 by default). It
//! tells whether this connection hears XInput 2 at all, apart from the hook's grabs.

#[cfg(target_os = "linux")]
fn main() {
    use x11rb::connection::Connection;
    use x11rb::protocol::ge::ConnectionExt as _;
    use x11rb::protocol::xinput::{self, ConnectionExt as _};
    use x11rb::protocol::xproto::ConnectionExt as _;

    let seconds = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(5u64);
    let Ok((conn, screen)) = x11rb::connect(None) else {
        println!("no X display");
        return;
    };
    let Some(root) = conn.setup().roots.get(screen).map(|s| s.root) else {
        return;
    };
    let ge = conn.ge_query_version(1, 0).map(|c| c.reply());
    println!("ge: {ge:?}");
    let xi = conn.xinput_xi_query_version(2, 2).map(|c| c.reply());
    println!("xi: {xi:?}");
    let selected = conn
        .xinput_xi_select_events(
            root,
            &[xinput::EventMask {
                deviceid: xinput::Device::ALL_MASTER.into(),
                mask: vec![
                    xinput::XIEventMask::RAW_KEY_PRESS | xinput::XIEventMask::RAW_KEY_RELEASE,
                ],
            }],
        )
        .map(|c| c.check());
    println!("selected: {selected:?}");
    // `grab`: every keycode on every top-level window, synchronously, as the hook grabs, each
    // reply read (`grab-dropped`: the replies dropped, as the hook drops them).
    let mode = std::env::args().nth(2).unwrap_or_default();
    if mode.starts_with("grab") {
        let tops = conn
            .query_tree(root)
            .ok()
            .and_then(|c| c.reply().ok())
            .map(|r| r.children)
            .unwrap_or_default();
        let mut failed = 0;
        for window in &tops {
            for code in 8u32..=255 {
                let cookie = conn.xinput_xi_passive_grab_device(
                    x11rb::CURRENT_TIME,
                    *window,
                    x11rb::NONE,
                    code,
                    3u16,
                    xinput::GrabType::KEYCODE,
                    xinput::GrabMode22::SYNC,
                    x11rb::protocol::xproto::GrabMode::ASYNC,
                    xinput::GrabOwner::NO_OWNER,
                    &[u32::from(
                        xinput::XIEventMask::KEY_PRESS | xinput::XIEventMask::KEY_RELEASE,
                    )],
                    &[0, 1],
                );
                if mode == "grab" {
                    if let Ok(Err(_)) = cookie.map(|c| c.reply()) {
                        failed += 1;
                    }
                }
            }
        }
        let _ = conn.flush();
        println!("grabbed {} windows ({mode}), failed {failed}", tops.len());
    }
    // `thread`: the events read as the hook reads them, `wait_for_event` in a thread of its own
    // over a shared connection.
    if std::env::args().nth(3).as_deref() == Some("thread") {
        let conn = std::sync::Arc::new(conn);
        let reader = std::sync::Arc::clone(&conn);
        std::thread::spawn(move || loop {
            match reader.wait_for_event() {
                Ok(event) => println!("thread event: {event:?}"),
                Err(error) => {
                    println!("thread error: {error}");
                    return;
                }
            }
        });
        std::thread::sleep(std::time::Duration::from_secs(seconds));
        return;
    }
    let until = std::time::Instant::now() + std::time::Duration::from_secs(seconds);
    while std::time::Instant::now() < until {
        match conn.poll_for_event() {
            Ok(Some(event)) => println!("event: {event:?}"),
            Ok(None) => std::thread::sleep(std::time::Duration::from_millis(10)),
            Err(error) => {
                println!("error: {error}");
                return;
            }
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn main() {}
