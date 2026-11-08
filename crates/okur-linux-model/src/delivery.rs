//! The order the engine is given the driver's records in, while pages are read off the event
//! thread (the code review of 2026-11-08, finding 6, and its second review). A page asked for is
//! a slot in the queue before the record that asked for it, as the page came first when it was
//! read on the event thread; the records after it wait until it is read, so the engine takes the
//! page and then its focus, as before. A focus outside the page's document cancels its slot: the
//! focus moved on (Alt+Tab to an editor, Ctrl+Tab to another page), and a page given to the
//! engine then would be read as a new load over the editor, its letters taken as quick keys
//! (the second review's finding 1). Pure, tested on every OS; the driver
//! (`okur_linux::events`) reads, and gives the engine what [`Delivery::take_ready`] releases.

use std::collections::VecDeque;

use xa11y_core::EventKind;

use crate::convert::{AtspiRecord, PageRead};

/// How long a page may keep the records after it waiting, in milliseconds: a page at the
/// driver's object limit reads in about 24 s in parallel. Past it the page is given up.
pub const HOLD_LIMIT_MS: u64 = 30_000;

#[derive(Debug)]
enum Item {
    Ready(Box<AtspiRecord>),
    Page {
        document: String,
        /// When it was asked for: the time its record has, as the record that asked for it.
        asked_ms: u64,
        read: Option<Box<PageRead>>,
    },
}

/// The records and the pages asked for, in the order the engine takes them.
#[derive(Debug, Default)]
pub struct Delivery {
    items: VecDeque<Item>,
}

impl Delivery {
    /// Whether the page of `document` is asked for and not read yet: a focus in it, or its load,
    /// does not ask again.
    pub fn is_awaited(&self, document: &str) -> bool {
        self.items
            .iter()
            .any(|item| matches!(item, Item::Page { document: d, read: None, .. } if d == document))
    }

    /// The page of `document` asked for at `at_ms`, before the record that asks for it.
    pub fn ask(&mut self, document: String, at_ms: u64) {
        self.items.push_back(Item::Page {
            document,
            asked_ms: at_ms,
            read: None,
        });
    }

    /// A record, after the pages asked before it. A focus outside a page's document cancels
    /// that page (unless `may_cancel` is false: GTK's twin focus, which was not read); the
    /// documents of the pages cancelled are returned, for the reader to skip and the converter
    /// to ask again for when the focus comes back.
    pub fn push(&mut self, record: AtspiRecord, may_cancel: bool) -> Vec<String> {
        let mut cancelled = Vec::new();
        if may_cancel && record.kind == EventKind::FocusChanged {
            let here = record.document.as_deref();
            self.items.retain(|item| match item {
                Item::Page { document, .. } if Some(document.as_str()) != here => {
                    cancelled.push(document.clone());
                    false
                }
                _ => true,
            });
        }
        self.items.push_back(Item::Ready(Box::new(record)));
        cancelled
    }

    /// A page read: into the first slot of its document still waiting. A page whose slot was
    /// cancelled, or never asked for, is dropped: whether it was taken.
    pub fn page_read(&mut self, page: PageRead) -> bool {
        let slot = self.items.iter_mut().find_map(|item| match item {
            Item::Page { document, read, .. } if read.is_none() && *document == page.document => {
                Some(read)
            }
            _ => None,
        });
        match slot {
            Some(read) => {
                *read = Some(Box::new(page));
                true
            }
            None => false,
        }
    }

    /// Pages waited for longer than [`HOLD_LIMIT_MS`] by `now_ms` given up; their documents.
    pub fn expire(&mut self, now_ms: u64) -> Vec<String> {
        let mut expired = Vec::new();
        self.items.retain(|item| match item {
            Item::Page {
                document,
                asked_ms,
                read: None,
            } if now_ms.saturating_sub(*asked_ms) > HOLD_LIMIT_MS => {
                expired.push(document.clone());
                false
            }
            _ => true,
        });
        expired
    }

    /// What the engine may take now, in order: records, and pages read (a record of their own
    /// at the time they were asked for), up to the first page still waiting.
    pub fn take_ready(&mut self) -> Vec<AtspiRecord> {
        let mut ready = Vec::new();
        while let Some(item) = self.items.pop_front() {
            match item {
                Item::Ready(record) => ready.push(*record),
                Item::Page {
                    asked_ms,
                    read: Some(page),
                    ..
                } => ready.push(AtspiRecord::page(asked_ms, *page)),
                waiting @ Item::Page { read: None, .. } => {
                    self.items.push_front(waiting);
                    break;
                }
            }
        }
        ready
    }

    /// Whether anything waits for a page.
    pub fn is_holding(&self) -> bool {
        !self.items.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::page::AtspiNode;

    fn record(kind: EventKind, document: Option<&str>, at_ms: u64) -> AtspiRecord {
        let mut record = AtspiRecord::bare(at_ms, kind, "chrome".to_owned(), None);
        record.document = document.map(str::to_owned);
        record
    }

    fn page(document: &str) -> PageRead {
        PageRead {
            document: document.to_owned(),
            base: 0,
            root: AtspiNode::default(),
            read_ms: 0,
        }
    }

    fn kinds(records: &[AtspiRecord]) -> Vec<(EventKind, Option<String>)> {
        records
            .iter()
            .map(|r| (r.kind.clone(), r.document.clone()))
            .collect()
    }

    /// The page comes first, its focus and what followed after it, as when the page was read on
    /// the event thread.
    #[test]
    fn a_focus_waits_for_its_page() {
        let mut delivery = Delivery::default();
        delivery.ask("A".to_owned(), 10);
        assert!(delivery
            .push(record(EventKind::FocusChanged, Some("A"), 10), true)
            .is_empty());
        delivery.push(record(EventKind::NameChanged, None, 20), true);
        assert!(delivery.take_ready().is_empty());
        assert!(delivery.is_awaited("A"));
        assert!(delivery.page_read(page("A")));
        let ready = delivery.take_ready();
        assert_eq!(
            kinds(&ready),
            vec![
                (EventKind::DocumentLoaded, None),
                (EventKind::FocusChanged, Some("A".to_owned())),
                (EventKind::NameChanged, None),
            ]
        );
        assert_eq!(ready.first().map(|r| r.at_ms), Some(10));
        assert!(!delivery.is_holding());
    }

    /// The focus leaving for another application before the page is read: the page is not given
    /// (it would be read as a new load over the editor), and what waited goes on at once.
    #[test]
    fn a_focus_elsewhere_cancels_the_page() {
        let mut delivery = Delivery::default();
        delivery.ask("A".to_owned(), 10);
        delivery.push(record(EventKind::FocusChanged, Some("A"), 10), true);
        // GTK's twin focus, not read, cancels nothing.
        assert!(delivery
            .push(record(EventKind::FocusChanged, None, 15), false)
            .is_empty());
        let cancelled = delivery.push(record(EventKind::FocusChanged, None, 30), true);
        assert_eq!(cancelled, vec!["A".to_owned()]);
        assert_eq!(delivery.take_ready().len(), 3);
        assert!(!delivery.page_read(page("A")));
        assert!(delivery.take_ready().is_empty());
    }

    /// A page waited for too long is given up, and what waited goes on.
    #[test]
    fn a_page_too_long_is_given_up() {
        let mut delivery = Delivery::default();
        delivery.ask("A".to_owned(), 0);
        delivery.push(record(EventKind::FocusChanged, Some("A"), 0), true);
        assert!(delivery.expire(HOLD_LIMIT_MS).is_empty());
        assert_eq!(delivery.expire(HOLD_LIMIT_MS + 1), vec!["A".to_owned()]);
        assert_eq!(delivery.take_ready().len(), 1);
    }
}
