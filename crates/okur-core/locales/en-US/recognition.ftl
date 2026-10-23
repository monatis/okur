## Recognizing the screen's text: Insert+Space, O, JAWS's Convenient OCR layer, with its
## results among the Okur cursor's lines. Okur's experimental feature, off until it is proven
## (docs/plans/2026-10-23-okur-cursor-screen-text.md).

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
# Insert+Space, O while the experimental setting is off: Okur's own words.
ocr-experimental-off = Recognizing the screen is experimental: turn it on in Okur Settings, in the Experimental group.
