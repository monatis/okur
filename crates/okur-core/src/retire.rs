//! Freeing large values away from the thread that lets go of them.
//!
//! A page's buffer holds a node per element, each with its own attribute map: freeing the one
//! a capture replaced took 18-36 ms on a page of 25,000 nodes, in the keyboard hook, which has
//! about 150 ms before Windows passes the key on (responsiveness review, 2026-10-19). Such
//! values go to a thread of their own, which frees them.

use std::sync::mpsc::{channel, Sender};
use std::sync::{Mutex, OnceLock};

type Retired = Box<dyn Send>;

/// The freeing thread's queue, started on first use; `None` when it could not start.
fn queue() -> Option<&'static Mutex<Sender<Retired>>> {
    static QUEUE: OnceLock<Option<Mutex<Sender<Retired>>>> = OnceLock::new();
    QUEUE
        .get_or_init(|| {
            let (sender, receiver) = channel::<Retired>();
            std::thread::Builder::new()
                .name("okur-retire".into())
                .spawn(move || {
                    // Each value is freed as it is received.
                    for retired in receiver {
                        drop(retired);
                    }
                })
                .ok()
                .map(|_| Mutex::new(sender))
        })
        .as_ref()
}

/// Frees `value` on the freeing thread, or here when that thread is not running.
pub fn retire<T: Send + 'static>(value: T) {
    let Some(queue) = queue() else {
        return;
    };
    // A value the queue refuses comes back in the error, and is freed here.
    if let Ok(sender) = queue.lock() {
        let _ = sender.send(Box::new(value));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A retired value is freed, on another thread.
    #[test]
    fn a_retired_value_is_freed_elsewhere() {
        struct Flag(std::sync::mpsc::Sender<std::thread::ThreadId>);
        impl Drop for Flag {
            fn drop(&mut self) {
                let _ = self.0.send(std::thread::current().id());
            }
        }
        let (sender, receiver) = channel();
        retire(Flag(sender));
        let freed_on = receiver.recv_timeout(std::time::Duration::from_secs(5));
        assert!(freed_on.is_ok_and(|id| id != std::thread::current().id()));
    }
}
