# JAWS's punctuation levels: the symbol table and what each level does

What JAWS 2025 does with symbols at each punctuation level, from its own interface and from
recordings on old-pc. The basis for Okur's punctuation setting (plan
[`2026-10-18-reading-settings.md`](../plans/2026-10-18-reading-settings.md), step 3). The
documentation's part is in [`jaws-reading-settings-research.md`](jaws-reading-settings-research.md),
Item 1.

## How this was read

- **The table** is JAWS's Customize Punctuation dialog (Settings Center, Default (All
  Applications); search "punctuat"; the dialog's list view, 75 rows), read on old-pc on
  2026-10-19 the way a user hears it: Down on each row, then JAWS's speech history (`okur
  jaws-history`), which held "symbol | description | level | n of 75". Nothing was changed;
  the dialog and Settings Center were closed with Escape, and the voice profile and
  `DEFAULT.JCF` were unchanged afterwards.
- **The levels' effect** is scenario 400 (`corpus/scenarios/400-desktop-punctuation-cycle.json`,
  three trials): Insert+Shift+2 four times on the desktop reading fixture, lines 1 to 4 read
  after each press. Its `spoken` records are what JAWS sent the synthesizer.

Levels are cumulative: a symbol is named at its level and every level above it. None, Some,
Most, All.

## The table

| Symbol | Spoken as | Named from |
|---|---|---|
| `!` | exclaim! | All |
| `"` | quote | Most |
| `#` | number | Some |
| `$` | dollar | Some |
| `%` | percent | Some |
| `&` | and | Some |
| `'` | apostrophe | All |
| `(` | left paren | Most |
| `)` | right paren | Most |
| `*` | star | Some |
| `+` | plus | Some |
| `,` | comma, | All |
| `-` | dash | Most |
| `.` | period. | All |
| `/` | slash | Some |
| `:` | colon | Most |
| `;` | semi colon | Most |
| `<` | less | Most |
| `=` | equals | Some |
| `>` | greater | Most |
| `?` | question? | All |
| `@` | at | Some |
| `[` | left bracket | Most |
| `\` | back slash | Most |
| `]` | right bracket | Most |
| `^` | carat | Some |
| `_` | underline | Most |
| `` ` `` | grawv | Most |
| `{` | left brace | Some |
| `\|` | vertical bar | Most |
| `}` | right brace | Some |
| `~` | til de | Most |
| U+000B | vertical tab | Most |
| U+000C | page break | Some |
| U+001E | non breaking hyphen | Most |
| U+00A1 `¡` | inverted exclaim | All |
| U+00A6 `¦` | broken bar | Most |
| U+00AB `«` | left double angle bracket | Most |
| U+00AD (soft hyphen) | Soft Hyphen | (no level given) |
| U+00B7 `·` | Middle Dot | Most |
| U+00BB `»` | right double angle bracket | Most |
| U+00BF `¿` | inverted question | All |
| U+0192 `ƒ` | florin | All |
| U+2013 `–` | en dash | All |
| U+2014 `—` | em dash | All |
| U+2018 `‘` | apostrophe | Most |
| U+2019 `’` | apostrophe | Most |
| U+201A `‚` | low single quote | Most |
| U+201B `‛` | reversed single quote | Most |
| U+201C `“` | left quote | Most |
| U+201D `”` | right quote | Most |
| U+201E `„` | low quote | Most |
| U+201F `‟` | reversed quote | Most |
| U+2020 `†` | single dagger | All |
| U+2021 `‡` | double dagger | All |
| U+2022 `•` | bullet | None |
| U+2023 `‣` | triangular bullet | None |
| U+2026 `…` | ellipsis | All |
| U+2043 `⁃` | dash bullet | None |
| U+204C `⁌` | leftwards bullet | None |
| U+204D `⁍` | rightwards bullet | None |
| U+2219 `∙` | Bullet Operator | Most |
| U+25C9 `◉` | japanese tainome bullet | None |
| U+25CF `●` | star | Some |
| U+25E6 `◦` | white bullet | None |
| U+2619 `☙` | rotated heart bullet | None |
| U+2B9A `⮚` | Hollow Arrow Bullet | None |
| U+F06E | emdash bullet | Some |
| U+F071 | hollow square bullet | None |
| U+F076 | starred bullet | None |
| U+F0A7 | filled square bullet | None |
| U+F0B7 | bullet | None |
| U+F0D8 | arrow bullet | None |
| U+F0E0 | right arrow bullet | None |
| U+F0FC | checkmark bullet | None |

The dialog lists the non-ASCII symbols by code point (`u+2018`); the eleven symbols of U+00A1 to
U+2026 that it lists as characters came through the console garbled and are identified by
their names. The names keep JAWS's spelling ("grawv", "til de", "carat", "semi colon"), which
the synthesizer is meant to say as written. The bullets are named even at None. The dialog's
Advanced list (special symbols that are not punctuation) is not read yet.

## What the levels do to running text (400)

Line 4 of the fixture, `Price $9.95, 50% off; total = 3 * 4 + 1 - 2 / 5 ^ 2 ~ 7`, as JAWS sent it
to the synthesizer (Say Line, Down; whitespace collapsed here):

| Level | Sent |
|---|---|
| None | Price 9.95, 50% off; total 3 4 1 2 5 2 7 |
| Some | Price dollar 9.95, 50% off; total equals 3 star 4 plus 1 2 slash 5 carat 2 7 |
| Most | Price dollar 9.95, 50% off semi colon total equals 3 star 4 plus 1 dash 2 slash 5 carat 2 til de 7 |
| All | Price dollar 9 point 9 5 comma, 50% off semi colon total equals 3 star 4 plus 1 dash 2 slash 5 carat 2 til de 7 |

Line 2, `Mail ada@example.org or see https://example.org/a-b_c?x=1&y=2#top`:

| Level | Sent |
|---|---|
| None | Mail ada example. org or see https: example. org a b c? x 1 y 2 top |
| Some | Mail ada at example dot org or see https: slash slash example dot org slash a b c? x equals 1 and y equals 2 number top |
| Most | Mail ada at example dot org or see https colon slash slash example dot org slash a dash b underline c? x equals 1 and y equals 2 number top |
| All | ... slash a dash b underline c question? x equals 1 ... |

Line 1, `Hello, world. Is it done? Yes! Wait; maybe: no.`: unchanged at None and Some; "Wait semi
colon maybe colon no." at Most; at All every mark named: "Hello comma, world period. Is it
done question? Yes exclaim! Wait semi colon maybe colon no period.".

Line 3, `C:\Users\Ada\notes.txt and /usr/local/bin`: None "C: Users Ada notes. txt and usr local
bin"; Some "C: Users Ada notes dot txt and slash usr slash local slash bin"; Most and All "C
colon back slash Users back slash Ada back slash notes dot txt and slash usr slash local
slash bin".

So, as observed:
- **A symbol at or below the level is replaced by its name**, with spaces around it.
- **Above the level, sentence punctuation stays as the character** (`, . ; : ? !`), so the
  voice still pauses; **other symbols become a space** (`$ = * + / ^ ~ @ & # - _ \`).
- **A period between letters** (`example.org`, `notes.txt`) is "dot" from Some, and at None it
  stays a period followed by a space ("example. org"). The help says the levels of "dot",
  "point" and a few others cannot be changed; JAWS uses them in special places.
- **A decimal number** stays whole below All ("9.95"); at All it is "9 point 9 5".
- **A percent sign after a number** stays as the character at every level ("50% off"), though
  `%` is named from Some.
- **Names that end with their symbol** ("comma,", "period.", "question?", "exclaim!") give
  the voice both, as alone (`pronounce.rs`).
- **The level is not saved** by Insert+Shift+2: it lasts until JAWS restarts. From Most the
  key gives All, None, Some, Most, and JAWS says the new level's name alone.

## Open

- The exact spacing JAWS sends (doubled spaces around names) needs the raw `spoken_records`;
  the parity compares collapsed whitespace.
- The Advanced list, Filter Repeated Characters (line 7), quotes and dashes (line 6), and
  per-context punctuation (the voice profile's contexts): later recordings.
- How the level is stored: changing it in Voice Adjustment and comparing the voice profile.
