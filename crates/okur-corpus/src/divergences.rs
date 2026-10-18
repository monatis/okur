//! Accepted divergences: where Okur differs from JAWS on purpose. Parity means JAWS's
//! intended behavior, not its accidents; when JAWS says something by a side effect or gets it
//! wrong, Okur keeps the right answer and the difference is written down here, with why and
//! who decided. This table is the one list; `docs/references/accepted-divergences.md`
//! explains each entry for readers, and a test keeps the two in step.
//!
//! An entry marks only what differs, so the rest of a step is still compared:
//! - [`Difference::JawsAlsoSays`]: a phrase JAWS says and Okur does not. It is taken out of
//!   JAWS's transcripts before they are compared.
//! - [`Difference::OkurAlsoSays`]: a phrase Okur says and JAWS does not. It is taken out of
//!   Okur's speech before it is compared.
//! - [`Difference::Step`]: the whole step differs. It is not counted in parity, and a step
//!   that comes to match JAWS fails the check, so a stale entry cannot linger.
//!
//! Every comparison of Okur with JAWS (`okur parity` on every layer, `okur live-parity`) goes
//! through [`compare`].

use crate::parity::normalize;

/// What an accepted divergence lets differ.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Difference {
    JawsAlsoSays(&'static str),
    OkurAlsoSays(&'static str),
    Step,
}

/// One decision: the steps it covers (`<scenario_id>/<step_id>`), what differs, why, and who
/// decided when.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Divergence {
    pub steps: &'static [&'static str],
    pub difference: Difference,
    pub why: &'static str,
    pub decided: &'static str,
}

/// Writes the table: `difference => [steps], why, decided;` per decision.
macro_rules! divergences {
    ($($difference:expr => [$($step:literal),+ $(,)?], $why:literal, $decided:literal;)*) => {
        /// Every accepted divergence.
        pub const DIVERGENCES: &[Divergence] = &[$(Divergence {
            steps: &[$($step),+],
            difference: $difference,
            why: $why,
            decided: $decided,
        }),*];
    };
}

divergences! {
    Difference::JawsAlsoSays("JAWS Version 2025.2505.43") => [
        "222-web-links-list/s1",
        "223-web-links-list-escape/s1",
        "224-web-headings-list/s1",
        "225-web-form-fields-list/s1",
        "226-web-element-lists/s1",
        "226-web-element-lists/s3",
    ],
    "JAWS's list dialogs belong to its own process (jfw.exe), and JAWS reads the version on its \
     main window as one of them opens: a side effect, not a feature to copy.",
    "the owner, 2026-10-18";

    Difference::OkurAlsoSays("\u{1f600}") => [
        "312-desktop-virtualize-control/s3",
        "312-desktop-virtualize-control/s4",
        "312-desktop-virtualize-control/s9",
        "312-desktop-virtualize-control/s14",
    ],
    "JAWS's Virtualize Current Control drops the emoji at the end of a line (and the \
     non-breaking space before it), though JAWS reads it in the field itself [obs, 313/s4]; \
     text copied from Okur's viewer keeps it.",
    "proposed by the parity gap audit, 2026-10-18; for the owner's review";

    Difference::Step => [
        "313-desktop-say-font/s1",
        "313-desktop-say-font/s2",
        "313-desktop-say-font/s3",
        "313-desktop-say-font/s4",
        "313-desktop-say-font/s5",
        "313-desktop-say-font/s6",
        "313-desktop-say-font/s7",
        "313-desktop-say-font/s9",
    ],
    "JAWS finds no font in a WinForms text box (\"No font\"), though it finds Notepad's \
     [obs, 314/s1]; Okur names it (\"Font is Consolas 11 point\"). Pressed twice, Okur then \
     shows it in the Virtual Viewer as JAWS does when it finds a font, so the scenario's later \
     keys act in the viewer, not the field.",
    "proposed by the parity gap audit, 2026-10-18; for the owner's review";
}

/// The divergence covering `step`, if any.
pub fn for_step(step: &str) -> Option<&'static Divergence> {
    DIVERGENCES.iter().find(|d| d.steps.contains(&step))
}

/// How Okur's speech for a step compares with JAWS's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Comparison {
    Matched,
    Differs,
    /// A whole-step divergence: not counted.
    Accepted,
}

/// Compares Okur's speech for `step` with JAWS's transcripts, whitespace normalized, with the
/// step's accepted divergence applied.
pub fn compare(step: &str, okur: &str, jaws: &[String]) -> Comparison {
    let difference = for_step(step).map(|d| d.difference);
    if difference == Some(Difference::Step) {
        return Comparison::Accepted;
    }
    let without = |text: &str, phrase: Option<&str>| match phrase {
        Some(phrase) => normalize(&text.replace(phrase, " ")),
        None => normalize(text),
    };
    let (jaws_extra, okur_extra) = match difference {
        Some(Difference::JawsAlsoSays(phrase)) => (Some(phrase), None),
        Some(Difference::OkurAlsoSays(phrase)) => (None, Some(phrase)),
        _ => (None, None),
    };
    let ours = without(okur, okur_extra);
    if jaws.iter().any(|v| without(v, jaws_extra) == ours) {
        Comparison::Matched
    } else {
        Comparison::Differs
    }
}

/// Every divergence still describes the corpus: its steps exist; a phrase JAWS also says is in
/// JAWS's transcripts and one Okur also says is in Okur's speech; and a whole step does not
/// match JAWS exactly. `steps` are the replayed steps: (key, Okur's speech, JAWS's
/// transcripts).
pub fn check(steps: &[(&str, &str, &[String])]) -> Result<(), String> {
    let mut stale = Vec::new();
    for divergence in DIVERGENCES {
        for &key in divergence.steps {
            let Some(&(_, okur, jaws)) = steps.iter().find(|(k, _, _)| *k == key) else {
                stale.push(format!("{key}: no such step"));
                continue;
            };
            match divergence.difference {
                Difference::JawsAlsoSays(phrase) if !jaws.iter().any(|v| v.contains(phrase)) => {
                    stale.push(format!("{key}: JAWS no longer says {phrase:?}"));
                }
                Difference::OkurAlsoSays(phrase) if !okur.contains(phrase) => {
                    stale.push(format!("{key}: Okur no longer says {phrase:?}"));
                }
                Difference::Step if jaws.iter().any(|v| normalize(v) == normalize(okur)) => {
                    stale.push(format!("{key}: Okur now matches JAWS"));
                }
                _ => {}
            }
        }
    }
    if stale.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "stale accepted divergences (update crates/okur-corpus/src/divergences.rs and \
             docs/references/accepted-divergences.md): {}",
            stale.join("; ")
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jaws(texts: &[&str]) -> Vec<String> {
        texts.iter().map(|t| (*t).to_owned()).collect()
    }

    /// A phrase JAWS also says is left out of JAWS's transcript; the rest must still match.
    #[test]
    fn only_the_accepted_phrase_is_left_out() {
        let recorded = jaws(&["JAWS Version 2025.2505.43 \nLinks List dialog\nlinks List view"]);
        let step = "222-web-links-list/s1";
        assert_eq!(
            compare(step, "Links List dialog links List view", &recorded),
            Comparison::Matched
        );
        assert_eq!(
            compare(step, "Links List dialog", &recorded),
            Comparison::Differs
        );
        // Elsewhere the phrase counts.
        assert_eq!(
            compare(
                "999-elsewhere/s1",
                "Links List dialog links List view",
                &recorded
            ),
            Comparison::Differs
        );
    }

    #[test]
    fn a_whole_step_divergence_is_not_counted() {
        assert_eq!(
            compare(
                "313-desktop-say-font/s1",
                "Font is Consolas 11 point",
                &jaws(&["No font"])
            ),
            Comparison::Accepted
        );
    }

    #[test]
    fn a_stale_divergence_is_reported() {
        let empty: Vec<String> = Vec::new();
        let steps: Vec<(&str, &str, &[String])> = vec![("222-web-links-list/s1", "", &empty)];
        let result = check(&steps);
        assert!(
            result
                .as_ref()
                .is_err_and(|e| e.contains("222-web-links-list/s1: JAWS no longer says")
                    && e.contains("313-desktop-say-font/s1: no such step")),
            "{result:?}"
        );
    }
}
