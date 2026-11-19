"""Agents' Spec on the whole launcher, over the cadence-spec daemon itself, on niri.

modules/quickshell/checks/spec.nix runs this through pytest inside tests/nested-niri.sh, with
Spec's place as the account's cadence module gives it, and cadence-spec serving what a
stand-in for cadence prints from a file here. On 18 November Spec was listed twice in Agents,
as its part and among Agents' own rows, and the cursor moved between the two as each read
came; Qt's own engine holds the parts' order (tests/ranking.qml), and this holds the whole
launcher to listing it once, the cursor staying on it, and a step held in Spec taking the
rows its daemon serves next.
"""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import time
from pathlib import Path
from typing import TYPE_CHECKING

import launcher_on_niri
import pytest

if TYPE_CHECKING:
    from collections.abc import Iterator

    from launcher_on_niri import Json, Shell

LOGS = Path(os.environ["LOGS"])
# What the stand-in for cadence prints, rewritten by a case to serve other rows.
SERVED = LOGS.resolve() / "served.json"


def serve(rows: Json) -> None:
    """Have the stand-in for cadence print these rows from its next read."""
    SERVED.write_text(json.dumps(rows))


def served_first() -> Json:
    """Give the rows the launcher's Spec check serves: two axes, one out of spec."""
    bash = shutil.which("bash")
    assert bash is not None, "no bash to run the rows' stand-in"
    printed = subprocess.run(
        [bash, os.environ["SPEC_ROWS"]], capture_output=True, check=True, text=True
    ).stdout
    rows: Json = json.loads(printed)
    return rows


def cadence() -> Path:
    """Write a stand-in for cadence printing what SERVED holds, and give its path."""
    bash = shutil.which("bash")
    assert bash is not None, "no bash to stand in for cadence"
    program = LOGS.resolve() / "cadence"
    program.write_text(f"#!{bash}\nexec cat {SERVED}\n")
    program.chmod(0o755)
    return program


def spec_daemon(shell: Shell) -> None:
    """Start cadence-spec on its socket, as its unit runs it, over the stand-in."""
    shell.daemon("cadence-spec", os.environ["CADENCE_SPEC"], str(cadence()), os.environ["WORDS"])


@pytest.fixture(scope="module")
def shell() -> Iterator[Shell]:
    """Give the shell, started once a run with cadence-spec serving two axes."""
    LOGS.mkdir(parents=True, exist_ok=True)
    serve(served_first())
    started = launcher_on_niri.started()
    spec_daemon(started)
    yield started
    started.stop()


@pytest.fixture(autouse=True)
def launcher_closed_after(shell: Shell) -> Iterator[None]:
    """Close the launcher after each test, so one failing with it open fails alone."""
    yield
    shell.close_launcher()


def agents_with_spec(shell: Shell) -> None:
    """Open Agents, once it lists Spec from the daemon's head."""
    shell.as_account(shell.ipc("collection", "agents"))
    shell.launcher_ready()
    shell.launcher_until(lambda shown: shown["held"] == ["agents"] and "Spec" in shown["rows"])


def test_agents_lists_spec_once_after_budgets_and_the_cursor_stays_on_it(shell: Shell) -> None:
    """Spec is one row in Agents, after Budgets where Budgets is, and stays chosen as reads come.

    Its key listed twice moved the cursor between the two rows as each read came, so the
    cursor is watched on it for longer than the daemon takes to answer a watch again.
    """
    agents_with_spec(shell)
    rows = shell.showing()["rows"]
    assert rows.count("Spec") == 1, f"Spec isn't listed once: {rows}"
    if "Budgets" in rows:
        assert rows.index("Spec") == rows.index("Budgets") + 1, f"Spec isn't after Budgets: {rows}"
    shell.down_to("Spec")
    deadline = time.monotonic() + 3
    while time.monotonic() < deadline:
        shown = shell.showing()
        chosen = (shown["selected"] or {}).get("name")
        assert chosen == "Spec", f"the cursor left Spec for {chosen}: {shown['rows']}"
        assert shown["rows"].count("Spec") == 1, f"Spec was listed again: {shown['rows']}"
        time.sleep(0.2)


def test_a_step_held_in_spec_takes_the_rows_its_daemon_serves_next(shell: Shell) -> None:
    """Held in Spec, its axes are those served, and become what the daemon serves after.

    cadence-spec reads cadence at most once in five minutes, so the next read here is its
    start again over rows rewritten, which the shell's connection holds through the gap and
    then replaces with what the daemon says (modules/quickshell/Daemon.qml).
    """
    agents_with_spec(shell)
    shell.down_to("Spec")
    shell.step_in()
    shell.launcher_until(
        lambda shown: (
            shown["held"] == ["agents", "Spec"] and shown["rows"] == ["Time to Done", "Quality"]
        )
    )
    rows = served_first()
    quality = next(row for row in rows["rows"] if row["key"] == "row:spec:quality")
    quality["name"] = "Quality Held Up"
    serve(rows)
    shell.daemon_stopped("cadence-spec")
    spec_daemon(shell)
    shell.launcher_until(
        lambda shown: (
            shown["held"] == ["agents", "Spec"]
            and shown["rows"] == ["Time to Done", "Quality Held Up"]
        )
    )
