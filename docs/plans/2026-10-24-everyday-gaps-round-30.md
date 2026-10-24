# Plan: everyday-use gaps, round 30 (2026-10-24)

Follows [`../handoff/2026-10-24-settings-redesign.md`](../handoff/2026-10-24-settings-redesign.md).
The owner's order:
1. Okur's settings redesigned as JAWS's, to its end (milestones 4 and 5 and what is left of
   1-3);
2. then the rest of Microsoft Word (the owner, 2026-10-24: "let's schedule it after you're done
   with settings thing");
3. then the README's "Not yet" list, in its order.

Each item is found in JAWS's documentation, recorded on old-pc, then built. JAWS's accidents
become accepted divergences. old-pc always has a round queued behind the running one. Okur's own
words never use JAWS's name (AGENTS.md §1.5). A stretch is a few hours or about thirty commits;
it ends with a handoff and the next plan, and the README follows a finished area.

## 0. old-pc

- Idle. okur-lab parity at `09608e47`: "8675/9239 steps match exactly; 251 accepted
  divergences".
- **Queue next:**
  - **Okur's dialogs as Okur reads them:** a scenario family that runs Okur (not JAWS) on old-pc
    and records its speech of its own Settings Center, Quick Settings, Basics and window. JAWS's
    recordings of the same dialogs are the reference (the walks' speech, `okur-settings-redesign.md`
    §1.1). Find how: `okur run --speech-log` with JAWS's speech left out, or the speech recorder
    voice.
  - **Settings Center's open questions:** Shift+Space; Enter on a setting with no search; F6;
    Quick Settings' Tab order and Apply; JAWS's own words for the three lifetimes; whether JAWS's
    window starts minimized; the menus' access keys (old-pc's Access Key setting is Off: turn it
    on for one walk, then back).
  - **Punctuation by word** (backlog): scenario `1037-desktop-notepad-words-with-punctuation`
    and its fixture `desktop-notepad-paths.ps1`, from
    [`../references/jaws-punctuation-by-unit-research.md`](../references/jaws-punctuation-by-unit-research.md).
    Check 1037 is still free.

## 1. Settings, to the end ([`../references/okur-settings-redesign.md`](../references/okur-settings-redesign.md) §6)

1. **Milestone 4, Voices:**
   - voice contexts (PC cursor, Okur cursor, keyboard, messages) as settings, each inheriting
     from All Context, with the voice, rate, punctuation, pitch and volume;
   - voice profiles: saved by name, factory profiles, one chosen globally and per application;
   - Voice Adjustment, as walked (§1.1: Profile Name, Delete Profile, Synthesizer Language, Speak
     Sample, Voice Adjust, Voice Person, Rate, Punctuation, Volume, Spell Rate Change, Upper Case
     Pitch Change, Save As, Cancel);
   - Select a Voice Profile (Insert+Ctrl+S); Options, Voices ▸ and Language, Voice Profiles ▸ in
     Okur's window.
2. **Milestone 5:** Startup Wizard (Help), Manage Application Settings and Restore to Factory
   Settings (Options).
3. **What is left of 1-3:**
   - Screen Echo, Smart Navigation and Indicate Headings in the tree, once their values' names are
     recorded;
   - F6 to an edit for numbers;
   - the lifetimes' icons;
   - Remove and Remove All in Last Changed Settings, and its history kept across restarts;
   - Basics' Automatically Start Okur... and Run Okur from System Tray, with setup's start entries
     and desktop icon (backlog, Installer);
   - Tutor Messages' and Access Key's other choices;
   - Commands Search in Help once Okur has it.

## 2. Word, the rest (the owner, 2026-10-24)

From plan 29 §2, unchanged: Say All with the structure (518/s20); Say Font's line spacing
(1034/s1, s3); Word's other views; tables without a header row; bold, italic and underline in Say
Font; then Excel (521) and Outlook. The README's Word entry then loses its "Not yet".

## 3. Punctuation by word

The backlog's entry: record 1037 at Most, None and Some, then speak words with every symbol named
wherever they are read (the virtual cursor, Say Word, the Okur cursor, the Virtual Viewer), with
JAWS's word boundaries (`C:` and `\` apart), and add known-gap tests for 106/s4, s6, s23 and
213/s11 meanwhile.

## 4. Still open from before

Plan 29 §3-§7: Convenient OCR's rest, Smart Navigation and Screen Layout, the replay's speech
clock, the hardening review's open findings, the backlog, and the owner's items on old-pc.
