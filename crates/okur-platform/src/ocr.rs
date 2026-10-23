//! Text recognized from the screen's pixels with Windows' own recognizer (Windows.Media.Ocr),
//! for the Okur cursor's screen text (Insert+Space, O;
//! `docs/plans/2026-10-23-okur-cursor-screen-text.md`). The pixels come from xa11y's GDI
//! capture of the area, in the physical desktop pixels UI Automation and the mouse use, so a
//! recognized word's place is where the Okur cursor moves the mouse to click it.

use okur_core::screen_model::{ScreenRun, Source};

/// A picture smaller than this on a side is enlarged before recognition: the recognizer misses
/// text a few pixels high, on a small control (NVDA enlarges such pictures four times).
const SMALL: u32 = 100;
/// How many times a small picture is enlarged.
const ENLARGE: u32 = 4;

/// A capture's pixels as the recognizer takes them: RGBA as captured to BGRA, opaque (GDI
/// leaves the alpha of screen pixels empty), enlarged `scale` times by repeating each pixel.
fn recognizer_pixels(rgba: &[u8], width: u32, height: u32, scale: u32) -> Vec<u8> {
    let scale = scale.max(1) as usize;
    let (width, height) = (width as usize, height as usize);
    let mut out = Vec::with_capacity(width * height * 4 * scale * scale);
    for row in rgba.chunks_exact(width * 4).take(height) {
        let mut line = Vec::with_capacity(width * 4 * scale);
        for &[r, g, b, _] in row.as_chunks::<4>().0 {
            for _ in 0..scale {
                line.extend_from_slice(&[b, g, r, 255]);
            }
        }
        for _ in 0..scale {
            out.extend_from_slice(&line);
        }
    }
    out
}

/// How many times a picture of `width` by `height` is enlarged: a small one so that it is no
/// longer small, within the recognizer's largest side.
fn enlargement(width: u32, height: u32, largest: u32) -> u32 {
    if width.min(height) >= SMALL {
        return 1;
    }
    let fits = largest / width.max(height).max(1);
    ENLARGE.min(fits).max(1)
}

/// A word the recognizer placed at `(x, y, w, h)` in a picture enlarged `scale` times, taken at
/// `origin` on the desktop: where it is on the desktop.
fn desktop_word(
    text: String,
    (x, y, w, h): (f32, f32, f32, f32),
    scale: u32,
    origin: (i32, i32),
) -> ScreenRun {
    let scale = scale.max(1) as f32;
    let at = |v: f32, base: i32| base + (v / scale).round() as i32;
    ScreenRun::new(
        text,
        (
            at(x, origin.0),
            at(y, origin.1),
            at(x + w, origin.0),
            at(y + h, origin.1),
        ),
        Source::Recognized,
    )
}

#[cfg(windows)]
pub use recognizer::recognize;

#[cfg(windows)]
mod recognizer {
    use super::{desktop_word, enlargement, recognizer_pixels};
    use crate::PlatformError;
    use okur_core::actions::RecognitionArea;
    use okur_core::screen_model::ScreenRun;
    use windows::Globalization::Language;
    use windows::Graphics::Imaging::{BitmapAlphaMode, BitmapPixelFormat, SoftwareBitmap};
    use windows::Media::Ocr::OcrEngine;
    use windows::Storage::Streams::DataWriter;
    use windows::Win32::Foundation::RECT;
    use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS};
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetSystemMetrics, GetWindowRect, SM_CXVIRTUALSCREEN,
        SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
    };
    use xa11y::ScreenshotProvider;

    fn failed(what: impl std::fmt::Display) -> PlatformError {
        PlatformError::Recognition(what.to_string())
    }

    /// The window in front's rectangle as drawn, without the invisible resizing border.
    fn window_rect() -> Option<(i32, i32, i32, i32)> {
        // SAFETY: plain Win32 calls; the rectangle is written into a local.
        unsafe {
            let hwnd = GetForegroundWindow();
            let mut rect = RECT::default();
            let drawn = DwmGetWindowAttribute(
                hwnd,
                DWMWA_EXTENDED_FRAME_BOUNDS,
                (&raw mut rect).cast(),
                std::mem::size_of::<RECT>() as u32,
            )
            .is_ok();
            if !drawn && GetWindowRect(hwnd, &mut rect).is_err() {
                return None;
            }
            Some((rect.left, rect.top, rect.right, rect.bottom))
        }
    }

    /// Every screen: the virtual desktop.
    fn screen_rect() -> (i32, i32, i32, i32) {
        // SAFETY: plain Win32 calls with no preconditions.
        unsafe {
            let left = GetSystemMetrics(SM_XVIRTUALSCREEN);
            let top = GetSystemMetrics(SM_YVIRTUALSCREEN);
            (
                left,
                top,
                left + GetSystemMetrics(SM_CXVIRTUALSCREEN),
                top + GetSystemMetrics(SM_CYVIRTUALSCREEN),
            )
        }
    }

    /// The recognizer for `language` (a BCP 47 tag), else for the first of the user's
    /// languages Windows has one for.
    fn engine_for(language: &str) -> Result<OcrEngine, PlatformError> {
        let wanted = Language::CreateLanguage(&windows::core::HSTRING::from(language))
            .and_then(|language| OcrEngine::TryCreateFromLanguage(&language));
        match wanted {
            Ok(engine) => Ok(engine),
            Err(_) => OcrEngine::TryCreateFromUserProfileLanguages()
                .map_err(|_| failed("no recognizer for the user's languages")),
        }
    }

    /// The words Windows' recognizer for `language` reads in `area`, each where it is on the
    /// desktop, in the order it reads them.
    pub fn recognize(
        area: RecognitionArea,
        language: &str,
    ) -> Result<Vec<ScreenRun>, PlatformError> {
        let (left, top, right, bottom) = match area {
            RecognitionArea::Window => window_rect().ok_or_else(|| failed("no window in front"))?,
            RecognitionArea::Screen => screen_rect(),
            RecognitionArea::Rect(left, top, right, bottom) => (left, top, right, bottom),
        };
        let width = u32::try_from(right - left).map_err(failed)?;
        let height = u32::try_from(bottom - top).map_err(failed)?;
        let engine = engine_for(language)?;
        let largest = OcrEngine::MaxImageDimension().map_err(failed)?;
        let capture = xa11y_windows::WindowsScreenshot::new()?.capture_region(xa11y::Rect {
            x: left,
            y: top,
            width,
            height,
        })?;
        let scale = enlargement(capture.width, capture.height, largest);
        let pixels = recognizer_pixels(&capture.pixels, capture.width, capture.height, scale);
        let writer = DataWriter::new().map_err(failed)?;
        writer.WriteBytes(&pixels).map_err(failed)?;
        let buffer = writer.DetachBuffer().map_err(failed)?;
        let to_i32 = |v: u32| i32::try_from(v * scale).map_err(failed);
        let bitmap = SoftwareBitmap::CreateCopyWithAlphaFromBuffer(
            &buffer,
            BitmapPixelFormat::Bgra8,
            to_i32(capture.width)?,
            to_i32(capture.height)?,
            BitmapAlphaMode::Premultiplied,
        )
        .map_err(failed)?;
        let result = engine
            .RecognizeAsync(&bitmap)
            .map_err(failed)?
            .join()
            .map_err(failed)?;
        let mut words = Vec::new();
        for line in result.Lines().map_err(failed)? {
            for word in line.Words().map_err(failed)? {
                let text = word.Text().map_err(failed)?.to_string();
                let place = word.BoundingRect().map_err(failed)?;
                words.push(desktop_word(
                    text,
                    (place.X, place.Y, place.Width, place.Height),
                    scale,
                    (left, top),
                ));
            }
        }
        Ok(words)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captured_pixels_become_opaque_bgra() {
        let rgba = [1, 2, 3, 0, 4, 5, 6, 0];
        assert_eq!(
            recognizer_pixels(&rgba, 2, 1, 1),
            [3, 2, 1, 255, 6, 5, 4, 255]
        );
    }

    #[test]
    fn a_small_picture_is_enlarged_by_repeating_its_pixels() {
        let rgba = [1, 2, 3, 0];
        assert_eq!(
            recognizer_pixels(&rgba, 1, 1, 2),
            [3, 2, 1, 255, 3, 2, 1, 255, 3, 2, 1, 255, 3, 2, 1, 255]
        );
        assert_eq!(enlargement(120, 40, 10_000), 4);
        assert_eq!(enlargement(600, 400, 10_000), 1);
        // Never past the recognizer's largest side.
        assert_eq!(enlargement(3000, 50, 10_000), 3);
    }

    #[test]
    fn a_word_is_placed_back_on_the_desktop() {
        let word = desktop_word("Start".to_owned(), (40.0, 8.0, 80.0, 24.0), 4, (300, 200));
        assert_eq!(
            (word.left, word.top, word.right, word.bottom),
            (310, 202, 330, 208)
        );
        assert_eq!(word.source, Source::Recognized);
    }
}
