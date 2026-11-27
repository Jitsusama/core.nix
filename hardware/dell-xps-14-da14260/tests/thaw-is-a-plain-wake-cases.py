"""Prove the thaw check's reader on edited copies of the driver it reads.

    thaw-is-a-plain-wake-cases CHECK SOURCE

CHECK is thaw-is-a-plain-wake.py and SOURCE the driver as the kernel builds it. Each case
edits the source the way a later kernel might, and says what the check must answer: a
pass, or a failure whose message holds the words given. A case whose edit no longer
changes the source fails by its name, so a moved driver can't leave a case proving
nothing. The real source passes on its own in the check this runs beside.
"""

import importlib.util
import re
import sys
import tempfile
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from types import ModuleType

WAKE_START = r"(\tstart = ktime_get\(\);\n)(\n\t/\* S3 and S4)"
WAKE_TEST = (
    r"if \(mesg\.event == PM_EVENT_RESTORE \|\|\n"
    r"\t    \(mesg\.event == PM_EVENT_RESUME &&\n"
    r"\t     pm_suspend_target_state == PM_SUSPEND_MEM\)\) \{"
)
RESET = "btintel_pcie_request_reset(data, BTINTEL_PCIE_IOSF_PRR_FLR);"
FIRST_RESET = rf"(\t\t{re.escape(RESET)}\n\t\treturn 0;\n\t\}})"
THAW = r"(static int btintel_pcie_thaw\(struct device \*dev\)\n\{\n)"


def at_start(lines: str) -> tuple[str, str]:
    """Give an edit putting lines at the wake's start, before its first test."""
    return WAKE_START, r"\1\n" + lines.replace("\\", "\\\\") + r"\n\2"


# Each case: its name, the pattern and replacement that make it, and the words its
# failure must hold, or None where it must pass.
CASES: list[tuple[str, tuple[str, str], str | None]] = [
    (
        "a negated test",
        (WAKE_TEST, "if (mesg.event != PM_EVENT_SUSPEND) {"),
        "when mesg.event != PM_EVENT_SUSPEND",
    ),
    ("an unguarded reset", at_start(f"\t{RESET}"), "on every wake"),
    (
        "a reset of another name",
        at_start("\tbtintel_pcie_reset_bt(data);"),
        "on every wake",
    ),
    (
        "a reset inside a test",
        at_start("\tif (btintel_pcie_reset_bt(data))\n\t\treturn 0;"),
        "on every wake",
    ),
    ("the reset work queued", at_start("\tschedule_work(&data->reset_work);"), "on every wake"),
    # The block it is the else of doesn't return, so only the else says what holds.
    (
        "a reset in an else",
        (FIRST_RESET, f"\t\t{RESET}\n\t}} else {{\n\t\t{RESET}\n\t}}"),
        "when not (",
    ),
    (
        "a second reset, braced",
        at_start(f"\tif (data->pm_sx_event == PM_EVENT_FREEZE) {{\n\t\t{RESET}\n\t}}"),
        "when data->pm_sx_event == PM_EVENT_FREEZE",
    ),
    (
        "a second reset, braceless",
        at_start(f"\tif (data->pm_sx_event == PM_EVENT_FREEZE)\n\t\t{RESET}"),
        "when data->pm_sx_event == PM_EVENT_FREEZE",
    ),
    (
        "a braceless if over a loop's block",
        at_start(
            "\tif (data->pm_sx_event == PM_EVENT_FREEZE)\n"
            f"\t\twhile (data->alive) {{\n\t\t\tdata->alive = 0;\n\t\t\t{RESET}\n\t\t}}"
        ),
        "when data->pm_sx_event == PM_EVENT_FREEZE",
    ),
    (
        "an or with a part a thaw meets",
        at_start(f"\tif (mesg.event == PM_EVENT_RESTORE || data->kept)\n\t\t{RESET}"),
        "when mesg.event == PM_EVENT_RESTORE || data->kept",
    ),
    (
        "the thaw resetting itself",
        (THAW, r"\1\tbtintel_pcie_reset_bt(pci_get_drvdata(to_pci_dev(dev)));\n"),
        "thaw resets its chip itself",
    ),
    (
        "a restore no longer reset",
        (
            WAKE_TEST,
            (
                "if ((mesg.event == PM_EVENT_RESUME &&\n\t     "
                "pm_suspend_target_state == PM_SUSPEND_MEM)) {"
            ),
        ),
        "no longer resets its chip for a restore",
    ),
    (
        "an and with a part a thaw can't meet",
        at_start(f"\tif (data->kept && mesg.event == PM_EVENT_RESTORE)\n\t\t{RESET}"),
        None,
    ),
    ("a comment naming the thaw", at_start("\t/* PM_EVENT_THAW takes no reset. */"), None),
    (
        "an early return for a thaw",
        at_start("\tif (mesg.event == PM_EVENT_THAW)\n\t\treturn 0;"),
        None,
    ),
    (
        "a reset past an early return for a thaw",
        at_start(f"\tif (mesg.event == PM_EVENT_THAW)\n\t\treturn 0;\n\t{RESET}"),
        None,
    ),
]


def loaded(check: Path) -> ModuleType:
    """Load the check as a module, so its verdict is read without a process each."""
    spec = importlib.util.spec_from_file_location("thaw_check", check)
    if spec is None or spec.loader is None:
        sys.exit(f"{check} can't be loaded.")
    module = importlib.util.module_from_spec(spec)
    # Its dataclasses look their module up by name.
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def verdict(module: ModuleType, source: str) -> str | None:
    """Give the check's failure for a source, or None when it passes."""
    with tempfile.NamedTemporaryFile("w", suffix=".c") as copy:
        copy.write(source)
        copy.flush()
        try:
            module.main([copy.name])
        except SystemExit as failure:
            return str(failure.code)
    return None


def wrong(module: ModuleType, source: str) -> list[str]:
    """Give a line for each case the check answers wrongly."""
    lines = []
    for name, (pattern, replacement), expected in CASES:
        edited = re.sub(pattern, replacement, source, count=1)
        if edited == source:
            lines.append(f"{name}: its edit no longer changes the driver; move it with the driver.")
            continue
        said = verdict(module, edited)
        if expected is None and said is not None:
            lines.append(f"{name}: should pass, and failed: {said}")
        elif expected is not None and (said is None or expected not in said):
            lines.append(f"{name}: should fail saying {expected!r}, and said: {said}")
    return lines


def main(argv: list[str]) -> None:
    """Fail, naming each case, unless the check answers every case rightly."""
    module = loaded(Path(argv[0]))
    lines = wrong(module, Path(argv[1]).read_text())
    if lines:
        sys.exit("The thaw check answers these edits of the driver wrongly:\n" + "\n".join(lines))


if __name__ == "__main__":
    main(sys.argv[1:])
