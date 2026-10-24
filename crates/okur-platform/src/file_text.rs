//! A picture's or a PDF's text, recognized from its pixels with Windows' own recognizer, for
//! Convenient OCR's F (`OCRSelectedFile`: the file selected in File Explorer, to the Results
//! Viewer; `docs/plans/2026-10-23-okur-cursor-screen-text.md` S4). Windows.Data.Pdf draws a
//! PDF's pages, as Edge's viewer draws them, so a scanned letter is read page by page; a
//! picture is decoded by the Windows Imaging Component, turned as its camera recorded.
//! The selected file is the shell's own: the File Explorer window's view, through the shell's
//! window list, as the address bar and the selection are the view's.

use okur_core::recognized_file::FileRecognition;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

/// Set to stop the selected file's recognition at its next page (Insert+Space, O, Q). One file
/// is recognized at a time, on the actions worker's bulk lane.
static CANCEL: AtomicBool = AtomicBool::new(false);

/// Stops the selected file's recognition under way at its next page.
pub fn cancel() {
    CANCEL.store(true, Ordering::Relaxed);
}

/// How finely a PDF's page is drawn for the recognizer, in dots per inch. Windows' recognizer
/// reads text from about 10 pixels high: at 200, a 9-point footnote is 25 pixels high.
const PDF_DPI: f32 = 200.0;
/// A PDF's page size unit, in dots per inch: Windows.Data.Pdf gives page sizes in device
/// independent pixels.
const PDF_UNIT_DPI: f32 = 96.0;
/// The most pages of a PDF read: past this, the rest are left (a book would hold the
/// recognizer for an hour).
pub const MAX_PAGES: u32 = 500;

/// Whether `path` is read as a PDF; any other file is tried as a picture.
pub fn is_pdf(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("pdf"))
}

/// The pixels a page `width` by `height` in device independent pixels is drawn at: at
/// `PDF_DPI`, its longer side no longer than the recognizer's `largest`.
fn page_pixels(width: f32, height: f32, largest: u32) -> (u32, u32) {
    let scale = PDF_DPI / PDF_UNIT_DPI;
    let longer = width.max(height).max(1.0) * scale;
    let fit = (largest as f32 / longer).min(1.0);
    let side = |v: f32| ((v.max(1.0) * scale * fit).round() as u32).max(1);
    (side(width), side(height))
}

/// The size a picture `width` by `height` is given to the recognizer at: as it is, or shrunk
/// so that its longer side is no longer than the recognizer's `largest`.
fn picture_pixels(width: u32, height: u32, largest: u32) -> (u32, u32) {
    let longer = width.max(height).max(1);
    if longer <= largest {
        return (width, height);
    }
    let fit = |v: u32| ((u64::from(v) * u64::from(largest) / u64::from(longer)) as u32).max(1);
    (fit(width), fit(height))
}

/// The file's name as it is said and shown: its last part.
fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

#[cfg(windows)]
pub use recognizer::{recognize_file, selected_file};

/// What recognizing the file selected in the window in front comes to; [`cancel`] stops it.
#[cfg(windows)]
pub fn recognize_selected_file(language: &str) -> FileRecognition {
    CANCEL.store(false, Ordering::Relaxed);
    recognizer::recognize_selected_file(language, &CANCEL)
}

#[cfg(windows)]
mod recognizer {
    use super::{file_name, is_pdf, page_pixels, picture_pixels, FileRecognition, MAX_PAGES};
    use crate::ocr::{engine_for, failed};
    use crate::PlatformError;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicBool, Ordering};
    use windows::core::{Interface, HSTRING};
    use windows::Data::Pdf::{PdfDocument, PdfPageRenderOptions};
    use windows::Graphics::Imaging::{
        BitmapAlphaMode, BitmapDecoder, BitmapInterpolationMode, BitmapPixelFormat,
        BitmapTransform, ColorManagementMode, ExifOrientationMode, SoftwareBitmap,
    };
    use windows::Media::Ocr::{OcrEngine, OcrResult};
    use windows::Storage::Streams::{IRandomAccessStream, InMemoryRandomAccessStream};
    use windows::Storage::{FileAccessMode, StorageFile};
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, IServiceProvider, CLSCTX_ALL, COINIT_MULTITHREADED,
    };
    use windows::Win32::System::Variant::VARIANT;
    use windows::Win32::UI::Shell::{
        IFolderView, IShellBrowser, IShellItemArray, IShellWindows, IWebBrowserApp,
        SID_STopLevelBrowser, ShellWindows, SIGDN_FILESYSPATH, SVGIO_SELECTION, SWC_DESKTOP,
        SWFO_NEEDDISPATCH,
    };
    use windows::Win32::UI::WindowsAndMessaging::{GetClassNameW, GetForegroundWindow};

    /// The recognizer's lines, in its order.
    fn lines(result: &OcrResult) -> Result<Vec<String>, PlatformError> {
        let mut lines = Vec::new();
        for line in result.Lines().map_err(failed)? {
            let text = line.Text().map_err(failed)?.to_string();
            if !text.trim().is_empty() {
                lines.push(text);
            }
        }
        Ok(lines)
    }

    /// A picture decoded from `stream`, upright, as the recognizer takes it.
    fn decoded(
        stream: &IRandomAccessStream,
        largest: u32,
    ) -> Result<SoftwareBitmap, PlatformError> {
        let decoder = BitmapDecoder::CreateAsync(stream)
            .map_err(failed)?
            .join()
            .map_err(failed)?;
        let width = decoder.OrientedPixelWidth().map_err(failed)?;
        let height = decoder.OrientedPixelHeight().map_err(failed)?;
        let (to_width, to_height) = picture_pixels(width, height, largest);
        let transform = BitmapTransform::new().map_err(failed)?;
        if (to_width, to_height) != (width, height) {
            transform.SetScaledWidth(to_width).map_err(failed)?;
            transform.SetScaledHeight(to_height).map_err(failed)?;
            transform
                .SetInterpolationMode(BitmapInterpolationMode::Fant)
                .map_err(failed)?;
        }
        decoder
            .GetSoftwareBitmapTransformedAsync(
                BitmapPixelFormat::Bgra8,
                BitmapAlphaMode::Premultiplied,
                &transform,
                ExifOrientationMode::RespectExifOrientation,
                ColorManagementMode::ColorManageToSRgb,
            )
            .map_err(failed)?
            .join()
            .map_err(failed)
    }

    fn recognized(
        engine: &OcrEngine,
        bitmap: &SoftwareBitmap,
    ) -> Result<Vec<String>, PlatformError> {
        let result = engine
            .RecognizeAsync(bitmap)
            .map_err(failed)?
            .join()
            .map_err(failed)?;
        lines(&result)
    }

    /// The text Windows' recognizer for `language` (a BCP 47 tag) reads in the picture or PDF at
    /// `path`, page by page; `cancel` set stops it between pages.
    pub fn recognize_file(path: &Path, language: &str, cancel: &AtomicBool) -> FileRecognition {
        let name = file_name(path);
        match read_file(path, language, cancel) {
            Ok(Some(pages)) => FileRecognition::Read { name, pages },
            Ok(None) => FileRecognition::Cancelled,
            Err(Unread::NotAPicture) => FileRecognition::Unsupported { name },
            Err(Unread::Failed(e)) => {
                tracing::warn!(%e, "recognizing a file failed");
                FileRecognition::Failed { name }
            }
        }
    }

    enum Unread {
        /// Windows decodes no picture from it.
        NotAPicture,
        Failed(PlatformError),
    }

    impl From<PlatformError> for Unread {
        fn from(e: PlatformError) -> Self {
            Self::Failed(e)
        }
    }

    fn read_file(
        path: &Path,
        language: &str,
        cancel: &AtomicBool,
    ) -> Result<Option<Vec<Vec<String>>>, Unread> {
        let engine = engine_for(language)?;
        let largest = OcrEngine::MaxImageDimension().map_err(failed)?;
        let file = StorageFile::GetFileFromPathAsync(&HSTRING::from(path.as_os_str()))
            .map_err(failed)?
            .join()
            .map_err(failed)?;
        if !is_pdf(path) {
            let stream = file
                .OpenAsync(FileAccessMode::Read)
                .map_err(failed)?
                .join()
                .map_err(failed)?;
            let bitmap = decoded(&stream, largest).map_err(|_| Unread::NotAPicture)?;
            return Ok(Some(vec![recognized(&engine, &bitmap)?]));
        }
        let document = PdfDocument::LoadFromFileAsync(&file)
            .map_err(failed)?
            .join()
            .map_err(failed)?;
        let count = document.PageCount().map_err(failed)?.min(MAX_PAGES);
        let mut pages = Vec::new();
        for index in 0..count {
            if cancel.load(Ordering::Relaxed) {
                return Ok(None);
            }
            let page = document.GetPage(index).map_err(failed)?;
            let size = page.Size().map_err(failed)?;
            let (width, height) = page_pixels(size.Width, size.Height, largest);
            let options = PdfPageRenderOptions::new().map_err(failed)?;
            options.SetDestinationWidth(width).map_err(failed)?;
            options.SetDestinationHeight(height).map_err(failed)?;
            let stream = InMemoryRandomAccessStream::new().map_err(failed)?;
            page.RenderWithOptionsToStreamAsync(&stream, &options)
                .map_err(failed)?
                .join()
                .map_err(failed)?;
            let stream: IRandomAccessStream = stream.cast().map_err(failed)?;
            let bitmap = decoded(&stream, largest)?;
            pages.push(recognized(&engine, &bitmap)?);
        }
        Ok(Some(pages))
    }

    /// The file selected in the File Explorer window `window`, or on the desktop when `window`
    /// is the desktop's: the first of the selection, as JAWS reads one file. `Ok(None)` when
    /// nothing is selected or `window` is no window of files.
    pub fn selected_file(window: HWND) -> Result<Option<PathBuf>, PlatformError> {
        // SAFETY: COM for this thread's shell calls; a thread already in COM keeps its own.
        let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        // SAFETY: the shell's window list, an out-of-process COM server.
        let windows: IShellWindows =
            unsafe { CoCreateInstance(&ShellWindows, None, CLSCTX_ALL) }.map_err(failed)?;
        let Some(browser) = shell_browser(&windows, window)? else {
            return Ok(None);
        };
        // SAFETY: COM calls on interfaces the shell gave; results are owned by the wrappers.
        unsafe {
            let view = browser.QueryActiveShellView().map_err(failed)?;
            let folder: IFolderView = view.cast().map_err(failed)?;
            let Ok(items) = folder.Items::<IShellItemArray>(SVGIO_SELECTION) else {
                return Ok(None);
            };
            if items.GetCount().map_err(failed)? == 0 {
                return Ok(None);
            }
            let item = items.GetItemAt(0).map_err(failed)?;
            let Ok(path) = item.GetDisplayName(SIGDN_FILESYSPATH) else {
                // A selected item with no file behind it (a library, a phone's folder).
                return Ok(None);
            };
            let text = path.to_string().map_err(failed);
            windows::Win32::System::Com::CoTaskMemFree(Some(path.0.cast()));
            Ok(Some(PathBuf::from(text?)))
        }
    }

    /// The shell's browser of `window`: a File Explorer window's, or the desktop's.
    fn shell_browser(
        windows: &IShellWindows,
        window: HWND,
    ) -> Result<Option<IShellBrowser>, PlatformError> {
        let top_level = |dispatch: windows::Win32::System::Com::IDispatch| {
            let provider: IServiceProvider = dispatch.cast().map_err(failed)?;
            // SAFETY: a COM call on the shell's own service provider.
            unsafe { provider.QueryService::<IShellBrowser>(&SID_STopLevelBrowser) }.map_err(failed)
        };
        if is_desktop(window) {
            let empty = VARIANT::default();
            let mut hwnd = 0;
            // SAFETY: the desktop's entry in the shell's window list; the variants outlive it.
            let found = unsafe {
                windows.FindWindowSW(&empty, &empty, SWC_DESKTOP, &mut hwnd, SWFO_NEEDDISPATCH)
            };
            return found.map_or(Ok(None), |dispatch| top_level(dispatch).map(Some));
        }
        // SAFETY: COM calls on the shell's window list.
        let count = unsafe { windows.Count() }.map_err(failed)?;
        for index in 0..count {
            // SAFETY: as above; an index past a window just closed fails and is skipped.
            let Ok(dispatch) = (unsafe { windows.Item(&VARIANT::from(index)) }) else {
                continue;
            };
            let Ok(app) = dispatch.cast::<IWebBrowserApp>() else {
                continue;
            };
            // SAFETY: a property read on the shell's window.
            let Ok(hwnd) = (unsafe { app.HWND() }) else {
                continue;
            };
            if hwnd.0 == window.0 as isize {
                return top_level(dispatch).map(Some);
            }
        }
        Ok(None)
    }

    /// Whether `window` is the desktop's (its icons are the shell's `Progman` or `WorkerW`).
    fn is_desktop(window: HWND) -> bool {
        let mut class = [0u16; 32];
        // SAFETY: the class name is written into a local buffer of its stated size.
        let length = unsafe { GetClassNameW(window, &mut class) };
        let class = String::from_utf16_lossy(class.get(..length.max(0) as usize).unwrap_or(&[]));
        matches!(class.as_str(), "Progman" | "WorkerW")
    }

    /// What recognizing the file selected in the window in front comes to.
    pub fn recognize_selected_file(language: &str, cancel: &AtomicBool) -> FileRecognition {
        // SAFETY: plain Win32 call with no preconditions.
        let window = unsafe { GetForegroundWindow() };
        match selected_file(window) {
            Ok(Some(path)) => recognize_file(&path, language, cancel),
            Ok(None) => FileRecognition::NoFile,
            Err(e) => {
                tracing::warn!(%e, "the selected file could not be found");
                FileRecognition::NoFile
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_a4_page_is_drawn_at_200_dots_per_inch() {
        // A4 is 793.7 by 1122.5 device independent pixels.
        assert_eq!(page_pixels(793.7, 1122.5, 10_000), (1654, 2339));
        // A poster is drawn within the recognizer's largest side.
        let (width, height) = page_pixels(4000.0, 8000.0, 10_000);
        assert_eq!(height, 10_000);
        assert_eq!(width, 5000);
    }

    #[test]
    fn a_picture_is_shrunk_only_past_the_recognizers_largest_side() {
        assert_eq!(picture_pixels(1200, 400, 10_000), (1200, 400));
        assert_eq!(picture_pixels(20_000, 5_000, 10_000), (10_000, 2_500));
    }

    #[test]
    fn a_pdf_is_known_by_its_extension() {
        assert!(is_pdf(Path::new(r"C:\Letters\Scan.PDF")));
        assert!(!is_pdf(Path::new(r"C:\Letters\Scan.png")));
        assert_eq!(file_name(Path::new(r"C:\Letters\Scan.png")), "Scan.png");
    }
}
