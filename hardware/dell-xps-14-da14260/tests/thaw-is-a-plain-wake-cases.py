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


# A failure names every test its reset sits under, outermost first, and a later kernel's
# wake may return early on an error before where a case puts its reset, which adds that
# return's guard before them. So a case expects the tests it adds itself, wherever they
# stand in the failure, and one adding none expects what a reset a thaw reaches fails
# saying, whether it sits under no test ("on every wake") or that guard ("when not ...").
REACHED = "Bluetooth's wake resets its chip "
NO_RESTORE = "no longer resets its chip for a restore"
# What the reader can't follow, it refuses, naming it.
CANNOT = "Bluetooth's wake now has a"
# Names that ask after a reset, by each word of asking, and names that reset the chip
# with reset glued into a word, or a word of asking only saying how.
ASKING = (
    "btintel_pcie_cancel_reset",
    "btintel_pcie_reset_cancelled",
    "btintel_pcie_reset_done",
    "btintel_pcie_has_reset",
    "btintel_pcie_in_reset",
    "btintel_pcie_is_reset",
    "btintel_pcie_reset_status",
    "btintel_pcie_was_reset",
)
RESETTING = (
    "btintel_pcie_softreset",
    "btintel_pcie_hardreset",
    "btintel_pcie_hwreset",
    "btintel_pcie_btreset",
    "btintel_pcie_reset_in_band",
)
# Calls that hand the reset work on, and so queue it, and calls that only cancel it,
# flush it, stop it or ask after it.
QUEUED = (
    ("queue_work_node", "0, system_wq, &data->reset_work"),
    ("enable_and_queue_work", "system_wq, &data->reset_work"),
    ("kthread_queue_work", "data->worker, &data->reset_work"),
    ("btintel_pcie_kick", "&data->reset_work"),
)
CALM = (
    "cancel_delayed_work_sync",
    "flush_delayed_work",
    "disable_work_sync",
    "delayed_work_pending",
    "work_busy",
)


def switching(cases: str) -> str:
    """Give a switch on the wake's event holding the cases given, as lines at its start."""
    return f"\tswitch (mesg.event) {{\n{cases}\tdefault:\n\t\tbreak;\n\t}}"


# Each case: its name, the pattern and replacement that make it, and the words its
# failure must hold, or None where it must pass.
CASES: list[tuple[str, tuple[str, str], str | None]] = [
    (
        "a negated test",
        (WAKE_TEST, "if (mesg.event != PM_EVENT_SUSPEND) {"),
        "mesg.event != PM_EVENT_SUSPEND",
    ),
    ("an unguarded reset", at_start(f"\t{RESET}"), REACHED),
    ("a reset of another name", at_start("\tbtintel_pcie_reset_bt(data);"), REACHED),
    ("a reset whose name starts with it", at_start("\treset_chip(data);"), REACHED),
    ("a function-level reset by its name alone", at_start("\tflr(data);"), REACHED),
    (
        "a reset inside a test",
        at_start("\tif (btintel_pcie_reset_bt(data))\n\t\treturn 0;"),
        REACHED,
    ),
    ("the reset work queued", at_start("\tschedule_work(&data->reset_work);"), REACHED),
    (
        "the reset work queued for later",
        at_start("\tqueue_delayed_work(system_wq, &data->reset_work, 0);"),
        REACHED,
    ),
    (
        "the reset work queued on a processor",
        at_start("\tschedule_work_on(0, &data->reset_work);"),
        REACHED,
    ),
    *(
        (f"the reset work handed to {name}", at_start(f"\t{name}({handed});"), REACHED)
        for name, handed in QUEUED
    ),
    *(
        (
            f"the reset work in {name}",
            at_start(f"\tif ({name}(&data->reset_work))\n\t\treturn 0;"),
            None,
        )
        for name in CALM
    ),
    ("the reset work cancelled", at_start("\tcancel_work_sync(&data->reset_work);"), None),
    ("the reset work flushed", at_start("\tflush_work(&data->reset_work);"), None),
    (
        "the reset work asked after in a test",
        at_start("\tif (work_pending(&data->reset_work))\n\t\treturn -EBUSY;"),
        None,
    ),
    (
        "a pending reset asked after in a test",
        at_start("\tif (btintel_pcie_reset_pending(data))\n\t\treturn -EBUSY;"),
        None,
    ),
    *(
        (
            f"a reset asked after by {name}",
            at_start(f"\tif ({name}(data))\n\t\treturn -EBUSY;"),
            None,
        )
        for name in ASKING
    ),
    *((f"a reset named {name}", at_start(f"\t{name}(data);"), REACHED) for name in RESETTING),
    ("a packet's headers set back", at_start("\tskb_reset_tail_pointer(data->skb);"), None),
    (
        "a reset named in a log",
        at_start('\tbt_dev_dbg(data->hdev, "no btintel_pcie_reset_bt(data); {");'),
        None,
    ),
    (
        "a return named in a log",
        at_start(
            f'\tif (mesg.event == PM_EVENT_THAW)\n\t\tbt_dev_dbg(data->hdev, "return");\n\t{RESET}'
        ),
        REACHED,
    ),
    (
        "a reset in a macro",
        at_start("#define btintel_pcie_wake_reset(d) \\\n\tbtintel_pcie_reset_bt(d)"),
        None,
    ),
    (
        "a comment naming a reset, a return and a brace",
        at_start("\t/* no btintel_pcie_reset_bt(data) here; return 0; { */"),
        None,
    ),
    # The block it is the else of doesn't return, so only the else says what holds.
    (
        "a reset in an else",
        (FIRST_RESET, f"\t\t{RESET}\n\t}} else {{\n\t\t{RESET}\n\t}}"),
        "not ((mesg.event == PM_EVENT_RESTORE ||",
    ),
    (
        "a reset in the last else of a chain",
        at_start(
            "\tif (mesg.event == PM_EVENT_RESTORE)\n\t\tdata->kept = 1;\n"
            "\telse if (mesg.event == PM_EVENT_RESUME)\n\t\tdata->kept = 2;\n"
            f"\telse\n\t\t{RESET}"
        ),
        "not ((mesg.event == PM_EVENT_RESTORE)) and not ((mesg.event == PM_EVENT_RESUME))",
    ),
    (
        "a reset in the last else of a chain past a thaw",
        at_start(
            "\tif (mesg.event == PM_EVENT_THAW) {\n\t\tdata->kept = 1;\n"
            "\t} else if (data->kept) {\n\t\tdata->kept = 2;\n"
            f"\t}} else {{\n\t\t{RESET}\n\t}}"
        ),
        None,
    ),
    (
        "a second reset, braced",
        at_start(f"\tif (data->pm_sx_event == PM_EVENT_FREEZE) {{\n\t\t{RESET}\n\t}}"),
        "data->pm_sx_event == PM_EVENT_FREEZE",
    ),
    (
        "a second reset, braceless",
        at_start(f"\tif (data->pm_sx_event == PM_EVENT_FREEZE)\n\t\t{RESET}"),
        "data->pm_sx_event == PM_EVENT_FREEZE",
    ),
    (
        "a braceless if over a loop's block",
        at_start(
            "\tif (data->pm_sx_event == PM_EVENT_FREEZE)\n"
            f"\t\twhile (data->alive) {{\n\t\t\tdata->alive = 0;\n\t\t\t{RESET}\n\t\t}}"
        ),
        "data->pm_sx_event == PM_EVENT_FREEZE",
    ),
    (
        "an or with a part a thaw meets",
        at_start(f"\tif (mesg.event == PM_EVENT_RESTORE || data->kept)\n\t\t{RESET}"),
        "mesg.event == PM_EVENT_RESTORE || data->kept",
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
        NO_RESTORE,
    ),
    # The early return's test names a restore, and holds back a thaw, but says only that
    # no restore gets past it to the reset under the restore's own test.
    (
        "a restore turned away before its reset",
        at_start(
            "\tif (mesg.event == PM_EVENT_THAW || mesg.event == PM_EVENT_RESTORE)\n\t\treturn 0;"
        ),
        NO_RESTORE,
    ),
    (
        "an and with a part a thaw can't meet",
        at_start(f"\tif (data->kept && mesg.event == PM_EVENT_RESTORE)\n\t\t{RESET}"),
        None,
    ),
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
    (
        "a reset past an early return for a thaw in a block of its own",
        at_start(
            "\tif (mesg.event == PM_EVENT_THAW) {\n\t\t{\n\t\t\treturn 0;\n\t\t}\n\t}\n"
            f"\t{RESET}"
        ),
        None,
    ),
    (
        "a reset past a braceless early return a thaw can meet",
        at_start(f"\tif (data->kept)\n\t\treturn 0;\n\t{RESET}"),
        "not ((data->kept))",
    ),
    (
        "a reset past a braced early return a thaw can meet",
        at_start(f"\tif (data->kept) {{\n\t\treturn 0;\n\t}}\n\t{RESET}"),
        "not ((data->kept))",
    ),
    (
        "a reset past a return only a loop reaches",
        at_start(
            "\tif (mesg.event == PM_EVENT_THAW)\n\t\twhile (data->busy)\n"
            f"\t\t\treturn -EBUSY;\n\t{RESET}"
        ),
        "not ((mesg.event == PM_EVENT_THAW) && ((a loop)))",
    ),
    (
        "a reset past a return only one case of a switch reaches",
        at_start(
            "\tif (mesg.event == PM_EVENT_THAW)\n\t\tswitch (data->state) {\n"
            "\t\tcase 1:\n\t\t\treturn 0;\n\t\tdefault:\n\t\t\tbreak;\n"
            f"\t\t}}\n\t{RESET}"
        ),
        REACHED,
    ),
    (
        "a reset under a case a thaw meets",
        at_start(switching(f"\tcase PM_EVENT_FREEZE:\n\t\t{RESET}\n\t\tbreak;\n")),
        "mesg.event == PM_EVENT_FREEZE,",
    ),
    (
        "a reset under a restore's case",
        at_start(switching(f"\tcase PM_EVENT_RESTORE:\n\t\t{RESET}\n\t\treturn 0;\n")),
        None,
    ),
    (
        "a thaw's case falling through to a reset",
        at_start(
            switching(
                "\tcase PM_EVENT_THAW:\n\t\tdata->kept = 1;\n\t\tfallthrough;\n"
                f"\tcase PM_EVENT_RESTORE:\n\t\t{RESET}\n\t\treturn 0;\n"
            )
        ),
        "mesg.event == PM_EVENT_THAW || mesg.event == PM_EVENT_RESTORE",
    ),
    (
        "a case jumped to past another case's early return for a thaw",
        at_start(
            "\tswitch (data->state) {\n\tcase 1:\n\t\tif (mesg.event == PM_EVENT_THAW)\n"
            f"\t\t\treturn 0;\n\t\tfallthrough;\n\tcase 2:\n\t\t{RESET}\n\t\tbreak;\n\t}}"
        ),
        "data->state == 1 || data->state == 2,",
    ),
    (
        "a reset under a switch's default",
        at_start(
            "\tswitch (mesg.event) {\n\tcase PM_EVENT_RESTORE:\n\t\treturn 0;\n"
            f"\tdefault:\n\t\t{RESET}\n\t\tbreak;\n\t}}"
        ),
        "(default)",
    ),
    # A jump past a thaw's early return reaches the reset after it.
    (
        "a goto past a thaw's early return",
        at_start(
            "\tif (data->kept)\n\t\tgoto reset;\n\tif (mesg.event == PM_EVENT_THAW)\n"
            f"\t\treturn 0;\nreset:\n\t{RESET}"
        ),
        f"{CANNOT} goto",
    ),
    (
        "a label past a thaw's early return",
        at_start(f"\tif (mesg.event == PM_EVENT_THAW)\n\t\treturn 0;\nreset:\n\t{RESET}"),
        f"{CANNOT} label, reset,",
    ),
    (
        "a question's colon",
        at_start("\tdata->kept = data->kept ? data->kept : 0;"),
        None,
    ),
    # A kernel built without the option resets every thaw.
    (
        "a guard only some kernels build",
        at_start(
            "#ifdef CONFIG_PM_SLEEP\n\tif (mesg.event == PM_EVENT_THAW)\n\t\treturn 0;\n"
            f"#endif\n\t{RESET}"
        ),
        f"{CANNOT} preprocessor conditional, #ifdef,",
    ),
    (
        "a reset only some kernels build",
        at_start(f"#if IS_ENABLED(CONFIG_PM_SLEEP)\n\tdata->kept = 1;\n#else\n\t{RESET}\n#endif"),
        f"{CANNOT} preprocessor conditional, #if,",
    ),
    (
        "a return only a loop's macro reaches",
        at_start(
            "\tif (mesg.event == PM_EVENT_THAW)\n"
            "\t\tlist_for_each_entry(entry, &data->list, node) {\n\t\t\treturn 0;\n\t\t}\n"
            f"\t{RESET}"
        ),
        "not ((mesg.event == PM_EVENT_THAW) && ((a loop)))",
    ),
    (
        "a return only a braceless loop's macro reaches",
        at_start(
            "\tif (mesg.event == PM_EVENT_THAW)\n"
            f"\t\tfor_each_set_bit(bit, &data->flags, 8)\n\t\t\treturn 0;\n\t{RESET}"
        ),
        "not ((mesg.event == PM_EVENT_THAW) && ((a loop)))",
    ),
    # Each is the wake's only restore reset.
    (
        "a restore reset in the else of a thaw",
        (WAKE_TEST, "if (mesg.event == PM_EVENT_THAW) {\n\t\tdata->kept = 1;\n\t} else {"),
        None,
    ),
    (
        "a restore reset past a thaw's early return",
        (WAKE_TEST, "if (mesg.event == PM_EVENT_THAW)\n\t\treturn 0;\n\t{"),
        None,
    ),
    (
        "a restore reset past a thaw's early return under a test it can't say of a restore",
        (WAKE_TEST, "if (mesg.event == PM_EVENT_THAW)\n\t\treturn 0;\n\tif (data->kept) {"),
        NO_RESTORE,
    ),
    (
        "a wake switched on its event, a thaw's case returning",
        at_start(switching("\tcase PM_EVENT_THAW:\n\t\treturn 0;\n") + f"\n\t{RESET}"),
        None,
    ),
    (
        "a wake switched on its event, a thaw's case falling through to a return",
        at_start(
            switching(
                "\tcase PM_EVENT_THAW:\n\t\tdata->kept = 1;\n\t\tfallthrough;\n"
                "\tcase PM_EVENT_FREEZE:\n\t\treturn 0;\n"
            )
            + f"\n\t{RESET}"
        ),
        None,
    ),
    (
        "a wake switched on its event, a thaw's case breaking before falling through to a return",
        at_start(
            switching(
                "\tcase PM_EVENT_THAW:\n\t\tif (data->kept)\n\t\t\tbreak;\n\t\tfallthrough;\n"
                "\tcase PM_EVENT_FREEZE:\n\t\treturn 0;\n"
            )
            + f"\n\t{RESET}"
        ),
        REACHED,
    ),
    (
        "a wake switched on its event, a thaw's case breaking before its return",
        at_start(
            switching(
                "\tcase PM_EVENT_THAW:\n\t\tif (data->kept) {\n\t\t\tbreak;\n\t\t}\n\t\treturn 0;\n"
            )
            + f"\n\t{RESET}"
        ),
        REACHED,
    ),
    (
        "a wake switched on another value, its thaw's case returning",
        at_start(
            "\tswitch (data->state) {\n\tcase PM_EVENT_THAW:\n\t\treturn 0;\n"
            f"\tdefault:\n\t\tbreak;\n\t}}\n\t{RESET}"
        ),
        REACHED,
    ),
    # Unless a return or a break ends the cases before it, the restore's case is reached
    # by the case above it, which a thaw meets.
    (
        "a case past a thaw's case that returned",
        at_start(
            switching(
                "\tcase PM_EVENT_THAW:\n\t\treturn 0;\n"
                f"\tcase PM_EVENT_RESTORE:\n\t\t{RESET}\n\t\tbreak;\n"
            )
        ),
        None,
    ),
    (
        "a case past a freeze's case that broke",
        at_start(
            switching(
                "\tcase PM_EVENT_FREEZE:\n\t\tdata->kept = 1;\n\t\tbreak;\n"
                f"\tcase PM_EVENT_RESTORE:\n\t\t{RESET}\n\t\tbreak;\n"
            )
        ),
        None,
    ),
    (
        "a case past a freeze's case that broke in its braces",
        at_start(
            switching(
                "\tcase PM_EVENT_FREEZE: {\n\t\tdata->kept = 1;\n\t\tbreak;\n\t}\n"
                f"\tcase PM_EVENT_RESTORE:\n\t\t{RESET}\n\t\tbreak;\n"
            )
        ),
        None,
    ),
    (
        "a case past a loop's break in a freeze's case",
        at_start(
            switching(
                "\tcase PM_EVENT_FREEZE:\n\t\twhile (data->busy) {\n\t\t\tbreak;\n\t\t}\n"
                f"\tcase PM_EVENT_RESTORE:\n\t\t{RESET}\n\t\tbreak;\n"
            )
        ),
        "mesg.event == PM_EVENT_FREEZE || mesg.event == PM_EVENT_RESTORE",
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
