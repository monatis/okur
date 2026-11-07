//! Microsoft Excel, as JAWS's script for it reads a workbook: a cell by its value, then its
//! coordinates, "12 | B 2" [obs, 521/s3], and Insert+C the coordinates alone, "B 2" [obs,
//! 521/s6]. A cell is UI Automation's table cell in Excel's grid window (`EXCEL7`), named by
//! its reference ("B2"). What JAWS says of a sheet's place, its readers, selections, titles and
//! totals is [`crate::spreadsheet`]'s.

use crate::aom::Role;
use crate::apps::{AppContext, AppModule, FocusResponse};
use crate::event::FocusEvent;
use crate::input::Key;
use crate::messages::msg;
use crate::spreadsheet::{
    coordinates, named_titles, reference_end, titles_of, SelectionKey, SheetTitles,
};

/// Excel's grid window, which holds a sheet's cells.
const GRID_WINDOW: &str = "EXCEL7";
/// Excel's cell editor, F2's (and typing's) edit.
const EDITOR_WINDOW: &str = "EXCEL6";
/// Excel's main window, which reports the focus back on the cell as a dialog of Excel's closes
/// [obs, 1124/s7, s9].
const MAIN_WINDOW: &str = "XLMAIN";
/// What Excel's UI Automation value of a cell with a hyperlink ends with, after its text and a
/// comma: "Docs , Contains Hyperlink" [obs, round 1191's trace].
const LINK_SUFFIX: &str = "Contains Hyperlink";

/// A cell's value without Excel's hyperlink suffix, and whether it had one.
fn link_value(value: &str) -> (&str, bool) {
    match value.trim().strip_suffix(LINK_SUFFIX) {
        Some(text) => (text.trim_end_matches([' ', ',']).trim(), true),
        None => (value.trim(), false),
    }
}

/// Excel's module.
#[derive(Debug, Default)]
pub struct Excel {
    /// The last cell reached was empty, which a Ctrl arrow leaves for a new data region.
    from_blank: bool,
    /// The last cell said, (column, row): the next one says the title of a column or a row it
    /// enters.
    last_cell: Option<(u32, u32)>,
    /// A cell said once its titles come with the sheet's place.
    pending: Option<PendingTitles>,
    /// A selecting key's change was said since the last cell: the cell is said again even
    /// where it has not moved [obs, 1186/s17].
    selection_said: bool,
}

/// A cell's speech waiting for the titles of the column or the row it entered: the row that
/// holds the column titles, the column that holds the row titles.
#[derive(Debug)]
struct PendingTitles {
    /// What the key says before the titles: "Last Cell | Weekends | 14 | C 4" [obs, 1166/s18].
    first: Vec<String>,
    lines: Vec<String>,
    /// The cell, and the one before it.
    at: (u32, u32),
    before: (u32, u32),
    /// The workbook's titles where known; else the place's, from the workbook's names.
    titles: Option<SheetTitles>,
}

/// The row of column titles and the column of row titles a move from `before` to `at` says:
/// into another column, the column's title; into another row, the row's; not the title of a
/// title cell itself, "Central | A 2" [obs, 1132/s3-s6, s9-s13].
fn titles_entered(
    titles: SheetTitles,
    (column, row): (u32, u32),
    (was_column, was_row): (u32, u32),
) -> (Option<u32>, Option<u32>) {
    (
        titles.row.filter(|&r| column != was_column && row != r),
        titles.column.filter(|&c| row != was_row && column != c),
    )
}

impl AppModule for Excel {
    fn owns(&self, focus: &FocusEvent) -> bool {
        cell(focus).is_some() || grid(focus) || editor(focus)
    }

    /// The sheet's place for a cell waiting for its titles: the column's title, the row's,
    /// then the cell, "Weekdays | 12 | B 2" [obs, 1132/s4, s5]; the cell alone without the
    /// place.
    fn sheet_place(&mut self, place: Option<&crate::event::SheetPlace>) -> Option<String> {
        let pending = self.pending.take()?;
        let mut lines = pending.first;
        let titles: Vec<String> = place
            .map(|place| {
                let titles = pending.titles.unwrap_or_else(|| named_titles(place));
                let (row, column) = titles_entered(titles, pending.at, pending.before);
                titles_of(place, row, column)
            })
            .unwrap_or_default();
        lines.extend(titles);
        lines.extend(pending.lines);
        Some(lines.join("\n"))
    }

    /// A cell Okur's list closes back onto, "22 | B 4" [obs, 521/s10, s14].
    fn focus_again(&self, focus: &FocusEvent) -> Option<String> {
        let coordinates = cell(focus)?;
        let (value, link) = link_value(focus.value.as_deref().unwrap_or_default());
        let mut lines: Vec<String> = Vec::new();
        if !value.is_empty() {
            lines.push(value.to_owned());
        }
        if link {
            lines.push(msg::excel_has_hyperlink().to_string());
        }
        lines.push(coordinates);
        // And how to follow it, as a list closes back onto it [obs, 1191/s6].
        if link {
            lines.push(msg::hint_activate_enter().to_string());
        }
        Some(lines.join("\n"))
    }

    /// A cell the menus give back, "Leaving menus | 14 | C 4" [obs, 1116/s7].
    fn says_focus_after_menus(&self, focus: &FocusEvent) -> bool {
        cell(focus).is_some()
    }

    /// A cell reached: its value, then its coordinates [obs, 521/s2-s5, s7, s8, s11, s12];
    /// by Ctrl+End and Ctrl+Home, which cell it is first, "Last Cell | 14 | C 4"; by a Ctrl
    /// arrow, that it is a data region's edge after, "Weekends | C 1 | edge of current data
    /// region" [obs, 1115/s1-s6].
    fn focus(&mut self, focus: &FocusEvent, cx: &mut AppContext<'_>) -> FocusResponse {
        // F2 into the cell's editor: JAWS says nothing [obs, 1118/s6].
        if editor(focus) {
            cx.note_window(focus);
            *cx.app_mode = crate::engine::AppMode::NativeControl;
            return FocusResponse::Silent;
        }
        // An empty cell: the grid itself has the focus, and the cell comes from Excel's object
        // model, "blank | D 1 | Active Cell: D1" [obs, 1118/s3, s4].
        if grid(focus) {
            cx.note_window(focus);
            *cx.app_mode = crate::engine::AppMode::NativeControl;
            cx.ask_sheet = true;
            self.from_blank = true;
            self.pending = None;
            return FocusResponse::Silent;
        }
        let Some(coordinates) = cell(focus) else {
            return FocusResponse::Pass;
        };
        // The cell reported again with no key between: Go To's Enter gives it back in four
        // reports, said once, "6 | C 3" [obs, 1124/s3].
        if cx.repeat {
            cx.note_window(focus);
            *cx.app_mode = crate::engine::AppMode::NativeControl;
            return FocusResponse::Silent;
        }
        // Another cell: one still waiting for its titles has gone by, and the place answering
        // for it is not this one's (said after it, it had repeated the cell left).
        self.pending = None;
        // A selecting key leaves the focus on the active cell: the selection is read from
        // Excel's object model and said for what it changed [obs, 1120, 1121].
        if cx.selecting
            || cx
                .key
                .and_then(|(key, m)| SelectionKey::of(key, m))
                .is_some()
        {
            cx.note_window(focus);
            *cx.app_mode = crate::engine::AppMode::NativeControl;
            cx.ask_sheet = true;
            self.selection_said = true;
            return FocusResponse::Silent;
        }
        let after_selection = std::mem::take(&mut self.selection_said);
        cx.note_window(focus);
        // A native control's keys are the application's, as for any cell the engine reads.
        *cx.app_mode = crate::engine::AppMode::NativeControl;
        // A move's titles ([`titles_entered`]): where the workbook's are not known yet, its
        // names may give them [obs, 1134], and the cell waits for the place to tell.
        let at = cell_at(focus);
        let before = std::mem::replace(&mut self.last_cell, at);
        let moved = at.zip(before);
        let waits = moved.is_some_and(|(at, before)| match cx.sheet_titles {
            Some(titles) => {
                let (row, column) = titles_entered(titles, at, before);
                row.is_some() || column.is_some()
            }
            None => true,
        });
        // A cell with a hyperlink: "has hyperlink" after its text, and after its coordinates how
        // to follow it, "Docs | has hyperlink | A 2 | To activate press Enter." [obs, 1191/s2,
        // 1195/s1, s3, s5].
        let (value, link) = link_value(focus.value.as_deref().unwrap_or_default());
        let link_hint = link && cx.hints;
        // The key's own word, before the titles where they are said.
        let mut first = Vec::new();
        let mut lines = Vec::new();
        let ctrl = cx
            .key
            .map(|(_, m)| m.ctrl && !m.shift && !m.alt && !m.insert && !m.win)
            .unwrap_or(false);
        let key = cx.key.map(|(k, _)| k).filter(|_| ctrl);
        first.extend(key_words(cx.key));
        // Ctrl+Home or Ctrl+End on the cell already active (a selection collapsed onto it): the
        // key's word alone, "First cell" [obs, 1187/s2]; after a selection said, the cell too,
        // "First cell | Branch | A 1" [obs, 1186/s17].
        if matches!(key, Some(Key::Home | Key::End))
            && at.is_some()
            && at == before
            && !after_selection
            // An empty cell between (the grid itself had the focus, its cell unknown here):
            // the cell is said, "First cell | Home page | ... | A 1" [obs, 1191/s10].
            && !self.from_blank
        {
            return FocusResponse::Speak(crate::announcement::Announcement::from_text(
                &first.join("\n"),
            ));
        }
        // An empty cell by its coordinates alone [hyp: no empty cell recorded yet].
        if !value.is_empty() {
            lines.push(value.to_owned());
        }
        if link {
            lines.push(msg::excel_has_hyperlink().to_string());
        }
        lines.push(coordinates);
        // From an empty cell a new data region starts, "8 | C 2 | start of new Data Region"
        // [obs, 1118/s5].
        if matches!(
            key,
            Some(Key::LeftArrow | Key::RightArrow | Key::UpArrow | Key::DownArrow)
        ) {
            lines.push(
                if self.from_blank {
                    msg::excel_data_region_start()
                } else {
                    msg::excel_data_region_edge()
                }
                .to_string(),
            );
        }
        if link_hint {
            lines.push(msg::hint_activate_enter().to_string());
        }
        self.from_blank = value.is_empty();
        if let Some((at, before)) = moved.filter(|_| waits) {
            self.pending = Some(PendingTitles {
                first,
                lines,
                at,
                before,
                titles: cx.sheet_titles,
            });
            cx.ask_sheet = true;
            return FocusResponse::Silent;
        }
        first.extend(lines);
        FocusResponse::Speak(crate::announcement::Announcement::from_text(
            &first.join("\n"),
        ))
    }
}

/// What a key says before the cell it reaches: Ctrl+End's "Last Cell" and Ctrl+Home's "First
/// cell" [obs, 1115/s1-s6], Ctrl+Z's "Undo", before the cell it gives back [obs, 1123/s8],
/// Alt+PageDown's "Moved right one screen" and Alt+PageUp's "Moved left one screen" [obs,
/// 1185/s14, s15].
fn key_words(key: Option<(Key, crate::input::Modifiers)>) -> Option<String> {
    let (key, m) = key?;
    if m.shift || m.insert || m.win || m.ctrl == m.alt {
        return None;
    }
    Some(match (key, m.ctrl) {
        (Key::End, true) => msg::excel_last_cell().to_string(),
        (Key::Home, true) => msg::excel_first_cell().to_string(),
        (Key::Alpha('Z'), true) => msg::edit_undo().to_string(),
        (Key::PageDown, false) => msg::excel_moved_right_screen().to_string(),
        (Key::PageUp, false) => msg::excel_moved_left_screen().to_string(),
        _ => return None,
    })
}

/// The cell `focus` is, as (column, row) from 1.
pub(crate) fn cell_at(focus: &FocusEvent) -> Option<(u32, u32)> {
    cell(focus)?;
    let (column, row) = reference_end(focus.name.as_deref()?.trim())?;
    Some((column?, row?))
}

/// Excel's grid itself focused, as an empty cell leaves it (no cell element to focus).
fn grid(focus: &FocusEvent) -> bool {
    focus.platform.window_class.as_deref() == Some(GRID_WINDOW) && focus.role != Role::TableCell
}

/// Excel's sheet focused: one of its cells, or the grid where the cell is empty.
pub(crate) fn in_sheet(focus: &FocusEvent) -> bool {
    cell(focus).is_some() || grid(focus)
}

/// Excel's cell editor focused (F2).
fn editor(focus: &FocusEvent) -> bool {
    focus.platform.window_class.as_deref() == Some(EDITOR_WINDOW) && focus.role == Role::Edit
}

/// The active cell read from Excel's object model, where the grid had no cell to focus: an
/// empty one as JAWS says it, "blank | D 1 | Active Cell: D1" [obs, 1118/s3, s4], or without
/// naming the active cell where Delete emptied it, "blank | D 1" [obs, 1123/s7]; one with a
/// value as any cell; the words of the `key` that moved there first ([`key_words`]), "Moved
/// right one screen | blank | Z 5 | Active Cell: Z5" [obs, 1185/s14]. `None` without its
/// reference.
pub(crate) fn blank_or_cell(
    place: &crate::event::SheetPlace,
    key: Option<(Key, crate::input::Modifiers)>,
) -> Option<String> {
    let at = coordinates(&place.address)?;
    let names_active_cell = key != Some((Key::Delete, crate::input::Modifiers::none()));
    let value = place
        .row
        .get(place.row_at)
        .map(|v| v.trim())
        .unwrap_or_default();
    let cell = if value.is_empty() && names_active_cell {
        format!(
            "{}\n{at}\n{}",
            msg::blank(),
            msg::excel_active_cell(place.address.trim()).render()
        )
    } else if value.is_empty() {
        format!("{}\n{at}", msg::blank())
    } else {
        format!("{value}\n{at}")
    };
    Some(match key_words(key) {
        Some(words) => format!("{words}\n{cell}"),
        None => cell,
    })
}

/// The coordinates of the cell `focus` is, as JAWS says them, its column's letters apart from
/// its row's number: "B 2"; `None` for any other focus.
pub(crate) fn cell(focus: &FocusEvent) -> Option<String> {
    let window = focus.platform.window_class.as_deref();
    if focus.role != Role::TableCell || !matches!(window, Some(GRID_WINDOW | MAIN_WINDOW)) {
        return None;
    }
    coordinates(focus.name.as_deref()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Modifiers;
    use crate::spreadsheet::{SheetReading, TitleKey};

    /// "12 | B 2" for the cell B2 holding 12 [obs, 521/s3]; nothing of another window's.
    #[test]
    fn a_cell_is_its_value_then_its_coordinates() {
        let mut focus = FocusEvent::new(Role::TableCell);
        focus.name = Some("B2".into());
        focus.value = Some("12".into());
        focus.platform.window_class = Some(GRID_WINDOW.into());
        assert_eq!(cell(&focus).as_deref(), Some("B 2"));
        let mut other = focus.clone();
        other.platform.window_class = Some("Chrome_RenderWidgetHostHWND".into());
        assert_eq!(cell(&other), None);
        // Excel's main window reports the cell as a dialog of Excel's closes [obs, 1124/s7].
        let mut back = focus.clone();
        back.platform.window_class = Some(MAIN_WINDOW.into());
        assert_eq!(cell(&back).as_deref(), Some("B 2"));
    }

    /// What the module says of `focus` after `key`, `repeat` as the engine tells it.
    fn said(
        excel: &mut Excel,
        focus: &FocusEvent,
        key: Option<(Key, crate::input::Modifiers)>,
        repeat: bool,
    ) -> Option<String> {
        let mut mode = crate::engine::AppMode::NativeControl;
        let (mut title, mut kind) = (None, None);
        let mut cx = AppContext {
            app_mode: &mut mode,
            window_title: &mut title,
            window_kind: &mut kind,
            selecting: false,
            alt_held: false,
            letter_key: false,
            escape_key: false,
            key,
            ask_view: false,
            ask_sheet: false,
            repeat,
            office_menu_closed: false,
            sheet_titles: Some(SheetTitles::default()),
            hints: true,
        };
        match excel.focus(focus, &mut cx) {
            FocusResponse::Speak(announcement) => Some(announcement.render()),
            FocusResponse::Silent => Some(String::new()),
            _ => None,
        }
    }

    /// A cell with a hyperlink: "has hyperlink" after its text, the hint after its coordinates,
    /// "Docs | has hyperlink | A 2 | To activate press Enter." [obs, 1191/s2, 1195/s1, s3, s5];
    /// a list closing back onto it, the same [obs, 1191/s6].
    #[test]
    fn a_cell_with_a_hyperlink_says_so() {
        let mut excel = Excel::default();
        assert_eq!(
            said(
                &mut excel,
                &link_cell("A2", "Docs , Contains Hyperlink"),
                down(),
                false
            )
            .as_deref(),
            Some("Docs\nhas hyperlink\nA 2\nTo activate press Enter.")
        );
        assert_eq!(
            excel
                .focus_again(&link_cell("A2", "Docs , Contains Hyperlink"))
                .as_deref(),
            Some("Docs\nhas hyperlink\nA 2\nTo activate press Enter.")
        );
    }

    /// JAWS leaves the hint out on A1, in every trial of two recordings, though it says it on
    /// every other cell with a link [obs, 1191/s3, 1195/s4]; and Excel's UI Automation calls the
    /// plain cell under two links "Plain , Contains Hyperlink", its link with no address, where
    /// JAWS says no "has hyperlink" but still the hint [obs, 1195/s2].
    #[test]
    #[ignore = "known gap: why JAWS leaves A1's hint out; Excel's linkless cell reported linked"]
    fn jaws_leaves_a1_hint_out_and_reads_links_from_the_object_model() {
        let mut excel = Excel::default();
        assert_eq!(
            said(
                &mut excel,
                &link_cell("A1", "Home page , Contains Hyperlink"),
                up(),
                false
            )
            .as_deref(),
            Some("Home page\nhas hyperlink\nA 1")
        );
        assert_eq!(
            said(
                &mut excel,
                &link_cell("A3", "Plain , Contains Hyperlink"),
                down(),
                false
            )
            .as_deref(),
            Some("Plain\nA 3\nTo activate press Enter.")
        );
    }

    /// Down and Up alone.
    fn down() -> Option<(Key, crate::input::Modifiers)> {
        Some((Key::DownArrow, crate::input::Modifiers::none()))
    }

    fn up() -> Option<(Key, crate::input::Modifiers)> {
        Some((Key::UpArrow, crate::input::Modifiers::none()))
    }

    /// A cell of Excel's grid named `name` whose value is `value`.
    fn link_cell(name: &str, value: &str) -> FocusEvent {
        let mut focus = FocusEvent::new(Role::TableCell);
        focus.name = Some(name.into());
        focus.value = Some(value.into());
        focus.platform.window_class = Some(GRID_WINDOW.into());
        focus
    }

    /// Ctrl+Z says "Undo" before the cell it gives back, "Undo | 7 | D 1" [obs, 1123/s8]; the
    /// cell reported again with no key between is said once [obs, 1124/s3].
    #[test]
    fn undo_comes_before_the_cell_and_a_repeat_is_silent() {
        let mut focus = FocusEvent::new(Role::TableCell);
        focus.name = Some("D1".into());
        focus.value = Some("7".into());
        focus.platform.window_class = Some(GRID_WINDOW.into());
        let ctrl = crate::input::Modifiers {
            ctrl: true,
            ..crate::input::Modifiers::none()
        };
        let mut excel = Excel::default();
        let undo = Some((Key::Alpha('Z'), ctrl));
        assert_eq!(
            said(&mut excel, &focus, undo, false).as_deref(),
            Some("Undo\n7\nD 1")
        );
        assert_eq!(said(&mut excel, &focus, undo, true).as_deref(), Some(""));
    }

    /// An empty cell from Excel's object model: "blank | D 1 | Active Cell: D1" reached by a
    /// key that moves [obs, 1118/s3], "blank | D 1" emptied by Delete [obs, 1123/s7].
    #[test]
    fn an_empty_cell_names_the_active_cell_unless_deleted() {
        let place = crate::event::SheetPlace {
            address: "D1".into(),
            sheet: String::new(),
            selection: "D1".into(),
            selection_first: String::new(),
            selection_last: String::new(),
            row_at: 3,
            column_at: 0,
            row: vec![
                "Branch".into(),
                "Weekdays".into(),
                "Weekends".into(),
                String::new(),
            ],
            column: vec![String::new()],
            title_row: None,
            title_column: None,
            column_sum: None,
            row_sum: None,
            format: None,
            cells: Vec::new(),
            details: None,
        };
        assert_eq!(
            blank_or_cell(&place, Some((Key::RightArrow, Modifiers::none()))).as_deref(),
            Some("blank\nD 1\nActive Cell: D1")
        );
        assert_eq!(
            blank_or_cell(&place, Some((Key::Delete, Modifiers::none()))).as_deref(),
            Some("blank\nD 1")
        );
        // A screen's move says so first [obs, 1185/s14, s15].
        let alt = Modifiers {
            alt: true,
            ..Modifiers::none()
        };
        assert_eq!(
            blank_or_cell(&place, Some((Key::PageDown, alt))).as_deref(),
            Some("Moved right one screen\nblank\nD 1\nActive Cell: D1")
        );
        assert_eq!(
            blank_or_cell(&place, Some((Key::PageUp, alt))).as_deref(),
            Some("Moved left one screen\nblank\nD 1\nActive Cell: D1")
        );
        assert_eq!(SheetReading::Cell.said(&place).as_deref(), Some("D 1"));
    }

    /// Row 1 as the column titles and column A as the row titles, from A1: a cell entering a
    /// column says its title first, one entering a row the row's, waiting for the sheet's place;
    /// a title cell says no title of its own [obs, 1132/s1-s10].
    #[test]
    fn titles_come_before_a_cell_entering_their_column_or_row() {
        let mut titles = SheetTitles::default();
        assert_eq!(
            TitleKey::SetColumnTitles
                .set(&mut titles, (1, 1))
                .as_deref(),
            Some("Setting column titles row range to 1")
        );
        assert_eq!(
            TitleKey::SetRowTitles.set(&mut titles, (1, 1)).as_deref(),
            Some("Setting row titles column range to A")
        );
        let cell = |name: &str, value: &str| {
            let mut focus = FocusEvent::new(Role::TableCell);
            focus.name = Some(name.into());
            focus.value = Some(value.into());
            focus.platform.window_class = Some(GRID_WINDOW.into());
            focus
        };
        // What the module says of `focus`, and whether it asked for the place.
        let step = |excel: &mut Excel, focus: &FocusEvent| {
            let mut mode = crate::engine::AppMode::NativeControl;
            let (mut title, mut kind) = (None, None);
            let mut cx = AppContext {
                app_mode: &mut mode,
                window_title: &mut title,
                window_kind: &mut kind,
                selecting: false,
                alt_held: false,
                letter_key: false,
                escape_key: false,
                key: None,
                ask_view: false,
                ask_sheet: false,
                repeat: false,
                office_menu_closed: false,
                sheet_titles: Some(titles),
                hints: true,
            };
            let said = match excel.focus(focus, &mut cx) {
                FocusResponse::Speak(announcement) => announcement.render(),
                _ => String::new(),
            };
            (said, cx.ask_sheet)
        };
        let mut excel = Excel::default();
        assert!(!step(&mut excel, &cell("A1", "Branch")).1);
        // A2 is in the row titles' column: no title of its own.
        assert_eq!(
            step(&mut excel, &cell("A2", "Central")),
            ("Central\nA 2".to_owned(), false)
        );
        // Into column B: its title comes with the place.
        assert_eq!(step(&mut excel, &cell("B2", "12")), (String::new(), true));
        let place = crate::event::SheetPlace {
            address: "B2".into(),
            sheet: String::new(),
            selection: "B2".into(),
            selection_first: String::new(),
            selection_last: String::new(),
            row_at: 1,
            column_at: 1,
            row: vec!["Central".into(), "12".into(), "8".into()],
            column: vec!["Weekdays".into(), "12".into(), "10".into(), "22".into()],
            title_row: None,
            title_column: None,
            column_sum: None,
            row_sum: None,
            format: None,
            cells: Vec::new(),
            details: None,
        };
        assert_eq!(
            excel.sheet_place(Some(&place)).as_deref(),
            Some("Weekdays\n12\nB 2")
        );
        assert_eq!(
            SheetReading::ColumnTitle(1).said(&place).as_deref(),
            Some("Weekdays")
        );
        assert_eq!(
            SheetReading::RowTitle(1).said(&place).as_deref(),
            Some("Central")
        );
        // Without the place, the cell alone.
        assert_eq!(step(&mut excel, &cell("B3", "10")), (String::new(), true));
        assert_eq!(excel.sheet_place(None).as_deref(), Some("10\nB 3"));
    }

    /// Ctrl+End with the column titles set: "Last Cell" before the title, "Last Cell | Weekends |
    /// 14 | C 4" [obs, 1166/s18].
    #[test]
    fn the_last_cell_is_said_before_its_title() {
        let mut titles = SheetTitles::default();
        let _ = TitleKey::SetColumnTitles.set(&mut titles, (1, 1));
        let cell = |name: &str, value: &str| {
            let mut focus = FocusEvent::new(Role::TableCell);
            focus.name = Some(name.into());
            focus.value = Some(value.into());
            focus.platform.window_class = Some(GRID_WINDOW.into());
            focus
        };
        let ctrl = crate::input::Modifiers {
            ctrl: true,
            ..crate::input::Modifiers::none()
        };
        let mut excel = Excel::default();
        let mut step = |focus: &FocusEvent, key: Option<(Key, crate::input::Modifiers)>| {
            let mut mode = crate::engine::AppMode::NativeControl;
            let (mut title, mut kind) = (None, None);
            let mut cx = AppContext {
                app_mode: &mut mode,
                window_title: &mut title,
                window_kind: &mut kind,
                selecting: false,
                alt_held: false,
                letter_key: false,
                escape_key: false,
                key,
                ask_view: false,
                ask_sheet: false,
                repeat: false,
                office_menu_closed: false,
                sheet_titles: Some(titles),
                hints: true,
            };
            let _ = excel.focus(focus, &mut cx);
            cx.ask_sheet
        };
        step(&cell("A1", "Branch"), None);
        assert!(step(&cell("C4", "14"), Some((Key::End, ctrl))));
        let place = crate::event::SheetPlace {
            address: "C4".into(),
            column: vec!["Weekends".into(), "8".into(), "6".into(), "14".into()],
            column_at: 3,
            row: vec!["Total".into(), "22".into(), "14".into()],
            row_at: 2,
            ..crate::event::SheetPlace::default()
        };
        assert_eq!(
            excel.sheet_place(Some(&place)).as_deref(),
            Some("Last Cell\nWeekends\n14\nC 4")
        );
    }

    /// A workbook whose name "Title" sits on A1: its titles, not known before, come with the
    /// first move's place, "Weekdays | 12 | B 2" [obs, 1134/s2].
    #[test]
    fn titles_come_from_the_workbooks_names() {
        let mut focus = FocusEvent::new(Role::TableCell);
        focus.platform.window_class = Some(GRID_WINDOW.into());
        let mut excel = Excel::default();
        let mut step = |name: &str, value: &str| {
            focus.name = Some(name.into());
            focus.value = Some(value.into());
            let mut mode = crate::engine::AppMode::NativeControl;
            let (mut title, mut kind) = (None, None);
            let mut cx = AppContext {
                app_mode: &mut mode,
                window_title: &mut title,
                window_kind: &mut kind,
                selecting: false,
                alt_held: false,
                letter_key: false,
                escape_key: false,
                key: None,
                ask_view: false,
                ask_sheet: false,
                repeat: false,
                office_menu_closed: false,
                sheet_titles: None,
                hints: true,
            };
            let said = match excel.focus(&focus, &mut cx) {
                FocusResponse::Speak(announcement) => announcement.render(),
                _ => String::new(),
            };
            (said, cx.ask_sheet)
        };
        assert_eq!(
            step("A1", "Branch"),
            (
                "Branch
A 1"
                .to_owned(),
                false
            )
        );
        assert_eq!(step("B2", "12"), (String::new(), true));
        let place = crate::event::SheetPlace {
            address: "B2".into(),
            sheet: String::new(),
            row_at: 1,
            column_at: 1,
            row: vec!["Central".into(), "12".into(), "8".into()],
            column: vec!["Weekdays".into(), "12".into(), "10".into(), "22".into()],
            title_row: Some(1),
            title_column: Some(1),
            ..Default::default()
        };
        assert_eq!(
            excel.sheet_place(Some(&place)).as_deref(),
            Some(
                "Weekdays
Central
12
B 2"
            )
        );
    }
}
