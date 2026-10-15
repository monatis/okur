//! Responsiveness from a live run's debug log (`sr-cli run --debug-log`): how long keys and
//! events took at each stage, as counts and percentiles (`sr-cli latency`).
//!
//! The live screen reader logs one line per timed step:
//! - `key` (`waited_us`, `held_us`): a key waiting for the engine, then handled by it, all on
//!   the keyboard hook;
//! - `event read` (`event`, `us`): the platform reading an accessibility event;
//! - `event handled` (`event`, `us`): the engine handling it;
//! - `page loaded`, `page refreshed` (`ms`): capturing a page; `page buffer built` (`ms`);
//! - `focused text read` (`ms`): a text read for the Virtual Viewer; `text context read`
//!   (`ms`): an edit key's text, read once the application has acted.

/// Timings of one kind of step, in microseconds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Category {
    pub name: String,
    pub micros: Vec<u64>,
}

impl Category {
    /// The `percent`th percentile (nearest rank), in microseconds.
    pub fn percentile(&self, percent: u64) -> u64 {
        let mut sorted = self.micros.clone();
        sorted.sort_unstable();
        let n = sorted.len() as u64;
        if n == 0 {
            return 0;
        }
        let rank = (percent * n).div_ceil(100).clamp(1, n);
        sorted
            .get(usize::try_from(rank - 1).unwrap_or(0))
            .copied()
            .unwrap_or(0)
    }
}

/// A field's value on a log line (`name=value`, or `name="quoted value"`).
fn field<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let key = format!(" {name}=");
    let start = line.find(&key)? + key.len();
    let rest = line.get(start..)?;
    match rest.strip_prefix('"') {
        Some(quoted) => quoted.split('"').next(),
        None => rest.split_whitespace().next(),
    }
}

fn number(line: &str, name: &str) -> Option<u64> {
    field(line, name)?.parse().ok()
}

/// The message of a log line: the words after the target (`sr_cli: `) and before the first
/// field.
fn message(line: &str) -> Option<String> {
    let after_target = line.split_once(": ")?.1;
    let words: Vec<&str> = after_target
        .split(' ')
        .take_while(|word| !word.contains('='))
        .collect();
    Some(words.join(" "))
}

/// The timed steps in `log`, by kind, in the order kinds first appear.
pub fn summarize(log: &str) -> Vec<Category> {
    let mut categories: Vec<Category> = Vec::new();
    let mut add = |name: String, micros: u64| match categories.iter_mut().find(|c| c.name == name) {
        Some(category) => category.micros.push(micros),
        None => categories.push(Category {
            name,
            micros: vec![micros],
        }),
    };
    for line in log.lines() {
        let Some(message) = message(line) else {
            continue;
        };
        let event = || field(line, "event").unwrap_or("?");
        match message.as_str() {
            "key" => {
                if let (Some(waited), Some(held)) =
                    (number(line, "waited_us"), number(line, "held_us"))
                {
                    add("key: wait + engine".to_owned(), waited + held);
                }
            }
            "event read" => {
                if let Some(us) = number(line, "us") {
                    add(format!("event read: {}", event()), us);
                }
            }
            "event handled" => {
                if let Some(us) = number(line, "us") {
                    add(format!("event handled: {}", event()), us);
                }
            }
            "page loaded" | "page refreshed" | "page buffer built" | "focused text read"
            | "text context read" => {
                if let Some(ms) = number(line, "ms") {
                    add(message.clone(), ms * 1000);
                }
            }
            _ => {}
        }
    }
    categories
}

/// The summary as a table, in milliseconds.
pub fn render(categories: &[Category]) -> String {
    let width = categories
        .iter()
        .map(|c| c.name.chars().count())
        .max()
        .unwrap_or(0)
        .max(4);
    let ms = |us: u64| format!("{:.1}", us as f64 / 1000.0);
    let mut out = format!(
        "{:<width$}  {:>6}  {:>8}  {:>8}  {:>8}  {:>8}  (ms)\n",
        "step", "count", "p50", "p90", "p99", "max"
    );
    for category in categories {
        out.push_str(&format!(
            "{:<width$}  {:>6}  {:>8}  {:>8}  {:>8}  {:>8}\n",
            category.name,
            category.micros.len(),
            ms(category.percentile(50)),
            ms(category.percentile(90)),
            ms(category.percentile(99)),
            ms(category.percentile(100)),
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOG: &str = "\
2026-10-15T07:34:25.053673Z DEBUG sr_cli: key key=Insert text=None ctrl=false action=Consume mode=NativeControl forms=false waited_us=120 held_us=380
2026-10-15T07:34:25.084453Z DEBUG sr_cli: key key=Down text=None ctrl=false action=Consume mode=NativeControl forms=false waited_us=0 held_us=1500
2026-10-15T07:34:25.090000Z DEBUG sr_platform::event_trace::capture: event read event=\"focus\" us=48000
2026-10-15T07:34:25.091000Z DEBUG sr_cli: event handled event=\"focus\" us=900
2026-10-15T07:34:25.092000Z DEBUG sr_platform::events::windows: page loaded title=Inbox - Mail ms=820
2026-10-15T07:34:25.093000Z DEBUG sr_cli: spoken for event msg=Inbox
";

    #[test]
    fn timings_are_grouped_by_step_and_event() {
        let categories = summarize(LOG);
        let names: Vec<&str> = categories.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "key: wait + engine",
                "event read: focus",
                "event handled: focus",
                "page loaded"
            ]
        );
        let keys = categories.first().map(|c| c.micros.clone());
        assert_eq!(keys, Some(vec![500, 1500]));
        assert_eq!(
            categories.last().map(|c| c.micros.clone()),
            Some(vec![820_000])
        );
    }

    #[test]
    fn percentiles_are_nearest_rank() {
        let category = Category {
            name: "x".to_owned(),
            micros: (1..=100).rev().collect(),
        };
        assert_eq!(category.percentile(50), 50);
        assert_eq!(category.percentile(90), 90);
        assert_eq!(category.percentile(100), 100);
        assert_eq!(category.percentile(0), 1);
    }

    #[test]
    fn the_table_is_in_milliseconds() {
        let table = render(&summarize(LOG));
        assert!(table.contains("page loaded"), "{table}");
        assert!(table.contains("820.0"), "{table}");
    }
}
