## Recognizing the screen's text: Insert+Space, O, JAWS's Convenient OCR layer, with its
## results among the Okur cursor's lines (docs/plans/2026-10-23-okur-cursor-screen-text.md).

# The layer key O [obs, 996/s2].
ocr-layer = OCR
# W, S or C: recognition starts, said with its engine [obs, 996/s3, 997/s3]. Okur's engine is
# Windows' own, which JAWS's option calls Microsoft OCR.
ocr-started = OCR started using { $engine }.
ocr-engine-windows = Microsoft OCR
# Recognition is done; the Okur cursor is then active [obs, 996/s3].
ocr-finished = finished.
# Q while recognizing [obs, 1006/s3].
ocr-canceled = OCR cancelled.
# B: Background OCR, the window recognized again and again [obs, 1001/s3].
ocr-background-started = Background OCR started
# Q while Background OCR runs [obs, 1001/s14].
ocr-background-cancelled = Background OCR cancelled
# Alt+Delete says it after the cursor's place [obs, 1001/s4, s11].
ocr-background-enabled = Background OCR is enabled

## Recognizing a file: F (OCRSelectedFile), and Enter in a window of files, read the picture or
## PDF selected in File Explorer into the Results Viewer.

# A picture or PDF is selected, and its recognition starts [obs, 1024/s3, 1025/s3].
ocr-document-started = Document OCR started.
# The Results Viewer's title: the file's name without its extension [obs, 1024/s10].
ocr-results-title = { $name } - OCR Results
# Its message box when nothing is selected [obs, 1026/s3], or a file that is neither a picture
# nor a PDF [obs, 1027/s3].
ocr-failed-no-file = OCR Failed to start. No file currently selected.
ocr-failed-unsupported = OCR Failed to start. Unsupported file currently selected.
# Said, not shown, for F in a window that shows no files. JAWS's third sentence sends the user
# to its own context menu item, which Okur does not add to Windows [obs, 1030/s3].
ocr-failed-to-start = OCR Failed to start.
ocr-file-explorer-needed = To OCR a file, you must open Windows Explorer and select a file to be recognized.
# Q with nothing being recognized [obs, 1029/s3].
ocr-cancel-nothing = You can not cancel a Recognition job that does not exist.

# D (OCRDocument): JAWS recognizes a PDF open in Adobe Acrobat, and elsewhere says so [obs,
# 1031/s3]. Okur recognizes no document open in a reader yet; it says JAWS's first sentence and
# points to F, never that it would in Acrobat (an accepted divergence).
ocr-document-use-file = To OCR a PDF document, select it in Windows Explorer and use the letter F.

## ? (OCRLayerHelp): the layer's keys in a viewer of their own, opened without a word, as
## JAWS's [obs, 1032/s3-s21]. Only the keys Okur has: JAWS's lines for A (a scanner or camera),
## D (a document open in Acrobat), R (a file into Word) and Ctrl+B are left out.

ocr-help-title = OCR Layer Help Screen
ocr-help-intro = The following commands are available in the OCR layer.
ocr-help-file = From Windows Explorer, you can OCR any Image File or PDF using the letter F.
ocr-help-screen = From the computer screen itself, you can OCR a Control with C, Window with W, or the entire Screen with S.
ocr-help-background = From any window, you can start Background OCR using the letter B.
ocr-help-enter = By simply pressing Enter, Okur will choose the most appropriate command based on the current application and its state.
ocr-help-quit = You can press Q to quit or cancel an OCR job.
# Its message box when more than one item is selected [obs, 1033/s3].
ocr-failed-multiple-files = OCR Failed to start. Multiple files currently selected.
