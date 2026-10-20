//! The development signing script (`signing/dev-signing.ps1`), read as text: it is never run
//! by the tests (it would change certificate stores), so what the proposal's §5.11 requires
//! of it, and forbids, is checked in its words.

const SCRIPT: &str = include_str!("../signing/dev-signing.ps1");

/// The script's lines without its comments and its help block, lowercased.
fn code() -> String {
    let mut in_help = false;
    let mut kept = Vec::new();
    for line in SCRIPT.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("<#") {
            in_help = true;
        }
        if in_help {
            if trimmed.ends_with("#>") {
                in_help = false;
            }
            continue;
        }
        if trimmed.starts_with('#') {
            continue;
        }
        kept.push(line.to_lowercase());
    }
    kept.join("\n")
}

/// A code-signing-only, end-entity certificate whose key never leaves the dev box, valid
/// for a year, trusted only in the local machine's Trusted Root store.
#[test]
fn the_certificate_is_scoped_as_the_proposal_says() {
    let code = code();
    for required in [
        "new-selfsignedcertificate",
        "-keyexportpolicy nonexportable",
        "2.5.29.37={text}1.3.6.1.5.5.7.3.3",
        "'2.5.29.19={text}'",
        "-keyusage digitalsignature",
        "(get-date).addmonths(12)",
        "-certstorelocation 'cert:\\currentuser\\my'",
        "export-certificate",
        "-certstorelocation 'cert:\\localmachine\\root'",
        "sign /fd sha256",
    ] {
        assert!(code.contains(required), "the script lacks {required:?}");
    }
}

/// None of the pasted answer's dangerous steps (proposal §4, C10-C12), and no private key
/// written out.
#[test]
fn nothing_beyond_the_root_store_is_changed() {
    let code = code();
    for forbidden in [
        "bcdedit",
        "testsigning",
        "nointegritychecks",
        "trustedpublisher",
        "export-pfxcertificate",
        "-executionpolicy bypass",
        "set-executionpolicy",
    ] {
        assert!(!code.contains(forbidden), "the script uses {forbidden:?}");
    }
}

/// Every change asks first (`-WhatIf` shows it), and the machine-wide steps refuse to run
/// unelevated.
#[test]
fn every_change_can_be_previewed() {
    let code = code();
    assert!(code.contains("supportsshouldprocess = $true"));
    let changes = [
        "new-selfsignedcertificate",
        "import-certificate",
        "remove-item",
    ];
    for change in changes {
        assert!(code.contains(change), "{change}");
    }
    let guarded = code.matches("$pscmdlet.shouldprocess(").count();
    assert!(guarded >= 5, "{guarded} guarded steps");
    assert!(code.contains("function assert-elevated"));
    assert_eq!(
        code.matches("assert-elevated\n").count(),
        2,
        "TrustOnLab, UndoLab"
    );
}
