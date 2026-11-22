"""Agents' Spec on the whole launcher, over the cadence-spec daemon itself, on niri.

modules/quickshell/checks/spec.nix runs this through pytest inside tests/nested-niri.sh, with
Spec's place as the account's cadence module gives it, and cadence-spec serving what a
stand-in for cadence prints from a file here. On 18 November Spec was listed twice in Agents,
as its part and among Agents' own rows, and the cursor moved between the two as each read
came; Qt's own engine holds the parts' order (tests/ranking.qml), and this holds the whole
launcher to listing it once, the cursor staying on it as the daemon's answers change, a
step held in Spec taking the rows the daemon serves after a restart, and Spec kept while its
head says the same, so a - out of Now makes it again only once the head has changed.
"""

from __future__ import annotations

import contextlib
import json
import os
import shlex
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
    program.write_text(f"#!{bash}\nexec cat {shlex.quote(str(SERVED))}\n")
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


@contextlib.contextmanager
def agents_with_spec(shell: Shell) -> Iterator[None]:
    """Open Agents once it lists Spec, and close it after, so a case failing open fails alone."""
    shell.as_account(shell.ipc("collection", "agents"))
    try:
        shell.launcher_ready()
        shell.launcher_until(lambda shown: shown["held"] == ["agents"] and "Spec" in shown["rows"])
        yield
    finally:
        shell.close_launcher()


def test_agents_lists_spec_once_and_the_cursor_stays_on_it_as_its_head_changes(
    shell: Shell,
) -> None:
    """Spec is one row in Agents, and stays chosen while the daemon's answer changes.

    Its key listed twice moved the cursor between the two rows as each answer came. The
    daemon reads cadence at most once in five minutes, so a new answer here is its start
    again over a head rewritten, and the cursor is watched until the row says it.
    """
    with agents_with_spec(shell):
        rows = shell.showing()["rows"]
        assert rows.count("Spec") == 1, f"Spec isn't listed once: {rows}"
        shell.down_to("Spec")
        served = served_first()
        served["head"]["said"] = "2 readings out of spec"
        serve(served)
        shell.daemon_stopped("cadence-spec")
        spec_daemon(shell)
        deadline = time.monotonic() + 20
        while True:
            shown = shell.showing()
            selected = shown["selected"] or {}
            chosen = selected.get("name")
            assert chosen == "Spec", f"the cursor left Spec for {chosen}: {shown['rows']}"
            assert shown["rows"].count("Spec") == 1, f"Spec was listed again: {shown['rows']}"
            if selected.get("side") == "2 readings out of spec":
                break
            assert time.monotonic() < deadline, f"Spec never said the new head: {selected}"
            time.sleep(0.2)


def test_a_step_held_in_spec_takes_the_rows_a_restarted_daemon_serves(shell: Shell) -> None:
    """Held in Spec, its axes are those served, and become what the daemon serves after.

    cadence-spec reads cadence at most once in five minutes, so the next read here is its
    start again over rows rewritten; the step stays held through the gap, and its rows are
    replaced with what the daemon says once it answers again (modules/quickshell/Daemon.qml).
    """
    with agents_with_spec(shell):
        shell.down_to("Spec")
        shell.step_in()
        shell.launcher_until(
            lambda shown: (
                shown["held"] == ["agents", "Spec"] and shown["rows"] == ["Time to Done", "Quality"]
            )
        )
        rows = served_first()
        quality = next((row for row in rows["rows"] if row["key"] == "row:spec:quality"), None)
        assert quality is not None, f"the rows' stand-in serves no Quality: {rows['rows']}"
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


def in_now(shell: Shell) -> None:
    """Open Now by its chord, home read first, and wait until Agents makes no more rows."""
    shell.launcher_closed()
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shell.made_settled("agents")
    shell.close_launcher()
    shell.as_account(shell.ipc("collection", "on"))
    shell.launcher_ready()
    shell.launcher_until(lambda shown: shown["held"] == ["now"])
    shell.made_settled("agents")


def stepped_out_of_now(shell: Shell) -> list[str]:
    """Press - in Now, and give the keys of Agents' rows made as home is held and after."""
    before = shell.rows_made("agents")
    shell.normal()
    shell.send_chars("-")
    shell.launcher_until(lambda shown: not shown["held"])
    shell.made_settled("agents")
    return shell.made_since("agents", before)


def test_a_step_out_of_now_makes_no_spec_while_its_head_says_the_same(shell: Shell) -> None:
    """- out of Now makes Agents' Spec again only if cadence-spec's head has changed.

    Made anew each time Agents' rows were read, Spec was made in every - out of Now as home was
    held, though nothing it says had changed, where the rest of Agents' are kept (core.nix#6037).
    """
    in_now(shell)
    try:
        made = stepped_out_of_now(shell)
    finally:
        shell.close_launcher()
    assert "agents:spec" not in made, f"- out of Now made Spec again: {made}"


def test_spec_says_a_head_that_changed_while_now_was_held(shell: Shell) -> None:
    """A head that changes while Now is held is the one Spec says once Agents is read again.

    Kept while what it says is the same, Spec is made again once the head changes, rather than
    saying the one it was made of while nothing read it.
    """
    in_now(shell)
    try:
        served = served_first()
        served["head"]["said"] = "3 readings out of spec"
        serve(served)
        shell.daemon_stopped("cadence-spec")
        spec_daemon(shell)
        stepped_out_of_now(shell)
    finally:
        shell.close_launcher()
    with agents_with_spec(shell):
        shell.down_to("Spec")
        shell.launcher_until(
            lambda shown: (shown["selected"] or {}).get("side") == "3 readings out of spec"
        )
