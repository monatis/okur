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
    ],
    "JAWS reads nothing of WordPad's document text on old-pc: the arrows, the word keys, Say \
     Line and Say All are silent or say a list item's bullet alone, Say Selected Text says \
     \"Nothing selected\" with text selected, and Ctrl+Home and Ctrl+End say \"top of file\" \
     and \"Bottom of file\" without the line, in every trial, though it reads the font there \
     (s15). Okur reads the text.",
    "the owner's rule on JAWS's accidents, 2026-10-18";

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
                Difference::JawsSaysBeforeItsDialog
                    if !any(&|_, jaws| jaws.iter().any(|v| from_its_dialog(v).is_some())) =>
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
