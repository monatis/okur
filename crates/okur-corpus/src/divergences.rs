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
    /// JAWS speaks its own windows (its main window, the list, the browser's title) before
    /// the line that names the dialog it opens: what comes before that line is left out.
    JawsSaysBeforeItsDialog,
    Step,
}

/// The role mark JAWS says after a dialog's title ("Select a Button  ‹ dialog ›").
const DIALOG_MARK: &str = "\u{1d} dialog \u{1e}";

/// JAWS's transcript from the line that names its dialog on, if it names one after saying
/// something else.
fn from_its_dialog(text: &str) -> Option<&str> {
    let mark = text.find(DIALOG_MARK)?;
    let line = text.get(..mark)?.rfind('\n').map_or(0, |n| n + 1);
    (line > 0).then(|| text.get(line..)).flatten()
}

/// The same on the spoken layer, where JAWS's synthesizer calls are joined by spaces and the
/// dialog's role has no mark: JAWS's transcript from the last place it says what Okur's
/// speech begins with, the dialog's title and "dialog" ("Links List dialog").
fn spoken_from_its_dialog(text: &str, ours: &str) -> Option<String> {
    const DIALOG: &str = " dialog";
    let named = ours.get(..ours.find(DIALOG)? + DIALOG.len())?;
    let text = normalize(text);
    let at = text.rfind(named).filter(|&at| at > 0)?;
    text.get(at..).map(str::to_owned)
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
    Difference::JawsSaysBeforeItsDialog => [
        "222-web-links-list/s1",
        "223-web-links-list-escape/s1",
        "226-web-element-lists/s3",
        "231-web-list-radio-buttons/s1",
        "232-web-list-buttons/s1",
        "224-web-headings-list/s1",
        "225-web-form-fields-list/s1",
        "226-web-element-lists/s1",
        "227-web-form-fields-list-items/s1",
        "228-web-form-fields-list-edit/s1",
        "230-web-element-lists-items/s1",
        "233-web-list-combo-boxes/s1",
        "234-web-list-edits/s1",
        "235-web-list-check-boxes/s1",
        "236-web-list-graphics/s1",
        "239-web-list-paragraphs/s1",
        "240-web-list-block-quotes/s1",
        "241-web-list-regions/s1",
        "242-web-list-tables/s1",
        "243-web-list-glance-highlights/s1",
        "244-web-list-divisions/s1",
        "245-web-list-tabs/s1",
        "247-web-list-buttons-enter/s1",
        "248-web-list-combo-boxes-enter/s1",
        "249-web-list-edits-enter/s1",
        "250-web-list-check-boxes-enter/s1",
        "251-web-list-graphics-enter/s1",
        "252-web-list-lists-enter/s1",
        "254-web-list-paragraphs-enter/s1",
        "255-web-list-block-quotes-enter/s1",
        "257-web-list-tables-enter/s1",
        "258-web-list-glance-highlights-enter/s1",
        "259-web-list-divisions-enter/s1",
        "260-web-list-tabs-enter/s1",
        "511-desktop-window-list-and-tray/s4",
    ],
    "Opening one of its list dialogs, JAWS's speech passes through its own windows before it \
     names the dialog: its main window (its version, a list's tutor message) and the browser's \
     title, or the dialog's list read before the dialog itself. Okur's dialog opens without \
     them. Some trials of the same keys have none of it.",
    "the owner's ruling on JAWS's version, 2026-10-18";

    Difference::JawsAlsoSays("Web App Patterns - Google Chrome") => [
        "226-web-element-lists/s1",
    ],
    "Opening Virtual HTML Features, JAWS's speech passed through the browser's window (its \
     title) on the way to its dialog. The same keys recorded again (230/s1) have none of it.",
    "the owner's ruling on JAWS's version, 2026-10-18";

    Difference::JawsAlsoSays(",") => ["230-web-element-lists-items/s18"],
    "Virtual HTML Features shows \"Radio Buttons List,\" with a stray comma, where every other \
     list is \"... List\": a typo in JAWS's text, not a name to copy.",
    "the owner's rule on JAWS's accidents, 2026-10-18";

    Difference::JawsAlsoSays("Unloading JAWS") => ["571-desktop-quit-question/s1"],
    "Insert+F4 says \"Unloading JAWS\" before it asks whether to quit, and JAWS stays when \
     the question is answered no (571/s3): it is not unloading yet. Okur asks first and says \
     its exit words only when it exits.",
    "the agent under the owner's rule on JAWS's accidents, 2026-10-20";

    Difference::OkurAlsoSays("\u{1f600}") => [
        "312-desktop-virtualize-control/s3",
        "312-desktop-virtualize-control/s4",
        "312-desktop-virtualize-control/s9",
        "312-desktop-virtualize-control/s14",
    ],
    "JAWS's Virtualize Current Control drops the emoji at the end of a line (and the \
     non-breaking space before it), though JAWS reads it in the field itself [obs, 313/s4]; \
     text copied from Okur's viewer keeps it.",
    "proposed by the parity gap audit, accepted by the owner, 2026-10-19";

    Difference::OkurAlsoSays("\u{1f600}") => ["504-desktop-edit-selection-and-clipboard/s22"],
    "JAWS's Say Bottom Line of Window (Insert+PageDown) in the editor fixture drops the emoji \
     and the non-breaking space that end the last line, as its control view does (312), though \
     JAWS reads the emoji in the field itself [obs, 313/s4].",
    "the owner's rule on JAWS's accidents, 2026-10-18";

    Difference::Step => [
        "526-desktop-screen-sensitive-help/s1",
        "526-desktop-screen-sensitive-help/s5",
        "526-desktop-screen-sensitive-help/s10",
        "526-desktop-screen-sensitive-help/s13",
        "526-desktop-screen-sensitive-help/s21",
        "527-desktop-notepad-help-keys/s1",
        "527-desktop-notepad-help-keys/s3",
        "527-desktop-notepad-help-keys/s5",
        "531-web-screen-sensitive-help/s2",
        "531-web-screen-sensitive-help/s5",
        "531-web-screen-sensitive-help/s8",
    ],
    "JAWS's help texts (Insert+F1, Insert+H, Insert+W) are Freedom Scientific's writing. \
     Okur's help keys open the Virtual Viewer as JAWS's do, with JAWS's hot keys link and \
     closing line, and Escape reads the focus as JAWS does; the help itself is in Okur's \
     own words, and Insert+H lists Okur's keys.",
    "the clean-room rule (AGENTS.md section 1.4), 2026-10-19";

    Difference::JawsAlsoSays("There are 1 Smart Glance Highlights on this page.") => [
        "539-web-page-load-summary-tables-navigation/s1",
    ],
    "JAWS's Smart Glance: see the next entry.",
    "the agent with the gaps report's row 20, 2026-10-20";

    Difference::JawsAlsoSays("There are 2 Smart Glance Highlights on this page.") => [
        "507-web-page-load-and-numpad-plus/s1",
    ],
    "JAWS's Smart Glance: see the next entry.",
    "the agent with the gaps report's row 20, 2026-10-20";

    Difference::JawsAlsoSays("There are 4 Smart Glance Highlights on this page.") => [
        "538-web-page-load-summary-news-article/s1",
    ],
    "After a page's summary JAWS counts its Smart Glance highlights, the content its own \
     heuristic picks as a page's most important, which no observation defines and Okur does \
     not have. Okur's page-load summary stops at the regions, headings and links.",
    "the agent with the gaps report's row 20, 2026-10-20";

    Difference::JawsAlsoSays("Column 65535, Row 65535") => [
        "545-desktop-data-grid/s2",
        "545-desktop-data-grid/s3",
        "545-desktop-data-grid/s4",
        "545-desktop-data-grid/s5",
        "545-desktop-data-grid/s6",
        "545-desktop-data-grid/s7",
        "545-desktop-data-grid/s8",
        "545-desktop-data-grid/s10",
        "545-desktop-data-grid/s11",
    ],
    "Moving between a WinForms data grid's cells, JAWS adds \"Column 65535, Row 65535\" after \
     each cell, in every trial: 65535 is -1 as an unsigned 16-bit number, a place JAWS failed \
     to find. The cell's name already gives its column and row (\"Quantity Row 1\").",
    "the owner's rule on JAWS's accidents, 2026-10-18";

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
    "proposed by the parity gap audit, accepted by the owner, 2026-10-19";

    Difference::Step => [
        "253-web-list-articles-enter/s3",
        "253-web-list-articles-enter/s4",
        "253-web-list-articles-enter/s5",
    ],
    "Enter on the article chosen in Select an Article took JAWS to the page's top, \"banner\", \
     in every trial, not to the article; Down then read the banner's link. The list is for \
     moving to an article, and JAWS's other lists move to what was chosen (252, 254, 255). \
     Okur moves to the article: \"article\", then its heading.",
    "the owner's rule on JAWS's accidents, 2026-10-18";

    Difference::Step => ["280-web-forms-arrows-auto-forms/s7"],
    "With Auto Forms Mode, Down onto the email field said it \"Required\" in every trial, \
     though nothing makes it required (no required attribute; UIA's IsRequiredForForm and \
     ARIA's required are false in the trace), and JAWS's Tab onto it (204/s2) does not: a \
     wrong fact. Okur says the field without it.",
    "the owner's rule on JAWS's accidents, 2026-10-18";

    Difference::Step => [
        "280-web-forms-arrows-auto-forms/s21",
        "280-web-forms-arrows-auto-forms/s22",
        "280-web-forms-arrows-auto-forms/s23",
        "280-web-forms-arrows-auto-forms/s24",
        "280-web-forms-arrows-auto-forms/s25",
        "280-web-forms-arrows-auto-forms/s26",
    ],
    "With Auto Forms Mode, Down out of the empty text area went to the page's last line, \
     \"main region end\", skipping the check boxes, the link and the buttons after it; Up from \
     there read them (s29-s33). JAWS's documentation has the arrow leave the field for the \
     page's next line (Surf's Up, \"Using Forms with JAWS\"), which Okur does.",
    "the owner's rule on JAWS's accidents, 2026-10-18";

    Difference::Step => [
        "282-web-forms-arrows-auto-forms-up/s8",
        "282-web-forms-arrows-auto-forms-up/s9",
        "282-web-forms-arrows-auto-forms-up/s10",
        "282-web-forms-arrows-auto-forms-up/s11",
        "282-web-forms-arrows-auto-forms-up/s12",
        "282-web-forms-arrows-auto-forms-up/s13",
        "282-web-forms-arrows-auto-forms-up/s14",
        "282-web-forms-arrows-auto-forms-up/s15",
        "282-web-forms-arrows-auto-forms-up/s16",
        "282-web-forms-arrows-auto-forms-up/s17",
        "282-web-forms-arrows-auto-forms-up/s18",
        "282-web-forms-arrows-auto-forms-up/s19",
        "282-web-forms-arrows-auto-forms-up/s20",
        "282-web-forms-arrows-auto-forms-up/s21",
        "282-web-forms-arrows-auto-forms-up/s22",
        "282-web-forms-arrows-auto-forms-up/s23",
        "282-web-forms-arrows-auto-forms-up/s24",
        "282-web-forms-arrows-auto-forms-up/s25",
    ],
    "The same accident going up: with Auto Forms Mode, Up out of the empty text area went to \
     the page's first lines (\"main region\", then the page's title), skipping the form above \
     it, and every Up after stayed there. Okur goes on to the line before the field.",
    "the owner's rule on JAWS's accidents, 2026-10-18";

    Difference::Step => [
        "517-web-speech-layer-keys/s4",
        "517-web-speech-layer-keys/s10",
        "570-web-speech-history-clear-and-copy/s10",
    ],
    "Escape out of the Speech History left JAWS outside the page: it said \"blank\" in one trial \
     and nothing in the others, and Down later said nothing with speech on (its synthesizer \
     log has only the harness's own message). After its help viewer the same Escape reads the \
     page's line (531/s3), which Okur does here too.",
    "the owner's rule on JAWS's accidents, 2026-10-18";

    Difference::Step => [
        "285-web-text-area-arrows-auto-forms/s19",
        "285-web-text-area-arrows-auto-forms/s20",
        "285-web-text-area-arrows-auto-forms/s21",
        "285-web-text-area-arrows-auto-forms/s22",
        "285-web-text-area-arrows-auto-forms/s23",
        "285-web-text-area-arrows-auto-forms/s24",
    ],
    "The same accident from a text area holding text: with Auto Forms Mode, Up from its first \
     line went to \"main region\", then the page's title for every Up after, in every trial of \
     round 55, skipping the page's lines above the field. Okur goes on to the line before the \
     field (its label, \"Text\") and up through the page.",
    "the owner's rule on JAWS's accidents, 2026-10-18";

    Difference::JawsSaysBeforeItsDialog => [
        "525-desktop-open-dialog/s1",
        "532-desktop-dialog-default-button-and-hot-key/s1",
    ],
    "Opening the Open dialog, JAWS says the file name field's name, \"File name:\", before the \
     dialog, then again with the field, in every trial of both scenarios; the Save As dialog \
     opens without it (524/s2). The field's name arrives before the dialog JAWS has not yet \
     announced. Okur says the dialog, then the field.",
    "the owner's rule on JAWS's accidents, 2026-10-18";

    Difference::Step => [
        "524-desktop-save-as-dialog/s11",
        "532-desktop-dialog-default-button-and-hot-key/s9",
    ],
    "Escape out of a common file dialog (Save As, Open) returns to the button that opened it, \
     and JAWS says it \"Unavailable\", without its window's title, in every trial: the owner \
     window is still disabled by the modal dialog as JAWS reads the focus, and the button is \
     enabled a moment later. Other dialogs return with the window's title (306/s3, 537/s10). \
     Okur says the window's title and the button.",
    "the owner's rule on JAWS's accidents, 2026-10-18";

    Difference::Step => ["541-desktop-notepad-font-dialog/s2"],
    "Opening Notepad's Font dialog, JAWS says the font, \"Consolas\", before the dialog, then \
     again in the field (in 3 of 3 trials of round 56, 2 of 3 earlier): the dialog fills its \
     font field 400 ms after the field takes the focus, and JAWS reads that text as it arrives, \
     ahead of the dialog it has not yet announced. Okur says the dialog, then the field with its \
     font once.",
    "the owner's rule on JAWS's accidents, 2026-10-18";

    Difference::Step => [
        "522-desktop-wordpad-reading/s1",
        "522-desktop-wordpad-reading/s2",
        "522-desktop-wordpad-reading/s3",
        "522-desktop-wordpad-reading/s4",
        "522-desktop-wordpad-reading/s5",
        "522-desktop-wordpad-reading/s6",
        "522-desktop-wordpad-reading/s7",
        "522-desktop-wordpad-reading/s8",
        "522-desktop-wordpad-reading/s9",
        "522-desktop-wordpad-reading/s10",
        "522-desktop-wordpad-reading/s11",
        "522-desktop-wordpad-reading/s12",
        "522-desktop-wordpad-reading/s13",
        "522-desktop-wordpad-reading/s14",
        "522-desktop-wordpad-reading/s16",
        "522-desktop-wordpad-reading/s17",
        "522-desktop-wordpad-reading/s18",
    ],
    "JAWS reads nothing of WordPad's document text on old-pc: the arrows, the word keys, Say \
     Line and Say All are silent or say a list item's bullet alone, Say Selected Text says \
     \"Nothing selected\" with text selected, and Ctrl+Home and Ctrl+End say \"top of file\" \
     and \"Bottom of file\" without the line, in every trial, though it reads the font there \
     (s15). Okur reads the text.",
    "the owner's rule on JAWS's accidents, 2026-10-18";
}

/// The divergence covering `step`, if any.
pub fn for_step(step: &str) -> impl Iterator<Item = &'static Divergence> + '_ {
    DIVERGENCES.iter().filter(move |d| d.steps.contains(&step))
}

/// How Okur's speech for a step compares with JAWS's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Comparison {
    Matched,
    Differs,
    /// A whole-step divergence: not counted.
    Accepted,
}

/// Compares Okur's speech for `step` with JAWS's transcripts, whitespace normalized, with every
/// accepted divergence of the step applied.
pub fn compare(step: &str, okur: &str, jaws: &[String]) -> Comparison {
    let differences: Vec<Difference> = for_step(step).map(|d| d.difference).collect();
    if differences.contains(&Difference::Step) {
        return Comparison::Accepted;
    }
    let without = |text: &str, okur_side: bool, ours: &str| {
        let removed = differences
            .iter()
            .fold(text.to_owned(), |text, difference| {
                match (difference, okur_side) {
                    (Difference::JawsAlsoSays(phrase), false)
                    | (Difference::OkurAlsoSays(phrase), true) => text.replace(phrase, " "),
                    (Difference::JawsSaysBeforeItsDialog, false) => from_its_dialog(&text)
                        .map(str::to_owned)
                        .or_else(|| spoken_from_its_dialog(&text, ours))
                        .unwrap_or(text),
                    _ => text,
                }
            });
        normalize(&removed)
    };
    let ours = without(okur, true, "");
    if jaws.iter().any(|v| without(v, false, &ours) == ours) {
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
                Difference::JawsSaysBeforeItsDialog
                    if !jaws.iter().any(|v| from_its_dialog(v).is_some()) =>
                {
                    stale.push(format!("{key}: JAWS says nothing before its dialog"));
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

    /// On the spoken layer JAWS's speech before its dialog is left out from where it names the
    /// dialog as Okur does; what follows must still match [live11, 222/s1].
    #[test]
    fn spoken_speech_before_the_dialog_is_left_out() {
        let step = "222-web-links-list/s1";
        let okur = "Links List dialog links List view News 1 of 4";
        let recorded = jaws(&[
            "links List view News 1 of 4 To change the selection press Up or Down Arrow. \
             Links List dialog links List view News 1 of 4",
        ]);
        assert_eq!(compare(step, okur, &recorded), Comparison::Matched);
        assert_eq!(
            compare(step, "Links List dialog links", &recorded),
            Comparison::Differs
        );
        // Elsewhere it counts.
        assert_eq!(
            compare("101-article-read-down/s1", okur, &recorded),
            Comparison::Differs
        );
    }

    /// A phrase JAWS also says is left out of JAWS's transcript; the rest must still match.
    #[test]
    fn only_the_accepted_phrase_is_left_out() {
        let recorded = jaws(&["Web App Patterns - Google Chrome \nList1 List view"]);
        let step = "226-web-element-lists/s1";
        assert_eq!(
            compare(step, "List1 List view", &recorded),
            Comparison::Matched
        );
        assert_eq!(compare(step, "List1", &recorded), Comparison::Differs);
        // Elsewhere the phrase counts.
        assert_eq!(
            compare("999-elsewhere/s1", "List1 List view", &recorded),
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
        let steps: Vec<(&str, &str, &[String])> = vec![("226-web-element-lists/s1", "", &empty)];
        let result = check(&steps);
        assert!(
            result.as_ref().is_err_and(|e| e
                .contains("226-web-element-lists/s1: JAWS no longer says")
                && e.contains("313-desktop-say-font/s1: no such step")),
            "{result:?}"
        );
    }
}
