# ARIA-AT's test pages

The folders here are copies of test plans' pages from W3C's ARIA and Assistive Technologies project, <https://github.com/w3c/aria-at>, at commit `6c0044502636c8fa6452579f6e10a0d7ca80b3b1`: each plan's `tests/apg/<plan>/reference/<date>/` folder, unchanged. A setup script's page the folder lacks is made from the plan's page and its `data/js/<script>.js` as ARIA-AT's build makes it (`lib/data/example-scripts-template.js`): the script before `</head>`, the "Run Test Setup" button before `</main>`.

Copyright (c) World Wide Web Consortium. The ARIA-AT repository's documents are licensed under the W3C Document License, <https://www.w3.org/copyright/document-license-2023/>; the pages are the ARIA Authoring Practices Guide's examples, <https://github.com/w3c/aria-practices>, under the W3C Software and Document License, <https://www.w3.org/copyright/software-license-2023/>.

They are used here to record how a screen reader reads them, as ARIA-AT's own testers do (`crates/okur-lab/src/aria_at.rs`).
