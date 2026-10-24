# Handoff: Okur's settings as JAWS's, round 2: questions, voices, applications (2026-10-24)

Follows [`2026-10-24-settings-redesign.md`](2026-10-24-settings-redesign.md) and covers its 19
later commits (`86f9f0a7` to `a470fb09`) and this one. They were made under
[`docs/plans/2026-10-24-everyday-gaps-round-30.md`](../plans/2026-10-24-everyday-gaps-round-30.md)
§1.

The owner's order stands:
1. the settings redesign to its end;
2. then the rest of Microsoft Word;
3. then punctuation by word and the README's "Not yet" list.

This round finished milestone 4's core (voice contexts and profiles) and two of milestone 5's
three dialogs. What is left of the settings is listed in the next plan, a short stretch before
Word.

The next plan is [`docs/plans/2026-10-24-everyday-gaps-round-31.md`](../plans/2026-10-24-everyday-gaps-round-31.md).

## What was built

The design and its record of JAWS's behavior:
[`../references/okur-settings-redesign.md`](../references/okur-settings-redesign.md). §1.1 now also
holds Settings Center's questions, Quick Settings' Tab order, Voice Adjustment's contexts and
profiles, Select a Voice Profile, Manage Application Settings and Restore to Factory Settings, as
old-pc's walks heard them.

### Settings Center and Quick Settings, as walked

- **Space** says the new value and its place: "Characters | 2 of 4". Shift+Space goes on the same
  way (`11497973`).
- **Apply** comes before OK and Cancel once something changed (`11497973`).
- **Cancel, Escape or the close box** with a change not applied asks, in JAWS's words with Okur's
  name: "Okur Settings Center dialog | You have made changes to Notepad's settings. Do you want
  to save them? | Yes Button". Its buttons are Yes, No and Cancel; Escape goes back to Settings
  Center (`c78f356e`, `855c652c`).
- **okur-ui:** a dialog whose Escape is its own key stays open on Cancel and the close box, and
  Okur hears the press.
- **Quick Settings** has JAWS's Tab order: the search box, the tree with its groups open, the
  description, "Expand Tree View" (checked; clearing it closes every group), OK. It has no Apply
  and no Cancel (`855c652c`).
- **Lifetimes** have JAWS's words: "Restore setting when Okur exits", "Restore setting when focus
  changes", "Save the setting".

### Voices (milestone 4)

- **Voice contexts** (`4b1a32ee`): `voice.contexts` keeps the PC Cursor's, the Okur Cursor's, the
  Keyboard's and the Tutor and Message voice: person, rate, punctuation and pitch. Each part a
  context leaves out is All Contexts'. The speech thread speaks:
  - typing echo in the keyboard's voice;
  - Okur's own messages (`say_message`: "Okur Cursor", "Faster", "left mouse button", "Not in a
    dialog box", ...) in the message voice;
  - everything else in the cursor's voice, the Okur cursor's while it is active.
  A voice is set only when the context speaking changes.
- **A context's Pitch** (`840b241e`): everything is said at the context's pitch, a capital's
  raise added. Pitch 0 speaks as before. A Say All's parts are not pitched yet.
- **Voice Adjustment:**
  - Profile Name comes first: the profiles in alphabetical order, a synthesizer's own named
    "OneCore - (factory)", as JAWS's "SAPI 5X - (factory)".
  - Delete Profile is there for a saved profile.
  - The "Voice" group: Adjust offers JAWS's five contexts ("Okur Cursor Voice" for JAWS's), then
    Person, Rate, Punctuation, and Pitch for a context or Volume for All Contexts. A change in All
    Contexts is every context's.
  - Save As... asks "Voice Profile Name:" in Save Voice Profile.
  - OK is there once something changed, but not for a synthesizer's own profile changed: that one
    is kept only by Save As (`76a611eb`).
- **Select a Voice Profile** (Insert+Ctrl+S, Default.JKM's `SelectAVoiceProfile`, and Options,
  Voices) (`bab3d80f`):
  - Default chooses the profile for all applications; "For notepad" chooses the application's,
    with Use Default first.
  - Choosing a profile copies its voice into the place's settings, with its name.
  - Use Default leaves the application nothing of its own.

### Applications (milestone 5)

- **Manage Application Settings** (`a470fb09`):
  - It lists the applications with settings of their own, checked while those are used.
  - Its buttons are Enable All, Disable All, Remove Selected, OK, Cancel.
  - An application unchecked keeps its settings but has those for all applications
    (`disabled_applications`). Remove Selected forgets its settings.
- **Restore to Factory Settings** asks first, in JAWS's words without "and restart JAWS": Okur's
  defaults apply at once, so there is no restart. Yes forgets every saved setting.

## Harness

- `settings-questions-walk.ps1` takes each dialog's keys as parameters and leaves any dialog
  still open by Escape, then No.
- `jaws-window-walk.ps1 -Parts` chooses its parts: `contexts`, `profile`, `profiles`, `saveas`,
  `manage`, `wizard`. `restore` was removed after its incident, below.
- `okur-ui-tree-probe.ps1` has two new modes:
  - `-Question`: Settings Center's question over the dialog;
  - `-Voice`: Voice Adjustment's shape, its Profile Name choices replaced as after Save As.
- The skill's troubleshooting notes gain §7: a JAWS Yes/No question left open, and how to answer
  it No safely.

## Found, and how

- **Restore to Factory Settings, left open with Yes focused.** JAWS's question is a Yes/No box,
  which ignores Escape, so the walk left it open with Yes focused. The walk's next part then sent
  Alt, Down and Enter into the box's system menu; these opened Move, not Yes.
  - Plain `okur-lab keys` cannot reach JAWS's elevated dialogs.
  - UI Automation showed the focus on Yes. One Tab through `okur-lab-keys.exe` (UI Access) moved
    it to No, UI Automation confirmed, and Space answered No.
  - JAWS kept its process (pid 4968), and its settings files were unchanged (DEFAULT.JCF last
    written 2026-10-22).
  - The walk no longer opens it.
- **The description edit:** JAWS reads its first line on Tab, and okur-ui's note reads none.
  Stopping the dialog manager from selecting the note's text did not change this, and the change
  was reverted. It stays open.
- **The Rate slider:** JAWS's says its number ("Voice | Rate: 20 | left right slider | 100
  percent"); okur-ui's says "Voice Rate: left right slider | 50 percent". Open.
- **Words with punctuation (1037):** old-pc's classic Notepad stops three times inside
  `C:\Users\Ada\Downloads`, and JAWS says the whole token at each stop. A hyphen ends JAWS's word,
  and Say Word on the first stop says "\". This is in
  [`../references/jaws-punctuation-by-unit-research.md`](../references/jaws-punctuation-by-unit-research.md);
  the owner's Notepad (RichEdit) is to record next.
- **Tutor hints in the message voice** need the focus speech's hint said apart from the rest; the
  engine joins them into one text today. Open.

## Proof

`okur-lab parity` at `a470fb09`:

```
JAWS spoken parity: 8663/9400 steps match exactly (evidence recorded through the speech recorder voice)
JAWS parity (Ia2): 8760/9339 steps match exactly; 251 accepted divergences not counted (0 scenarios could not run)
```

The new steps are scenario 1037's, 85 of 100 matching. `okur-lab parity --only 1037` lists the
other 15, which are the three findings above.

`cargo test --workspace`: 1471 passed, 0 failed, 201 ignored. Tests cover each piece:
- the dialogs' models (`settings_tree.rs`, `voice_adjustment.rs`, `voice_profile.rs`,
  `applications.rs`);
- the layers (`a_saved_voice_profile_is_removed`,
  `disabled_application_settings_are_kept_but_not_used`);
- the engine (`voice_profiles_are_chosen_for_all_applications_and_one`);
- the speech thread (`a_voice_context_speaks_with_its_own_rate`).

okur-ui's new pieces were probed on old-pc with JAWS reading them. Quoted from the probes:
- "Okur Settings Center dialog | You have made changes to Notepad's settings. Do you want to save
  them? | Yes Button";
- "Profile Name: Combo box | Automatic - (factory) | 1 of 3";
- "Voice Adjust: Combo box | All Contexts | 1 of 5";
- "Fast | 2 of 4" after the choices were replaced.

## old-pc

- Idle, with Notepad in front and JAWS running with its own settings.
- Still the owner's to end: the windowless Word (pid 9688) and the PowerShell (4200) from the
  first Word fixture.
