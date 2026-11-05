//! XInput 2 through x11rb, alone, for the Linux lab: the version queries and raw key events
//! selected on the root, then every event printed for the seconds given (5 by default). It
//! tells whether this connection hears XInput 2 at all, apart from the hook's grabs.

#[cfg(target_os = "linux")]
fn main() {
    use x11rb::connection::Connection;
    use x11rb::protocol::ge::ConnectionExt as _;
    use x11rb::protocol::xinput::{self, ConnectionExt as _};

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
