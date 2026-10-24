//! A file's text read by a recognizer, for Convenient OCR's F (`OCRSelectedFile`: the picture or
//! PDF selected in File Explorer, to the Results Viewer;
//! `docs/plans/2026-10-23-okur-cursor-screen-text.md` S4): what the platform gives the engine,
//! and what the recorder saves after a step for the replay.

use serde::{Deserialize, Serialize};

/// What recognizing the selected file came to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum FileRecognition {
    /// The file's pages, each its lines in the order the recognizer read them; a picture has
    /// one page.
    Read {
        name: String,
        pages: Vec<Vec<String>>,
    },
    /// No file is selected in the window in front, or it is not a window of files.
    NoFile,
    /// More than one item is selected: JAWS recognizes one file at a time [obs, 1033/s3].
    MultipleFiles,
    /// The selected item is neither a picture Windows decodes nor a PDF.
    Unsupported { name: String },
    /// The recognizer could not read the file (no recognizer for the language, a damaged
    /// file).
    Failed { name: String },
    /// Recognition was cancelled before the file was read.
    Cancelled,
}

impl FileRecognition {
    /// The file's lines, page after page.
    pub fn lines(&self) -> impl Iterator<Item = &str> {
        let pages: &[Vec<String>] = match self {
            Self::Read { pages, .. } => pages,
            _ => &[],
        };
        pages.iter().flatten().map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_recognition_is_saved_with_its_outcome() {
        let read = FileRecognition::Read {
            name: "Sign.png".into(),
            pages: vec![vec!["Opening hours".into()]],
        };
        let json = serde_json::to_string(&read).unwrap_or_default();
        assert_eq!(
            json,
            r#"{"outcome":"read","name":"Sign.png","pages":[["Opening hours"]]}"#
        );
        assert_eq!(
            serde_json::from_str::<FileRecognition>(&json).ok(),
            Some(read)
        );
        assert_eq!(
            serde_json::from_str::<FileRecognition>(r#"{"outcome":"no_file"}"#).ok(),
            Some(FileRecognition::NoFile)
        );
    }

    #[test]
    fn its_lines_run_page_after_page() {
        let read = FileRecognition::Read {
            name: "Letter.pdf".into(),
            pages: vec![vec!["one".into(), "two".into()], vec!["three".into()]],
        };
        assert_eq!(read.lines().collect::<Vec<_>>(), ["one", "two", "three"]);
        assert_eq!(FileRecognition::NoFile.lines().count(), 0);
    }
}
