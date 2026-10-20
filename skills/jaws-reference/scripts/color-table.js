// Writes Okur's English color table (crates/okur-core/locales/en-US/colors.txt) from X.Org's
// rgb.txt and its license (https://gitlab.freedesktop.org/xorg/app/rgb: rgb.txt, COPYING).
// Usage: node color-table.js <rgb.txt> <COPYING> > crates/okur-core/locales/en-US/colors.txt
const fs = require('fs');
const [rgbPath, copyingPath] = process.argv.slice(2);
// X11's 2014 web additions, which JAWS's names do not have [obs, 618].
const dropped = /x11|web|^lime$|rebecca|^crimson$|^dark (grey|gray|blue|cyan|magenta|red)$|^Dark(Grey|Gray|Blue|Cyan|Magenta|Red)$|^light green$|^LightGreen$|^aqua$|^fuchsia$/i;
const table = [];
const seen = new Map();
for (const line of fs.readFileSync(rgbPath, 'utf8').split('\n')) {
  const m = line.match(/^\s*(\d+)\s+(\d+)\s+(\d+)\s+(.+?)\s*$/);
  if (!m) continue;
  const [r, g, b] = [m[1], m[2], m[3]].map(Number);
  let name = m[4];
  if (dropped.test(name)) continue;
  const key = `${r} ${g} ${b}`;
  if (seen.has(key)) continue;
  if (/[a-z][A-Z]/.test(name)) name = name.replace(/([a-z])([A-Z])/g, '$1 $2');
  seen.set(key, table.length);
  table.push([key, name]);
}
// The colors JAWS was seen to name otherwise than X11, or that X11 lacks [obs, 565, 618]; and
// two of its own that 619's grid shows, each where JAWS said it: "crimson" at 102 0 51 (CSS
// crimson is "firebrick3" to JAWS) and "Dark Gray" at 153 255 255.
const own = [
  '128 128 128\tgray', '128 0 0\tmaroon', '128 0 128\tpurple', '0 128 0\tlime green 2',
  '169 169 169\tDark Gray', '128 128 0\tolive', '192 192 192\tsilver', '0 128 128\tteal',
  '75 0 130\tindigo', '102 0 51\tcrimson', '153 255 255\tDark Gray',
];
const extra = [];
for (const line of own) {
  const [key, name] = line.split('\t');
  const k = key.trim().split(/\s+/).join(' ');
  if (seen.has(k)) table[seen.get(k)][1] = name.trim(); else extra.push([k, name.trim()]);
}
const copying = fs.readFileSync(copyingPath, 'utf8').trimEnd().split('\n').map(l => ('# ' + l).trimEnd());
const out = [
  "# Okur's English color names, for Say Color (Insert+5): one `r g b<TAB>name` line per color.",
  "# A color not listed is said by its nearest line's name; of two lines for one color, the first.",
  '#',
  "# The names are X11's (X.Org's app/rgb, rgb.txt), as JAWS says them: CamelCase names in",
  '# words ("PaleGreen2" as "Pale Green2"), without the web colors X11 added in 2014, and with the',
  '# colors JAWS was seen to name otherwise or that X11 lacks (corpus scenarios 565, 618, 619).',
  '# Written by skills/jaws-reference/scripts/color-table.js; change that, not this file.',
  '# X11\'s rgb.txt is distributed under the following terms:',
  '#',
  ...copying,
  '#',
  ...table.map(([k, n]) => `${k}\t${n}`),
  ...extra.map(([k, n]) => `${k}\t${n}`),
];
process.stdout.write(out.join('\n') + '\n');
