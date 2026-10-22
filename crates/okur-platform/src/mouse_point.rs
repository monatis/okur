//! What the mouse points to, for mouse echo: the element under a point of the screen, with its
//! ancestors, and its text there by unit when it gives its text by range (UI Automation's
//! `RangeFromPoint`, expanded to a character, a word, a line, a paragraph). The tracer records
//! it where the mouse comes to rest, and the live worker reads it when the engine asks, so
//! replay and live use the same data.

#[cfg(windows)]
pub use reader::read;

#[cfg(windows)]
mod reader {
    use crate::uia_snapshot::UiaNode;
    use okur_core::UnitTexts;
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::Accessibility::{
        IUIAutomation, IUIAutomationTextPattern, TextUnit, TextUnit_Character, TextUnit_Line,
        TextUnit_Paragraph, TextUnit_Word, UIA_TextPatternId,
    };

    /// Characters read of one unit at most (a paragraph of a long document).
    const MAX_UNIT_CHARS: i32 = 4000;

    /// The element at `x`, `y` in screen pixels with its control-view ancestors, outermost
    /// first, and its text there by unit; `None` when nothing is there.
    pub fn read(uia: &IUIAutomation, x: i32, y: i32) -> Option<(Vec<UiaNode>, UnitTexts)> {
        let point = POINT { x, y };
        // SAFETY: UIA client calls on valid objects.
        unsafe {
            let element = uia.ElementFromPoint(point).ok()?;
            let element =
                crate::uia_snapshot::capture::with_properties(uia, &element).unwrap_or(element);
            let chain = crate::uia_snapshot::capture::focus_chain_from(uia, &element);
            let texts = element
                .GetCurrentPatternAs::<IUIAutomationTextPattern>(UIA_TextPatternId)
                .ok()
                .and_then(|pattern| pattern.RangeFromPoint(point).ok())
                .map(|range| {
                    let unit = |unit: TextUnit| {
                        let range = range.Clone().ok()?;
                        range.ExpandToEnclosingUnit(unit).ok()?;
                        let text = range.GetText(MAX_UNIT_CHARS).ok()?.to_string();
                        Some(text)
                    };
                    UnitTexts {
                        character: unit(TextUnit_Character),
                        word: unit(TextUnit_Word),
                        line: unit(TextUnit_Line),
                        paragraph: unit(TextUnit_Paragraph),
                    }
                })
                .unwrap_or_default();
            Some((chain, texts))
        }
    }
}
