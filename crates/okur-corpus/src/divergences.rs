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
//! Every comparison of Okur with JAWS (`okur-lab parity` on every layer, `okur-lab live-parity`) goes
//! through [`compare`], which reads both sides as [`comparable`]: whitespace normalized, and
//! JAWS's name for itself read as Okur's, a rule for the whole corpus rather than a divergence.

use crate::parity::comparable;

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
    let line = dialog_line(text)?;
    (line > 0).then(|| text.get(line..)).flatten()
}

/// Where the line that names a dialog starts.
fn dialog_line(text: &str) -> Option<usize> {
    let mark = text.find(DIALOG_MARK)?;
    Some(text.get(..mark)?.rfind('\n').map_or(0, |n| n + 1))
}

/// [`from_its_dialog`] after what Okur says before its own dialog line, when JAWS says that
/// first too: "Virtual Find" before JAWS Find [obs, 506/s1].
fn from_its_dialog_after(text: &str, okur_before: &str) -> Option<String> {
    let rest = from_its_dialog(text)?;
    Some(match text.strip_prefix(okur_before) {
        Some(_) if !okur_before.is_empty() && !rest.starts_with(okur_before) => {
            format!("{okur_before}{rest}")
        }
        _ => rest.to_owned(),
    })
}

/// The same on the spoken layer, where JAWS's synthesizer calls are joined by spaces and the
/// dialog's role has no mark: JAWS's transcript from the last place it says what Okur's
/// speech begins with, the dialog's title and "dialog" ("Links List dialog").
fn spoken_from_its_dialog(text: &str, ours: &str) -> Option<String> {
    const DIALOG: &str = " dialog";
    let named = ours.get(..ours.find(DIALOG)? + DIALOG.len())?;
    let text = comparable(text);
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
        "576-web-element-lists-items-intermediate/s1",
        "586-web-element-lists-items-advanced/s1",
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
        "506-web-jaws-find/s1",
        "699-web-placemarker-list/s12",
        "701-web-placemarker-add-dialog/s4",
        "701-web-placemarker-add-dialog/s13",
        "703-web-placemarker-none/s2",
        "703-web-placemarker-none/s4",
        "704-web-placemarker-two/s3",
        "704-web-placemarker-two/s34",
        "704-web-placemarker-two/s38",
        "704-web-placemarker-two/s44",
        "560-web-word-list/s1",
        "560-web-word-list/s4",
        "900-web-word-list-advanced/s4",
        "764-web-word-list-a-word-followed/s1",
        "765-web-word-list-dialog-and-case/s1",
        "766-web-word-list-past-the-last/s1",
        "760-web-skim-reading-dialog/s1",
        "763-web-skim-reading-rule-types/s1",
        "769-web-skim-read-by-sentence/s1",
        "771-web-skim-reading-rule-saved/s1",
        "776-web-skim-reading-text-rules-controls/s1",
        "777-web-skim-reading-speak-unit-and-attributes/s1",
        "799-web-skim-read-by-bold/s1",
        "800-web-skim-reading-back-to-first-lines/s1",
        "803-web-skim-read-by-italic/s1",
        "804-web-skim-read-by-underline/s1",
        "805-web-skim-read-by-text-color/s1",
        "809-web-skim-read-by-strikeout/s1",
        "810-web-skim-read-by-double-strikeout/s1",
        "811-web-skim-read-by-superscript/s1",
        "812-web-skim-read-by-subscript/s1",
        "813-web-skim-read-by-shadow/s1",
        "815-web-skim-read-by-superscript-positions/s1",
        "816-web-skim-read-by-subscript-positions/s1",
        "826-desktop-notepad-clipboard-edges/s8",
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

    Difference::JawsAlsoSays(",") => [
        "230-web-element-lists-items/s18",
        "576-web-element-lists-items-intermediate/s18",
        "586-web-element-lists-items-advanced/s18",
    ],
    "Virtual HTML Features shows \"Radio Buttons List,\" with a stray comma, where every other \
     list is \"... List\": a typo in JAWS's text, not a name to copy.",
    "the owner's rule on JAWS's accidents, 2026-10-18";

    Difference::JawsAlsoSays("_Rules\nRules") => [
        "787-desktop-notification-rule-made-managed-deleted/s8",
        "787-desktop-notification-rule-made-managed-deleted/s10",
        "787-desktop-notification-rule-made-managed-deleted/s15",
        "787-desktop-notification-rule-made-managed-deleted/s18",
        "798-desktop-notification-rule-shorten/s18",
        "798-desktop-notification-rule-shorten/s20",
    ],
    "Manage Notification Rules says its list's label twice, \"Rules _Rules\", then \"Rules List \
     box\": the label's text as WPF holds it, its access key's underscore and all, before the \
     list. Okur says the list by its name once.",
    "the agent under the owner's rule on JAWS's accidents, 2026-10-22";

    Difference::JawsAlsoSays("Space run at column 12") => ["614-desktop-text-analysis-rules/s23"],
    "JAWS's text analysis names a run of spaces one column past the end of \"Hello!World\", \
     which has no space at all; it names none past the end of the other fourteen lines, \
     which end alike (614).",
    "the agent under the owner's rule on JAWS's accidents, 2026-10-20";

    Difference::JawsAlsoSays("Space run at column 17") => ["614-desktop-text-analysis-rules/s27"],
    "The same accident past the end of \"Mixed Case wORD.\" (614/s23's entry).",
    "the agent under the owner's rule on JAWS's accidents, 2026-10-20";

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

    Difference::OkurAlsoSays("\u{1f600}") => [
        "504-desktop-edit-selection-and-clipboard/s22",
        "599-desktop-edit-selection-and-clipboard-intermediate/s22",
        "609-desktop-edit-selection-and-clipboard-advanced/s22",
    ],
    "JAWS's Say Bottom Line of Window (Insert+PageDown) in the editor fixture drops the emoji \
     and the non-breaking space that end the last line, as its control view does (312), though \
     JAWS reads the emoji in the field itself [obs, 313/s4].",
    "the owner's rule on JAWS's accidents, 2026-10-18";

    Difference::OkurAlsoSays("2 of 3") => ["534-web-numpad-slash-check-box/s10"],
    "Say Line on a radio button JAWS's mouse click has just checked leaves out its place in \
     its group, which Say Line on a radio button says otherwise (246/s4, \"2 of 3\") and Down \
     onto one (534/s8). Okur says it.",
    "the owner's rule on JAWS's accidents, 2026-10-21";

    Difference::Step => [
        "526-desktop-screen-sensitive-help/s1",
        "526-desktop-screen-sensitive-help/s5",
        "526-desktop-screen-sensitive-help/s10",
        "526-desktop-screen-sensitive-help/s13",
        "526-desktop-screen-sensitive-help/s21",
        "601-desktop-screen-sensitive-help-intermediate/s1",
        "601-desktop-screen-sensitive-help-intermediate/s5",
        "601-desktop-screen-sensitive-help-intermediate/s10",
        "601-desktop-screen-sensitive-help-intermediate/s13",
        "601-desktop-screen-sensitive-help-intermediate/s21",
        "611-desktop-screen-sensitive-help-advanced/s1",
        "611-desktop-screen-sensitive-help-advanced/s5",
        "611-desktop-screen-sensitive-help-advanced/s10",
        "611-desktop-screen-sensitive-help-advanced/s13",
        "611-desktop-screen-sensitive-help-advanced/s21",
        "527-desktop-notepad-help-keys/s1",
        "527-desktop-notepad-help-keys/s3",
        "527-desktop-notepad-help-keys/s5",
        "531-web-screen-sensitive-help/s2",
        "531-web-screen-sensitive-help/s5",
        "531-web-screen-sensitive-help/s8",
        "634-desktop-keyboard-help-spoken/s2",
        "634-desktop-keyboard-help-spoken/s3",
        "634-desktop-keyboard-help-spoken/s4",
        "634-desktop-keyboard-help-spoken/s5",
        "634-desktop-keyboard-help-spoken/s6",
        "642-desktop-keyboard-help-second-press/s2",
        "642-desktop-keyboard-help-second-press/s3",
        "642-desktop-keyboard-help-second-press/s4",
        "642-desktop-keyboard-help-second-press/s5",
        "642-desktop-keyboard-help-second-press/s6",
    ],
    "JAWS's help texts (Insert+F1, Insert+H, Insert+W, and Keyboard Help's descriptions of \
     keys) are Freedom Scientific's writing. Okur's help keys open the Virtual Viewer as \
     JAWS's do, with JAWS's hot keys link and closing line, and Escape reads the focus as \
     JAWS does; the help itself is in Okur's own words, and Insert+H lists Okur's keys. In \
     Keyboard Help a key is named as JAWS names it, then described in Okur's words.",
    "the clean-room rule (AGENTS.md section 1.4), 2026-10-19";

    Difference::JawsAlsoSays("There are 1 Smart Glance Highlights on this page.") => [
        "539-web-page-load-summary-tables-navigation/s1",
    ],
    "JAWS's Smart Glance: see the next entry.",
    "the agent with the gaps report's row 20, 2026-10-20";

    Difference::JawsAlsoSays("1 Smart Glance Highlights") => [
        "857-web-page-load-summary-tables-navigation-intermediate/s1",
        "858-web-page-load-summary-tables-navigation-advanced/s1",
    ],
    "JAWS's Smart Glance, its short message above Beginner: see the entry for 538.",
    "the agent with round 259, 2026-10-23";

    Difference::JawsAlsoSays("2 Smart Glance Highlights") => [
        "847-web-page-load-and-numpad-plus-intermediate/s1",
        "848-web-page-load-and-numpad-plus-advanced/s1",
    ],
    "JAWS's Smart Glance, its short message above Beginner: see the entry for 538.",
    "the agent with round 259, 2026-10-23";

    Difference::JawsAlsoSays("4 Smart Glance Highlights") => [
        "855-web-page-load-summary-news-article-intermediate/s1",
        "856-web-page-load-summary-news-article-advanced/s1",
    ],
    "JAWS's Smart Glance, its short message above Beginner: see the entry for 538.",
    "the agent with round 259, 2026-10-23";

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

    Difference::JawsAlsoSays("irections") => [
        "677-article-say-all-by-sentence/s1",
    ],
    "Say All by sentence in the news article: JAWS says the table's two \"Directions\" links as \
     \"irections\", their first letter lost after the sentence \"to 9 p.m.\" before them, in \
     every trial. By line it says them whole (111). Okur says \"Directions\" (the next entry).",
    "the owner's rule on JAWS's accidents, 2026-10-18";

    Difference::OkurAlsoSays("Directions") => [
        "677-article-say-all-by-sentence/s1",
    ],
    "The links JAWS cuts to \"irections\" by sentence, said whole (see the entry before).",
    "the owner's rule on JAWS's accidents, 2026-10-18";

    Difference::JawsAlsoSays("Preview Bottom:") => [
        "544-desktop-notepad-page-setup/s13",
        "674-desktop-notepad-page-setup-intermediate/s13",
    ],
    "Say Window in Notepad's Page Setup dialog ends, in JAWS, with the Preview box's three \
     pictures of the page, read by names they do not have: \"Bottom:\" twice, the label \
     before them in the dialog, and \"Page Setup\", the dialog's title (the next entry). They \
     have no text. Okur reads no picture in Say Window, as JAWS reads none in the other \
     dialogs recorded.",
    "the owner's rule on JAWS's accidents, 2026-10-18";

    Difference::JawsAlsoSays("Preview Page Setup") => [
        "544-desktop-notepad-page-setup/s13",
        "674-desktop-notepad-page-setup-intermediate/s13",
    ],
    "The last of the Page Setup preview's pictures, named after the dialog's title (see the \
     entry before).",
    "the owner's rule on JAWS's accidents, 2026-10-18";

    Difference::JawsAlsoSays("\nBottom: \nBottom: \nPage Setup") => [
        "675-desktop-notepad-page-setup-advanced/s13",
    ],
    "The Page Setup preview's pictures at Advanced, without the box's name before each (see \
     the entries before).",
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
        "513-desktop-dialog-helpers/s2",
        "513-desktop-dialog-helpers/s8",
        "516-desktop-jaws-cursor/s8",
        "930-desktop-dialog-helpers-intermediate/s2",
        "930-desktop-dialog-helpers-intermediate/s8",
        "931-desktop-dialog-helpers-advanced/s2",
        "931-desktop-dialog-helpers-advanced/s8",
        "932-desktop-jaws-cursor-intermediate/s8",
        "933-desktop-jaws-cursor-advanced/s8",
    ],
    "At 125% (old-pc: 1920 by 1080 pixels, 120 DPI) JAWS gives the PC cursor's place 1.25 times \
     the screen's pixels: \"1045, 544\" for a button whose middle is at 836, 435, read from its \
     window by a tracer aware of each monitor's DPI (perf92's log: awareness 2, the window's own \
     rectangle). Near the screen's right edge JAWS's numbers would lie past it. Okur says the \
     screen's pixels, where its mouse keys put the pointer.",
    "the owner's rule on JAWS's accidents, 2026-10-18";

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

    Difference::Step => [
        "506-web-jaws-find/s8",
        "506-web-jaws-find/s9",
        "506-web-jaws-find/s10",
        "506-web-jaws-find/s11",
        "633-web-jaws-find-next-board/s9",
        "633-web-jaws-find-next-board/s10",
        "633-web-jaws-find-next-board/s11",
        "633-web-jaws-find-next-board/s12",
        "633-web-jaws-find-next-board/s13",
        "633-web-jaws-find-next-board/s14",
        "926-web-jaws-find-intermediate/s8",
        "926-web-jaws-find-intermediate/s9",
        "926-web-jaws-find-intermediate/s10",
        "926-web-jaws-find-intermediate/s11",
        "927-web-jaws-find-advanced/s8",
        "927-web-jaws-find-advanced/s9",
        "927-web-jaws-find-advanced/s10",
        "927-web-jaws-find-advanced/s11",
    ],
    "Right after JAWS Find found \"board\", F3 and Shift+F3 say nothing, in every trial of 506 \
     and 633, and the virtual cursor lands on no match: it stays on the match's line, or ends \
     a line or two above it, on lines without the word (633: \"published March 14, 2026\", \
     then \"Maria Lopez,\"), and not the same way in both. From the top of the page F3 finds \
     the match and says its line (633/s16), as JAWSFindNext, F3 in JAWS's key map, is to. \
     Okur's F3 and Shift+F3 always go to the next or previous match and say its line.",
    "the owner's rule on JAWS's accidents, 2026-10-20";

    Difference::Step => [
        "280-web-forms-arrows-auto-forms/s7",
        "632-web-field-returned-later/s10",
        "640-web-field-returned-later-intermediate/s10",
        "641-web-field-returned-later-advanced/s10",
    ],
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
        "863-web-speech-history-clear-and-copy-intermediate/s10",
        "864-web-speech-history-clear-and-copy-advanced/s10",
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
        "600-desktop-save-as-dialog-intermediate/s11",
        "610-desktop-save-as-dialog-advanced/s11",
        "532-desktop-dialog-default-button-and-hot-key/s9",
    ],
    "Escape out of a common file dialog (Save As, Open) returns to the button that opened it, \
     and JAWS says it \"Unavailable\", without its window's title, in every trial: the owner \
     window is still disabled by the modal dialog as JAWS reads the focus, and the button is \
     enabled a moment later. Other dialogs return with the window's title (306/s3, 537/s10). \
     Okur says the window's title and the button.",
    "the owner's rule on JAWS's accidents, 2026-10-18";

    Difference::Step => [
        "541-desktop-notepad-font-dialog/s2",
        "665-desktop-font-dialog-say-window/s2",
        "673-desktop-font-dialog-say-window-advanced/s2",
        "934-desktop-notepad-font-dialog-intermediate/s2",
        "935-desktop-notepad-font-dialog-advanced/s2",
    ],
    "Opening Notepad's Font dialog, JAWS says the font, \"Consolas\", before the dialog, then \
     again in the field (in 3 of 3 trials of round 56, 2 of 3 earlier): the dialog fills its \
     font field 400 ms after the field takes the focus, and JAWS reads that text as it arrives, \
     ahead of the dialog it has not yet announced. Okur says the dialog, then the field with its \
     font once.",
    "the owner's rule on JAWS's accidents, 2026-10-18";

    Difference::Step => [
        "684-desktop-say-all-by-sentence/s1",
        "684-desktop-say-all-by-sentence/s5",
        "685-desktop-say-all-by-paragraph/s1",
        "685-desktop-say-all-by-paragraph/s5",
    ],
    "Say All in a WinForms text box by sentence or by paragraph, from the line \"Say All reads \
     this. It has three sentences. The last one ends here.\": by sentence JAWS starts with the \
     text after the line before's \"P.M.\" (\"aB9 é €\"), then the line's last two sentences, \
     and stops before the text's last line; by paragraph it reads the text box's first line, \
     \"Hello, world. Is it done? Yes! Wait; maybe: no.\", in every trial. By line it reads \
     from the caret's line to the end (686). Okur reads from the caret's sentence or \
     paragraph to the end.",
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
        "523-desktop-wordpad-ribbon/s9",
        "970-desktop-wordpad-ribbon-lower/s14",
        "971-desktop-wordpad-ribbon-alt-twice/s5",
        "972-desktop-wordpad-ribbon-f10/s5",
        "973-desktop-wordpad-ribbon-key-tips/s6",
    ],
    "JAWS reads nothing of WordPad's document text on old-pc: the arrows, the word keys, Say \
     Line and Say All are silent or say a list item's bullet alone, Say Selected Text says \
     \"Nothing selected\" with text selected, and Ctrl+Home and Ctrl+End say \"top of file\" \
     and \"Bottom of file\" without the line, in every trial, though it reads the font there \
     (s15). Okur reads the text.",
    "the owner's rule on JAWS's accidents, 2026-10-18";

    Difference::Step => [
        "523-desktop-wordpad-ribbon/s2",
        "523-desktop-wordpad-ribbon/s3",
        "970-desktop-wordpad-ribbon-lower/s2",
        "970-desktop-wordpad-ribbon-lower/s3",
        "970-desktop-wordpad-ribbon-lower/s5",
        "972-desktop-wordpad-ribbon-f10/s2",
    ],
    "In WordPad's ribbon JAWS gives each tab's and command's place a key behind, the place of      the one the focus left: Right from Home says \"View Tab selected 1 of 2\", Left back      \"Home 2 of 2\", Right from Paste \"Copy 1 of 3\", in every trial. The ribbon's MSAA      objects put View second of the two tabs shown and Copy third of its group's three. Okur      says those places.",
    "the agent under the owner's rule on JAWS's accidents, 2026-10-23";

    Difference::Step => [
        "1011-desktop-screen-text-ocr-window/s4",
        "1015-desktop-screen-text-ocr-window-english/s4",
    ],
    "After Convenient OCR's W, JAWS's Alt+Delete says \"JAWSCursor\", its name and the cursor's \
     run together and without \"active\" (\"JAWSCursor | 590, 722 | Unrestricted\"), in every \
     trial, where its JAWS cursor's own is \"JAWS Cursor active\" (994/s4): an internal name \
     said for the words. Okur says \"Okur Cursor active | 590, 722 | Unrestricted\".",
    "the agent under the owner's rule on JAWS's accidents, 2026-10-24";

    Difference::Step => [
        "1022-desktop-wpf-scan-cursor-controls-alone/s18",
        "1022-desktop-wpf-scan-cursor-controls-alone/s19",
        "1022-desktop-wpf-scan-cursor-controls-alone/s20",
        "1022-desktop-wpf-scan-cursor-controls-alone/s21",
        "1022-desktop-wpf-scan-cursor-controls-alone/s24",
        "1022-desktop-wpf-scan-cursor-controls-alone/s25",
        "1022-desktop-wpf-scan-cursor-controls-alone/s26",
        "1022-desktop-wpf-scan-cursor-controls-alone/s27",
        "1022-desktop-wpf-scan-cursor-controls-alone/s29",
        "1022-desktop-wpf-scan-cursor-controls-alone/s30",
        "1022-desktop-wpf-scan-cursor-controls-alone/s32",
        "1023-desktop-wpf-scan-cursor-end-alone/s10",
        "1023-desktop-wpf-scan-cursor-end-alone/s11",
        "1023-desktop-wpf-scan-cursor-end-alone/s12",
        "1023-desktop-wpf-scan-cursor-end-alone/s13",
        "1023-desktop-wpf-scan-cursor-end-alone/s14",
        "1023-desktop-wpf-scan-cursor-end-alone/s15",
    ],
    "JAWS's Scan Cursor goes past a WPF window's last element onto the window itself and is      stuck there, in every trial: Down from the last element says it again and the window's      title (\"Focus here Edit | SR WPF Controls Fixture\"), then Down, Up, Home and End say      nothing and Right says the title's first letter again and again; Ctrl+End lands there      too, its line the title. Only Ctrl+Home brings it back. Okur's Scan Cursor stays on the      last element, Down saying it again, and Ctrl+End goes to it.",
    "the agent under the owner's rule on JAWS's accidents, 2026-10-24";

    Difference::Step => [
        "1024-desktop-ocr-file-picture/s3",
        "1025-desktop-ocr-file-pdf/s3",
        "1028-desktop-ocr-all-in-one-file/s3",
    ],
    "As F or Enter opens its Results Viewer, JAWS reads the text's first line three times, \
     then all of it with its \"Open in Word...\" link, then often all of it again, and for the \
     picture says its engine's \"Page is right side up.\" before \"finished.\", in every trial. \
     Okur says \"Document OCR started.\", \"finished.\" and the text once. Windows' recognizer \
     reports no page orientation; the Word link is a known gap of its own.",
    "the agent under the owner's rule on JAWS's accidents, 2026-10-24";

    Difference::Step => [
        "1025-desktop-ocr-file-pdf/s4",
        "1025-desktop-ocr-file-pdf/s5",
    ],
    "JAWS's Results Viewer puts each page of the scanned PDF on one line, its two lines \
     joined (\"A letter drawn as pictures Here is the first page\"), in every trial, where it \
     keeps the picture's two lines apart (1024/s6): its engine's (OmniPage's) paragraphs. \
     Windows' recognizer gives lines, and Okur shows them as it reads them.",
    "the agent under the owner's rule on JAWS's accidents, 2026-10-24";

    Difference::JawsSaysBeforeItsDialog => [
        "1026-desktop-ocr-file-none-selected/s3",
        "1027-desktop-ocr-file-not-a-picture/s3",
        "1033-desktop-ocr-file-two-selected/s3",
    ],
    "Before its message box (\"OCR Failed to start. No file currently selected.\"), JAWS \
     says its own main window, \"JAWS Professional | JAWS Version 2025.2505.43\" and the folder, \
     or the tutor message of a button, in most trials, and nothing in others. Okur's message box \
     opens without them.",
    "the agent under the owner's rule on JAWS's accidents, 2026-10-24";

    Difference::JawsAlsoSays("Or, when not in Windows Explorer, you may use the context menu while focus is on a file and choose Recognize With JAWS.") => [
        "1030-desktop-ocr-file-outside-explorer/s3",
    ],
    "F outside File Explorer: JAWS's third sentence sends the user to \"Recognize With JAWS\" \
     in File Explorer's context menu, an item JAWS's installer adds to Windows. Okur adds none, \
     and says the first two.",
    "the agent under the owner's rule on JAWS's accidents, 2026-10-24";

    Difference::Step => [
        "1024-desktop-ocr-file-picture/s11",
        "1025-desktop-ocr-file-pdf/s14",
        "1028-desktop-ocr-all-in-one-file/s6",
    ],
    "Escape out of the Results Viewer: JAWS says the selected file and its place first \
     (\"SR OCR Sign.png | 3 of 3\"), then the window with the file again, in every trial: the \
     list's focus arrives before the window's. Okur says the window with the file once.",
    "the agent under the owner's rule on JAWS's accidents, 2026-10-24";

    Difference::Step => [
        "1031-desktop-ocr-document-not-open/s3",
    ],
    "D where no PDF is open in Adobe Acrobat: JAWS says \"OCR document is only available \
     inside of Adobe Acrobat.\" and how to open one there. Okur recognizes no document open in \
     a reader yet, so those sentences would be false of it: after \"OCR Failed to start.\" it \
     points to F, which recognizes a PDF selected in File Explorer.",
    "the agent, 2026-10-24";

    Difference::Step => [
        "1032-desktop-ocr-layer-help/s5",
        "1032-desktop-ocr-layer-help/s6",
        "1032-desktop-ocr-layer-help/s7",
        "1032-desktop-ocr-layer-help/s8",
        "1032-desktop-ocr-layer-help/s9",
        "1032-desktop-ocr-layer-help/s10",
        "1032-desktop-ocr-layer-help/s11",
    ],
    "The OCR layer's help lists Okur's keys: JAWS's lines for A (a scanner or camera), D \
     (a document open in Acrobat), R (a file into Word) and Ctrl+B's sound are left out, as \
     Okur has none of them, and the lines after them come earlier. The first line and the last \
     (Q) are JAWS's.",
    "the agent, 2026-10-24";

    Difference::Step => ["688-web-custom-label/s15"],
    "After a custom label is given, JAWS's virtual cursor is at the page's top: Say Line says \
     the page's title, in every trial, as JAWS builds the page again with the label. The user \
     labelled the field to go on working there. Okur builds the page again with the cursor \
     still on the field, and Say Line reads it with its label (\"lamp Edit\").",
    "the agent under the owner's rule on JAWS's accidents, 2026-10-21";

    Difference::Step => [
        "704-web-placemarker-two/s6",
        "704-web-placemarker-two/s7",
        "704-web-placemarker-two/s8",
        "704-web-placemarker-two/s9",
        "704-web-placemarker-two/s10",
        "704-web-placemarker-two/s14",
        "704-web-placemarker-two/s15",
        "704-web-placemarker-two/s16",
        "704-web-placemarker-two/s17",
        "704-web-placemarker-two/s18",
        "704-web-placemarker-two/s20",
    ],
    "After the first PlaceMarker is added, JAWS reads the page again and its cursor is left at      the page's end, though Ctrl+Home was pressed: H then went \"wrapping to bottom\" to the      last heading and up from there, so its second PlaceMarker went on the line after the first      heading where the keys meant the third, in every trial; K, Shift+K and 8 then find it      there. Okur moves as the keys say, its second PlaceMarker after the third heading.",
    "the agent under the owner's rule on JAWS's accidents, 2026-10-21";

    Difference::Step => [
        "702-web-placemarker-saved/s8",
        "702-web-placemarker-saved/s9",
        "702-web-placemarker-saved/s10",
    ],
    "After a PlaceMarker is added, JAWS builds the page again and reads it from its top as it      does on loading: Ctrl+Home gets \"selected\" and that reading, K then says \"wrapping to      bottom\" before the PlaceMarker after the reading moved the cursor down, and Insert+Up      says \"Not in a Table\", in every trial. Okur keeps the page and its cursor: Ctrl+Home      says the page's title, K the PlaceMarker's line, Insert+Up that line.",
    "the agent under the owner's rule on JAWS's accidents, 2026-10-21";

    Difference::Step => [
        "716-web-flexible-web-hide-and-undo/s17",
        "716-web-flexible-web-hide-and-undo/s18",
        "716-web-flexible-web-hide-and-undo/s19",
        "716-web-flexible-web-hide-and-undo/s20",
        "716-web-flexible-web-hide-and-undo/s21",
        "716-web-flexible-web-hide-and-undo/s22",
        "725-web-flexible-web-hidden-region-read/s14",
        "725-web-flexible-web-hidden-region-read/s15",
        "725-web-flexible-web-hidden-region-read/s27",
        "725-web-flexible-web-hidden-region-read/s28",
        "725-web-flexible-web-hidden-region-read/s29",
        "725-web-flexible-web-hidden-region-read/s30",
        "735-web-flexible-web-hidden-after-refresh/s15",
        "735-web-flexible-web-hidden-after-refresh/s16",
    ],
    "After Flexible Web hides a region, JAWS still reads it, in every trial: R finds the      hidden advertisement and Down reads its lines, though the wizard's first page then      offers its Undo, and after JAWS's refresh (Insert+Escape) too. JAWS's guide says a customization \"is immediately applied to the      page\". Okur takes the region out of the page it reads, and Undo puts it back.",
    "the agent under the owner's rule on JAWS's accidents, 2026-10-21";

    Difference::Step => [
        "743-web-flexible-web-saved-rule-for-page/s24",
        "743-web-flexible-web-saved-rule-for-page/s28",
        "743-web-flexible-web-saved-rule-for-page/s29",
        "743-web-flexible-web-saved-rule-for-page/s46",
        "743-web-flexible-web-saved-rule-for-page/s50",
        "743-web-flexible-web-saved-rule-for-page/s51",
    ],
    "A Flexible Web rule saved for the page does not hide the advertisement when the page      loads again: JAWS reads it on loading and R finds it, as after a temporary      customization (725). Okur applies the rule as the page loads, as the guide describes      rules: the page has one region and one link less, and R goes from the navigation region      to the main region.",
    "the agent under the owner's rule on JAWS's accidents, 2026-10-21";

    Difference::Step => [
        "742-web-flexible-web-save-page-and-reload-question/s30",
        "746-web-flexible-web-rule-saved-from-question/s11",
    ],
    "When a page loads while the one before has temporary Flexible Web customizations, JAWS      asks whether to save them, and says only \"Flexible Web dialog | Yes Button\": not the      question, though it reads the question of its other message boxes (\"Are you sure you      want to remove all PlaceMarkers?\", 702/s27). Okur says the page's title, then the      dialog with its question.",
    "the agent under the owner's rule on JAWS's accidents, 2026-10-21";

    Difference::Step => [
        "742-web-flexible-web-save-page-and-reload-question/s39",
        "742-web-flexible-web-save-page-and-reload-question/s41",
        "742-web-flexible-web-save-page-and-reload-question/s42",
        "742-web-flexible-web-save-page-and-reload-question/s43",
        "742-web-flexible-web-save-page-and-reload-question/s44",
        "742-web-flexible-web-save-page-and-reload-question/s45",
        "742-web-flexible-web-save-page-and-reload-question/s46",
    ],
    "After Escape on Flexible Web's save page that the question as a page loads opened, the      focus is left in JAWS's own hidden main window: F5 reloads nothing and Ctrl+Home and R      say nothing. Okur's wizard gives the focus back to the page, which F5 reloads and reads.",
    "the agent under the owner's rule on JAWS's accidents, 2026-10-21";

    Difference::Step => ["711-desktop-typing-echo-jcf-3/s7"],
    "With typing echo set to both characters and words, a word corrected with Backspace is not      said when Enter ends it: \"c d\", Backspace, Enter says only \"Enter\", in every trial,      while the same keys with words alone say \"c\" then \"Enter\" (710/s7), and both says      the word an uncorrected Enter ends (403/s27). Okur says the word as it stands in both      modes.",
    "the agent under the owner's rule on JAWS's accidents, 2026-10-21";

    Difference::Step => [
        "701-web-placemarker-add-dialog/s15",
        "703-web-placemarker-none/s6",
        "704-web-placemarker-two/s37",
    ],
    "After the PlaceMarker List closed (Escape after K opened it, MoveTo), the focus back in      the page, Say Line said nothing, in every trial of these scenarios. The virtual cursor is      on its line, which Okur reads.",
    "the agent under the owner's rule on JAWS's accidents, 2026-10-21";

    Difference::Step => [
        "763-web-skim-reading-rule-types/s23",
        "763-web-skim-reading-rule-types/s24",
        "776-web-skim-reading-text-rules-controls/s19",
        "776-web-skim-reading-text-rules-controls/s20",
        "777-web-skim-reading-speak-unit-and-attributes/s28",
        "777-web-skim-reading-speak-unit-and-attributes/s29",
    ],
    "Closing Skim Reading by Escape (763, 777) or by Close's Alt+C (776) closes the dialog and      opens JAWS's summary in its Results Viewer, which then says nothing to Say Line: in      every trial of four recordings, after changes and right after it opened (781, 782);      only 760's Escape, after Tab through the dialog, closed it alone. Okur's Escape and      Close close the dialog and give the page back.",
    "the agent under the owner's rule on JAWS's accidents, 2026-10-21";

    Difference::Step => [
        "808-web-word-list-summary-by-alt-s/s3",
        "808-web-word-list-summary-by-alt-s/s4",
        "808-web-word-list-summary-by-alt-s/s5",
        "808-web-word-list-summary-by-alt-s/s6",
        "808-web-word-list-summary-by-alt-s/s7",
        "808-web-word-list-summary-by-alt-s/s8",
    ],
    "The Word List's Summary, by Alt+S or by Space on it (807), closed the dialog and said      the page, and opened nothing, in every trial: Tab, Enter and Insert+Up after it met the      page. JAWS's help says it opens the virtual viewer with a link for each sentence holding      the word, which Okur does.",
    "the agent under the owner's rule on JAWS's accidents, 2026-10-22";

    Difference::Step => [
        "811-web-skim-read-by-superscript/s15",
        "812-web-skim-read-by-subscript/s16",
        "815-web-skim-read-by-superscript-positions/s15",
        "816-web-skim-read-by-subscript-positions/s16",
        "818-web-number-positions-read-down/s2",
        "818-web-number-positions-read-down/s7",
        "824-web-number-positions-by-word/s2",
        "824-web-number-positions-by-word/s5",
        "824-web-number-positions-by-word/s6",
    ],
    "JAWS passes over raised and lowered text of one character: skim reading's Superscript \
     and Subscript found 1st and 2nd, 10 to the 23rd, log10 and aij, but not E = mc2, x to the \
     n, a footnote's a, H2O, CO2 or x sub i, though Chromium marks them all (text-position); \
     and a line reads 2 cubed as \"23\" and binary 101 as \"1012\", where it marks 10 to the \
     23rd (\"10 superscript 23 end superscript\"). Okur finds every raised and lowered text, \
     as JAWS's help describes the rules, and marks a one-digit number after a number too.",
    "the agent under the owner's rule on JAWS's accidents, 2026-10-22";

    Difference::Step => ["520-desktop-say-clipboard/s3"],
    "The recording's Windows+Insert+X never reached JAWS: Windows takes an injected \
     Windows-key chord and opened its own Win+X menu, which JAWS read. Pressed by a hand, the \
     key is JAWS's SayClipboard, which reads the clipboard (825, by script name); Okur reads \
     it, then the menu the replay's trace holds.",
    "the agent, 2026-10-22: the evidence records the harness's injected chord, not JAWS's key";

    Difference::Step => ["790-desktop-drag-and-drop-in-notepad/s6"],
    "Drag and drop's second Ctrl+Insert+Numpad Slash in Notepad, after the mouse moved three      steps along the text, said in every trial that the object to be dragged was not found      and canceled. Okur drags from where the first press was to where the mouse is, and says      nothing.",
    "the agent under the owner's rule on JAWS's accidents, 2026-10-22";

    Difference::Step => [
        "769-web-skim-read-by-sentence/s4",
        "769-web-skim-read-by-sentence/s6",
        "769-web-skim-read-by-sentence/s7",
        "771-web-skim-reading-rule-saved/s4",
    ],
    "In JAWS's Results Viewer, which skim reading started from its dialog opens at its end,      the recordings got nothing for Say Line, the arrows or Escape, by our keys or JAWS's      TypeKey (762, 767, 768, 769, 771), while Tab got its links (780). Okur's viewer reads      its lines as the Virtual Viewer does, and Escape closes it and reads the page's line.",
    "the agent under the owner's rule on JAWS's accidents, 2026-10-22";

    Difference::JawsSaysBeforeItsDialog => [
        "1036-desktop-word-heading-list-move/s5",
        "1049-desktop-word-links-list-items/s1",
        "1050-desktop-word-fields-list-items/s1",
        "1053-desktop-word-fields-list-controls/s1",
        "1054-desktop-word-fields-list-move/s1",
        "519-desktop-word-lists-and-spelling/s2",
        "519-desktop-word-lists-and-spelling/s5",
    ],
    "Before its Heading List, Links List, List Fields and Spelling Errors lists in Word, JAWS \
     says what it said or showed before them in most trials, in every one before its Links List \
     and List Fields: its main \
     window's version, \"JAWS Version \
     2025.2505.43\", the document's name, \"Microsoft Word Document\", a button's hint, or \
     \"Please wait processing 1 items\" while it gathers the errors. Okur's lists open without \
     them, as JAWS's do in the other trials.",
    "the agent under the owner's rule on JAWS's accidents, 2026-10-24";

    Difference::Step => [
        "519-desktop-word-lists-and-spelling/s6",
        "519-desktop-word-lists-and-spelling/s7",
    ],
    "In JAWS's Spelling Errors list, Say Line and Insert+Shift+R got nothing in every trial: \
     the recorder's keys do not reach JAWS's own dialogs, which run elevated (the Results \
     Viewer's entry above). Okur's list reads its selected error, \"teh 1 of 1\", and \
     Insert+Shift+R says there are no regions, as in any window.",
    "the agent under the owner's rule on JAWS's accidents, 2026-10-24";

    Difference::Step => [
        "1034-desktop-word-font-paragraphs-table-keys/s1",
        "1034-desktop-word-font-paragraphs-table-keys/s3",
        "1040-desktop-word-formats/s1",
        "1040-desktop-word-formats/s3",
        "1040-desktop-word-formats/s5",
        "1040-desktop-word-formats/s7",
        "1052-desktop-word-say-font-line-spacing/s3",
        "1052-desktop-word-say-font-line-spacing/s11",
    ],
    "Say Font in Word: a paragraph's line spacing as a multiple of lines. Word keeps it in \
     points, 12 a line (1.08 lines is Word's default for Normal). JAWS divides by the font's \
     size instead and drops the fraction: \"1 lines.\" for 1.08 and 1.15, \"2 lines.\" for \
     2.5, \"0 lines.\" for 1.08 at 16 points. Okur says the true number, \"1.08 lines.\", \
     \"2.5 lines.\", in JAWS's words around it.",
    "the agent under the owner's rule on JAWS's accidents, 2026-10-25";

    Difference::Step => [
        "1057-desktop-word-protected-form-change/s3",
    ],
    "Space on a Word form's check box just checked: JAWS says nothing, in every trial, where \
     the same key on the next check box says its new state (1057/s5, \"not checked\"). Okur \
     says the state each time: \"not checked\".",
    "the agent under the owner's rule on JAWS's accidents, 2026-10-28";

    Difference::Step => [
        "1057-desktop-word-protected-form-change/s9",
        "1057-desktop-word-protected-form-change/s11",
    ],
    "A Word form field said again: after Enter in a drop-down's list JAWS says the field with \
     its new choice and its hint, then the field once more, or passes through its own menu bar \
     first (\"Context Menu | leaving menu bar\"); after Ctrl+Home onto a text field it says \
     \"Name: Edit | top of file\" three times over in every trial (one trial of round-1055g said \
     it once). Okur says each once: \"Leaving menus | Branch: Combo box | Northside | 2 of 3 | To \
     change the selection use the Arrow keys.\", \"Name: Edit\".",
    "the agent under the owner's rule on JAWS's accidents, 2026-10-28";

    Difference::Step => [
        "1060-desktop-word-paragraph-dialog/s4",
    ],
    "Word's Paragraph dialog opened by the Home tab's key tips (Alt, H, P, G): JAWS says \
     \"Leaving menus | Leaving ribbons\" and nothing of the dialog or its first field, in every \
     trial; the next Tab reads the next field. Okur says the dialog's first field, \
     \"Alignment: Combo box | Left | To change the selection use the Arrow keys.\"",
    "the agent under the owner's rule on JAWS's accidents, 2026-10-29";

    Difference::Step => [
        "1058-desktop-word-font-dialog/s1",
    ],
    "Opening Word's Font dialog (Ctrl+D), JAWS's words vary by trial: the field's value again \
     after its hint in one, the page tab's role in place of the dialog's name in two. Okur says \
     the dialog's name, the field and its hint once: \"Font dialog | Font: | edit combo | \
     Calibri | To set the value use the Arrow keys or type the value.\"",
    "the agent under the owner's rule on JAWS's accidents, 2026-10-28";
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

/// Compares Okur's speech for `step` with JAWS's transcripts, both [`comparable`], with every
/// accepted divergence of the step applied.
pub fn compare(step: &str, okur: &str, jaws: &[String]) -> Comparison {
    let differences: Vec<Difference> = for_step(step).map(|d| d.difference).collect();
    if differences.contains(&Difference::Step) {
        return Comparison::Accepted;
    }
    // What Okur says before its own dialog's line ("Virtual Find").
    let okur_before = dialog_line(okur)
        .and_then(|line| okur.get(..line))
        .unwrap_or_default();
    let without = |text: &str, okur_side: bool, ours: &str| {
        let removed = differences
            .iter()
            .fold(text.to_owned(), |text, difference| {
                match (difference, okur_side) {
                    (Difference::JawsAlsoSays(phrase), false)
                    | (Difference::OkurAlsoSays(phrase), true) => text.replace(phrase, " "),
                    (Difference::JawsSaysBeforeItsDialog, false) => {
                        from_its_dialog_after(&text, okur_before)
                            .or_else(|| spoken_from_its_dialog(&text, ours))
                            .unwrap_or(text)
                    }
                    _ => text,
                }
            });
        comparable(&removed)
    };
    let ours = without(okur, true, "");
    if jaws.iter().any(|v| without(v, false, &ours) == ours) {
        Comparison::Matched
    } else {
        Comparison::Differs
    }
}

/// Every divergence still describes the corpus: its steps exist; a phrase JAWS also says is in
/// JAWS's transcripts and one Okur also says is in Okur's speech, on some layer; and a whole
/// step does not match JAWS exactly on every layer (a typing echo differs only in what is
/// spoken, the speech history holding none). `steps` are the replayed steps, one entry per
/// layer compared: (key, Okur's speech, JAWS's transcripts).
pub fn check(steps: &[(&str, &str, &[String])]) -> Result<(), String> {
    let mut stale = Vec::new();
    for divergence in DIVERGENCES {
        for &key in divergence.steps {
            let layers: Vec<(&str, &[String])> = steps
                .iter()
                .filter(|(k, _, _)| *k == key)
                .map(|&(_, okur, jaws)| (okur, jaws))
                .collect();
            if layers.is_empty() {
                stale.push(format!("{key}: no such step"));
                continue;
            }
            let any = |test: &dyn Fn(&str, &[String]) -> bool| {
                layers.iter().any(|&(okur, jaws)| test(okur, jaws))
            };
            match divergence.difference {
                Difference::JawsAlsoSays(phrase)
                    if !any(&|_, jaws| jaws.iter().any(|v| v.contains(phrase))) =>
                {
                    stale.push(format!("{key}: JAWS no longer says {phrase:?}"));
                }
                Difference::OkurAlsoSays(phrase) if !any(&|okur, _| okur.contains(phrase)) => {
                    stale.push(format!("{key}: Okur no longer says {phrase:?}"));
                }
                // On the spoken layer the dialog line has no role marks: "Overwrite Clipboard
                // dialog" (826/s8, recorded without JAWS's history).
                Difference::JawsSaysBeforeItsDialog
                    if !any(&|okur, jaws| {
                        jaws.iter().any(|v| {
                            from_its_dialog(v).is_some()
                                || spoken_from_its_dialog(v, &comparable(okur)).is_some()
                        })
                    }) =>
                {
                    stale.push(format!("{key}: JAWS says nothing before its dialog"));
                }
                Difference::Step
                    if !any(&|okur, jaws| {
                        !jaws.iter().any(|v| comparable(v) == comparable(okur))
                    }) =>
                {
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

    /// What Okur says before its own dialog, and JAWS first too, stays compared: "Virtual
    /// Find", then JAWS's Find what field said before JAWS Find [obs, 506/s1].
    #[test]
    fn speech_before_the_dialog_is_left_out_after_what_both_say_first() {
        let jaws = "Virtual Find\nFind what: \u{1d} edit combo \u{1e}\nboard\n\
                    JAWS Find \u{1d} dialog \u{1e}\nFind what:";
        let dialog = "JAWS Find \u{1d} dialog \u{1e}\nFind what:";
        assert_eq!(
            from_its_dialog_after(jaws, "Virtual Find\n"),
            Some(format!("Virtual Find\n{dialog}"))
        );
        assert_eq!(from_its_dialog_after(jaws, ""), Some(dialog.to_owned()));
        // What JAWS did not say first is not put back.
        assert_eq!(
            from_its_dialog_after(jaws, "Links\n"),
            Some(dialog.to_owned())
        );
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

    /// Where JAWS names itself, Okur names itself (AGENTS.md §1.5): the names compare as equal,
    /// on every step, and nothing else does [obs, 516/s1, s3].
    #[test]
    fn okurs_name_compares_with_jaws_own() {
        let step = "516-desktop-jaws-cursor/s1";
        assert_eq!(
            compare(step, "Okur Cursor", &jaws(&["JAWS Cursor"])),
            Comparison::Matched
        );
        assert_eq!(
            compare(step, "Route Okur to PC", &jaws(&["Route JAWS to PC"])),
            Comparison::Matched
        );
        assert_eq!(
            compare(step, "PC Cursor", &jaws(&["JAWS Cursor"])),
            Comparison::Differs
        );
        // A text's own "JAWS", read as written by both [obs, 405/s23].
        assert_eq!(
            compare(step, "JAWS sister-in-law", &jaws(&["JAWS sister-in-law"])),
            Comparison::Matched
        );
        // Only the whole word, in JAWS's capitals.
        assert_eq!(
            crate::parity::in_okurs_name("JAWS's JAWSKey+F4 jaws (JAWS) JAWS"),
            "Okur's JAWSKey+F4 jaws (Okur) Okur"
        );
        assert_eq!(
            compare(step, "Okur Key", &jaws(&["JAWSKey"])),
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

    /// A whole step that matches JAWS on the history layer still differs while its spoken layer
    /// does: a typing echo is spoken, never written to the history.
    #[test]
    fn a_step_that_differs_on_one_layer_is_not_stale() {
        let blank = vec![String::new()];
        let enter = vec!["Enter".to_owned()];
        let key = "711-desktop-typing-echo-jcf-3/s7";
        let differs: Vec<(&str, &str, &[String])> =
            vec![(key, "", &blank), (key, "c Enter", &enter)];
        let result = check(&differs);
        assert!(
            !result.as_ref().is_err_and(|e| e.contains(key)),
            "{result:?}"
        );
        let matches: Vec<(&str, &str, &[String])> = vec![(key, "", &blank), (key, "Enter", &enter)];
        let result = check(&matches);
        assert!(
            result
                .as_ref()
                .is_err_and(|e| e.contains(&format!("{key}: Okur now matches JAWS"))),
            "{result:?}"
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
