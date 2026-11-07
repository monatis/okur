#!/usr/bin/env python3
"""Okur's live run of a web scenario on Linux, in the lab (docs/plans/2026-11-05-linux-first.md
§2.2): the run file `okur-lab live-parity` compares with JAWS's evidence of the same scenario,
as `run-scenario.ps1 -Reader okur` writes on Windows. Never evidence.

Okur runs (the example okur_linux, its silent synthesizer, its speech log kept) beside Chrome on
the scenario's fixture page; the scenario's reset keys and then each step's keys are sent by
XTest (python-xlib, by keycode), and each step's speech is what Okur's log says between its key and the quiet
after it. Key steps, and JAWS scripts by the key Okur binds to them (okur-core's example
script_key); a step of another type is skipped, and the trial marked unsuccessful.

A desktop scenario's fixture (a Windows program, `desktop-forms.ps1`) runs as its Linux replica,
`corpus/fixtures/linux/<name>.py` (GTK 4, the same controls and labels), in Chrome's place; a
desktop fixture without one is skipped.

    python3 run-scenario-linux.py <scenario.json> <run.json>

Run inside the headless desktop (a11y-stack.sh), from ~/okur, after
`cargo build -p okur-linux --example okur_linux`.
"""

import json
import os
import re
import subprocess
import sys
import tempfile
import time

OKUR = "./target/debug/examples/okur_linux"
XA11Y = "./target/debug/xa11y"
SCRIPT_KEY = "./target/debug/examples/script_key"


def recorded_web_verbosity(scenario):
    """The web verbosity JAWS read the scenario's pages at: its evidence's Chrome.JCF
    (`VirtualCursorVerbosityLevel`, 2 on old-pc), so Okur reads at the same level; None
    without one."""
    path = os.path.expanduser(f"~/okur/corpus/evidence/{scenario['id']}-evidence.json")
    try:
        with open(path, encoding="utf-8-sig") as f:
            settings = json.load(f).get("provenance", {}).get("jaws_settings", {})
    except (OSError, ValueError):
        return None
    level = settings.get("Chrome.JCF", {}).get("VirtualCursorVerbosity", {})
    return level.get("VirtualCursorVerbosityLevel")


def keys_for_scripts(scenario):
    """The key Okur binds to each JAWS script the scenario's steps run (okur-core's example
    script_key, from Okur's own key map): a script Okur binds no key to is left out."""
    names = sorted({s.get("command", "") for s in scenario.get("steps", [])
                    if s.get("type") == "jaws_script"})
    if not names or not os.path.exists(SCRIPT_KEY):
        return {}
    out = subprocess.run([SCRIPT_KEY, *names], capture_output=True, text=True).stdout
    keys = {}
    for line in out.splitlines():
        name, _, chord = line.partition("\t")
        if chord:
            keys[name] = chord
    return keys
FIXTURES = os.path.expanduser("~/okur/corpus/fixtures/")

# The scenarios' key names as X's keysym names.
NAMES = {
    "down": "Down", "up": "Up", "left": "Left", "right": "Right", "home": "Home", "end": "End",
    "pageup": "Prior", "pagedown": "Next", "pgup": "Prior", "pgdn": "Next", "tab": "Tab",
    "enter": "Return", "escape": "Escape", "esc": "Escape", "space": "space",
    "backspace": "BackSpace", "delete": "Delete", "del": "Delete", "insert": "Insert",
    "numpad5": "KP_Begin", "numpadplus": "KP_Add", "numpadminus": "KP_Subtract",
    "numpadslash": "KP_Divide", "numpadstar": "KP_Multiply", "comma": "comma",
    "period": "period", "slash": "slash", "semicolon": "semicolon", "apostrophe": "apostrophe",
    "minus": "minus", "equals": "equal", "leftbracket": "bracketleft",
    "rightbracket": "bracketright", "backslash": "backslash",
}
# The modifiers by the keysym of the key that holds each.
MODIFIERS = {"ctrl": "Control_L", "control": "Control_L", "alt": "Alt_L", "shift": "Shift_L",
             "win": "Super_L", "windows": "Super_L", "insert": "Insert"}


def chord(name):
    """A key name with its modifiers ("Insert+Down", "Shift+H", "Ctrl+Home") as the keysym names
    of the keys pressed, the modifiers first: Insert is held as one, though X has it as none."""
    parts = name.split("+")
    key = parts[-1].strip()
    mods = [MODIFIERS[m.strip().lower()] for m in parts[:-1] if m.strip().lower() in MODIFIERS]
    if re.fullmatch(r"[A-Za-z]", key):
        key = key.lower()
    elif re.fullmatch(r"[Ff]([1-9]|1[0-2])", key):
        key = key.upper()
    else:
        key = NAMES.get(key.lower(), key)
    return mods + [key]


def sendkeys(text):
    """SendKeys syntax ("^{HOME}", "{DOWN}", "+{TAB}") as key names."""
    keys = []
    mods = []
    for match in re.finditer(r"([\^+%]*)(\{[^}]+\}|.)", text):
        mods = [{"^": "Ctrl", "+": "Shift", "%": "Alt"}[m] for m in match.group(1)]
        key = match.group(2).strip("{}")
        keys.append("+".join(mods + [key.capitalize() if len(key) > 1 else key]))
    return keys


_DISPLAY = None


def press(name):
    """Presses the chord's keys in order and lets them go in reverse, by their keycodes through
    XTest. xdotool (3.20160805) pressed and let go Alt before each function key, whose press the
    window manager's Alt grabs then took: F5 never reached Okur (712's live run, 2026-11-05)."""
    from Xlib import X, XK, display
    from Xlib.ext import xtest
    global _DISPLAY
    if _DISPLAY is None:
        _DISPLAY = display.Display()
    codes = []
    for keysym_name in chord(name):
        code = _DISPLAY.keysym_to_keycode(XK.string_to_keysym(keysym_name))
        if not code:
            print(f"no keycode for {keysym_name} in {name}", file=sys.stderr)
            return
        codes.append(code)
    for code in codes:
        xtest.fake_input(_DISPLAY, X.KeyPress, code)
        _DISPLAY.sync()
    for code in reversed(codes):
        xtest.fake_input(_DISPLAY, X.KeyRelease, code)
        _DISPLAY.sync()


def said_since(log, offset):
    """The speak records written to the log after `offset`, and where it ends now."""
    try:
        with open(log, encoding="utf-8") as f:
            f.seek(offset)
            text = f.read()
            end = f.tell()
    except OSError:
        return [], offset
    said = []
    for line in text.splitlines():
        try:
            entry = json.loads(line)
        except ValueError:
            continue
        if entry.get("kind") == "speak":
            said.append(entry.get("text", ""))
    return said, end


def settle(log, offset, quiet=1.0, longest=10.0):
    """Waits until the log has been quiet for `quiet` seconds (at most `longest`). Shorter than
    the pause after which a character the arrows moved onto is spelled phonetically (1.5 s,
    JAWS's default, 1140, 1141): the speech log keeps the character again then, and with a 1.5 s
    quiet the spelling raced the next key ("s s" for "s", 07 and 107, 2026-11-06). JAWS's
    recordings end a step on audio silence, before it too."""
    started = time.time()
    last_size = -1
    last_change = time.time()
    while time.time() - started < longest:
        size = os.path.getsize(log) if os.path.exists(log) else 0
        if size != last_size:
            last_size = size
            last_change = time.time()
        elif time.time() - last_change >= quiet:
            return True
        time.sleep(0.1)
    return False


def main():
    scenario_path, out_path = sys.argv[1], sys.argv[2]
    with open(scenario_path, encoding="utf-8-sig") as f:
        scenario = json.load(f)
    fixture = scenario.get("fixture", "")
    page = "file://" + FIXTURES + fixture.rsplit("/fixtures/", 1)[-1]
    # A Windows program's fixture runs as its Linux replica.
    replica = None
    if fixture.endswith(".ps1"):
        name = fixture.rsplit("/", 1)[-1][:-len(".ps1")]
        replica = os.path.join(FIXTURES, "linux", name + ".py")
        if not os.path.exists(replica):
            print(f"{scenario['id']}: no Linux replica of {name}.ps1", file=sys.stderr)
            sys.exit(2)
    work = tempfile.mkdtemp()
    log = os.path.join(work, "speech.jsonl")
    profile = os.path.join(work, "chrome")
    # Okur's own output is kept: a run in which it said nothing shows why.
    okur_out = open(os.path.join(work, "okur.log"), "w", encoding="utf-8")
    level = recorded_web_verbosity(scenario)
    okur = subprocess.Popen([OKUR, "--no-speech", "3600", "--speech-log", log,
                             *(["--web-verbosity", level] if level else [])],
                            stdin=subprocess.DEVNULL, stdout=okur_out, stderr=subprocess.STDOUT,
                            env={**os.environ, "RUST_LOG": os.environ.get("RUST_LOG", "okur_linux=debug")})
    time.sleep(4)
    if replica:
        chrome = subprocess.Popen(["python3", replica], stdout=subprocess.DEVNULL,
                                  stderr=subprocess.DEVNULL)
        time.sleep(6)
    else:
        chrome = subprocess.Popen(["google-chrome", "--no-sandbox", "--no-first-run",
                                   "--disable-gpu", f"--user-data-dir={profile}",
                                   "--force-renderer-accessibility", page],
                                  stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        time.sleep(15)
    settle(log, 0, quiet=2.0, longest=30.0)
    # The desktop's applications as AT-SPI lists them, for a run in which Okur said nothing:
    # whether Chrome was on the bus at all.
    apps = ""
    if os.path.exists(XA11Y):
        apps = subprocess.run([XA11Y, "apps"], capture_output=True, text=True, timeout=30).stdout
    reset = scenario.get("reset", {})
    for key in reset.get("keys", []) + [k for s in reset.get("keystrokes", []) for k in sendkeys(s)]:
        press(key)
        time.sleep(0.3)
    settle(log, 0)
    _, offset = said_since(log, 0)
    script_keys = keys_for_scripts(scenario)
    results = []
    success = True
    for step in scenario.get("steps", []):
        kind = step.get("type")
        command = step.get("command", "")
        if kind == "key":
            keys = [command]
        elif kind == "keyboard":
            keys = sendkeys(command)
        elif kind == "jaws_script" and command in script_keys:
            # JAWS ran its script itself; Okur's user presses the key Okur binds to it.
            keys = [script_keys[command]]
        else:
            success = False
            results.append({"step_id": step["id"], "action": {"type": kind, "command": command},
                            "observation": {"speech_raw": "", "skipped": True}, "step_success": False})
            continue
        start = int(time.time() * 1000)
        for key in keys:
            press(key)
        settled = settle(log, offset)
        said, offset = said_since(log, offset)
        # A step Okur was no longer running for heard nothing: never a result.
        running = okur.poll() is None
        success = success and settled and running
        results.append({
            "step_id": step["id"],
            "t_start_ms": start,
            "t_end_ms": int(time.time() * 1000),
            "action": {"type": kind, "command": command},
            "observation": {"speech_raw": "\n".join(said), "settled": settled,
                            **({} if running else {"okur_exited": True})},
            "step_success": settled and running,
        })
    exited = okur.poll()
    if exited is not None:
        print(f"{scenario['id']}: Okur exited ({exited}) before the run ended ({work})",
              file=sys.stderr)
    okur.terminate()
    chrome.terminate()
    okur.wait(timeout=10)
    okur_out.close()
    # Silence from the first key to the last is Okur not running or not hearing, never a result.
    everything, _ = said_since(log, 0)
    if os.environ.get("OKUR_RUN_VERBOSE"):
        print(f"{scenario['id']}: Okur's speech log ({work}):", file=sys.stderr)
        with open(log, encoding="utf-8") as f:
            for line in f:
                print("  " + line.rstrip()[:300], file=sys.stderr)
        for step in results:
            print(f"  step {step['step_id']} {step.get('t_start_ms')}..{step.get('t_end_ms')}",
                  file=sys.stderr)
    if not everything:
        success = False
        with open(os.path.join(work, "okur.log"), encoding="utf-8", errors="replace") as f:
            # Without the grabs refused on windows already gone, which bury the rest.
            tail = [line for line in f.read().splitlines()
                    if "request failed" not in line and not line.startswith("  - ")][-25:]
        print(f"{scenario['id']}: Okur said nothing; its last lines ({work}):", file=sys.stderr)
        for line in tail:
            print("  " + re.sub(r"\x1b\[[0-9;]*m", "", line), file=sys.stderr)
        print(f"{scenario['id']}: AT-SPI's applications after the page loaded:", file=sys.stderr)
        for line in apps.splitlines():
            print("  " + line, file=sys.stderr)
    run = {
        "scenario_id": scenario["id"],
        "provenance": {"reader": "okur", "platform": "linux", "web_verbosity": level,
                       **({"application": "corpus/fixtures/linux/" + os.path.basename(replica)}
                          if replica else {"chrome": "google-chrome"}),
                       "commit": subprocess.run(["git", "rev-parse", "--short", "HEAD"],
                                                capture_output=True, text=True).stdout.strip()},
        "trials": [{"trial_index": 1, "trial_success": success, "step_results": results}],
    }
    with open(out_path, "w", encoding="utf-8") as f:
        json.dump(run, f, indent=1)
    print(f"{scenario['id']}: {len(results)} steps, success {success}")


if __name__ == "__main__":
    main()
