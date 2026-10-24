"""Show the desktop's shell from the working copy, in a niri window of its own.

    shell-preview [--type TEXT] [--check NAME] [--account NAME]

It builds the shell's home from a check's machine, vm-desktop-works by default, so the
launcher lists that machine's stand-in programs rather than the session's own. Its Quickshell
folder is laid out afresh: each file that is a tracked file of the working copy is linked to
it, so an edit shows as Quickshell reloads, and each the build made, the theme among them, is
copied. niri then starts nested in the session's compositor, running that Quickshell with a
runtime folder, a D-Bus session and state of its own, so nothing reaches the session's own
shell. With --type, the launcher opens and the text is typed into it, to see the rows a query
gives. Closing the window ends it. A change to what Nix generates needs a new preview.
"""

import argparse
import hashlib
import json
import os
import subprocess
import sys
import tempfile
import time
from dataclasses import dataclass
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Mapping, Sequence

# dbus-run-session and wtype, which tests/shell-preview.nix puts in place of the markers.
DBUS_RUN_SESSION = "@dbus_run_session@"
WTYPE = "@wtype@"
# How long the nested shell may take to answer, in seconds: it starts cold, drawn in software
# when the GPU is the session's.
STARTING = 60
# The nested D-Bus session's config: a bus anyone in the account may use, with no services
# to start, so no portal starts and mounts its documents in the runtime folder, which the
# preview then couldn't take away. The shell serves what it needs itself.
BUS = """<!DOCTYPE busconfig PUBLIC "-//freedesktop//DTD D-Bus Bus Configuration 1.0//EN"
 "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
<busconfig>
  <type>session</type>
  <listen>unix:dir={runtime}</listen>
  <policy context="default">
    <allow send_destination="*" eavesdrop="true"/>
    <allow eavesdrop="true"/>
    <allow own="*"/>
  </policy>
</busconfig>
"""


@dataclass(frozen=True)
class Machine:
    """What a preview takes from a check's machine, each a path in the store."""

    home: Path
    niri: Path
    quickshell: Path
    data: list[Path]


def run(*command: str) -> str:
    """Run a program on the path, and give what it printed."""
    # git and nix are the person's own, with their settings, so they're found on the path.
    return subprocess.run(command, check=True, stdout=subprocess.PIPE, text=True).stdout.strip()  # noqa: S603


def digest(path: Path) -> str:
    """Give a file's SHA-256, which says it's the same file wherever it's kept."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def sources(root: Path) -> dict[str, Path]:
    """Give the working copy's QML and JavaScript, each by its digest."""
    tracked = run("git", "-C", str(root), "ls-files", "-z", "*.qml", "*.js").split("\0")
    return {digest(root / name): root / name for name in tracked if name}


def lay_out(built: Path, known: Mapping[str, Path], into: Path) -> tuple[int, int]:
    """Lay the built folder out in into, a tracked file as a link to it and any other copied.

    Home Manager copies a module's file as it is, so a built file is the working copy's
    file with the same bytes. Gives how many were linked and how many copied.
    """
    linked = copied = 0
    for entry in sorted(built.rglob("*")):
        if not entry.is_file():
            continue
        target = into / entry.relative_to(built)
        target.parent.mkdir(parents=True, exist_ok=True)
        source = known.get(digest(entry))
        if source is None:
            target.write_bytes(entry.read_bytes())
            copied += 1
        else:
            target.symlink_to(source)
            linked += 1
    return linked, copied


def niri_config(built: Path, into: Path, quickshell: Path, shell: Path) -> Path:
    """Copy niri's config into into, starting the shell laid out at shell, and give its path."""
    into.mkdir(parents=True, exist_ok=True)
    for entry in built.iterdir():
        if entry.is_file():
            (into / entry.name).write_text(entry.read_text())
    config = into / "config.kdl"
    # A KDL string is quoted and escaped as a JSON one is.
    start = " ".join(json.dumps(str(word)) for word in [quickshell, "-p", shell])
    config.write_text(f"{config.read_text()}\nspawn-at-startup {start}\n")
    return config


def machine(flake: str, system: str, check: str, account: str) -> Machine:
    """Build what a preview takes from the check's machine."""
    found = json.loads(
        run(
            "nix",
            "eval",
            "--json",
            f"{flake}#checks.{system}.{check}.nodes.machine",
            "--apply",
            "m: let home = m.home-manager.users." + account + "; in {"
            " home = home.home-files.outPath; niri = m.programs.niri.package.outPath;"
            " quickshell = builtins.head home.systemd.user.services.quickshell.Service.ExecStart;"
            " data = [ m.system.path.outPath home.home.path.outPath ]; }",
        )
    )
    attribute = f"{flake}#checks.{system}.{check}.nodes.machine"
    run(
        "nix",
        "build",
        "--no-link",
        f"{attribute}.home-manager.users.{account}.home-files",
        f"{attribute}.home-manager.users.{account}.home.path",
        f"{attribute}.programs.niri.package",
        f"{attribute}.system.path",
    )
    return Machine(
        home=Path(found["home"]),
        niri=Path(found["niri"]),
        quickshell=Path(found["quickshell"]),
        data=[Path(path) / "share" for path in found["data"]],
    )


def nested(own: Path, data: Sequence[Path]) -> dict[str, str]:
    """Give the nested niri's environment: the session's display, and all else its own."""
    runtime = own / "runtime"
    runtime.mkdir(mode=0o700)
    parent = Path(os.environ["XDG_RUNTIME_DIR"]) / os.environ["WAYLAND_DISPLAY"]
    return {
        **os.environ,
        "XDG_RUNTIME_DIR": str(runtime),
        "WAYLAND_DISPLAY": str(parent),
        "XDG_STATE_HOME": str(own / "state"),
        "XDG_CACHE_HOME": str(own / "cache"),
        # Where pi's quests and the like are kept, so the preview shows none of the
        # account's own and the same rows each time.
        "XDG_DATA_HOME": str(own / "data"),
        "XDG_DATA_DIRS": ":".join(str(path) for path in data),
    }


def inside(environment: Mapping[str, str]) -> dict[str, str]:
    """Give the environment a program inside the nested niri has, on its display.

    The shell's IPC finds a shell by the display it runs on, so it's asked from there.
    """
    displays = sorted(Path(environment["XDG_RUNTIME_DIR"]).glob("wayland-*[0-9]"))
    return {**environment, "WAYLAND_DISPLAY": displays[0].name if displays else ""}


def ready(quickshell: Path, shell: Path, environment: Mapping[str, str]) -> bool:
    """Wait until the nested shell answers, and say whether it did in time.

    When it never does, what it last said is printed, to say why.
    """
    deadline = time.monotonic() + STARTING
    said = ""
    while time.monotonic() < deadline:
        asked = subprocess.run(  # noqa: S603
            [str(quickshell), "-p", str(shell), "ipc", "call", "launcher", "showing"],
            env=inside(environment),
            capture_output=True,
            text=True,
            check=False,
        )
        if asked.returncode == 0:
            return True
        said = asked.stderr + asked.stdout
        time.sleep(1)
    print(said, file=sys.stderr)
    return False


def type_into(quickshell: Path, shell: Path, environment: Mapping[str, str], text: str) -> None:
    """Open the nested launcher, and type the text into it on the nested niri's keyboard."""
    subprocess.run(  # noqa: S603
        [str(quickshell), "-p", str(shell), "ipc", "call", "launcher", "toggle"],
        env=inside(environment),
        check=True,
    )
    # The launcher takes the keys a frame after it opens.
    time.sleep(1)
    subprocess.run([WTYPE, text], env=inside(environment), check=True)  # noqa: S603


def arguments(argv: Sequence[str]) -> argparse.Namespace:
    """Read the command line."""
    parser = argparse.ArgumentParser(
        prog="shell-preview",
        description="Show the desktop's shell from the working copy, in a niri window.",
    )
    parser.add_argument("--type", help="open the launcher and type this into it")
    parser.add_argument(
        "--check", default="vm-desktop-works", help="the check whose machine's shell is shown"
    )
    parser.add_argument("--account", default="jitsusama", help="the machine's account")
    return parser.parse_args(argv)


def main(argv: Sequence[str]) -> int:
    """Show the shell, and wait for its window to close."""
    options = arguments(argv)
    root = Path(run("git", "rev-parse", "--show-toplevel"))
    system = run("nix", "eval", "--raw", "--impure", "--expr", "builtins.currentSystem")
    print("building the shell's home from the working copy", file=sys.stderr)
    built = machine(f"git+file://{root}", system, options.check, options.account)
    with tempfile.TemporaryDirectory(prefix="shell-preview-") as folder:
        own = Path(folder)
        shell = own / "quickshell"
        linked, copied = lay_out(built.home / ".config/quickshell", sources(root), shell)
        print(f"{linked} files follow the working copy, {copied} were built", file=sys.stderr)
        config = niri_config(built.home / ".config/niri", own / "niri", built.quickshell, shell)
        environment = nested(own, built.data)
        bus = own / "bus.conf"
        bus.write_text(BUS.format(runtime=environment["XDG_RUNTIME_DIR"]))
        niri = subprocess.Popen(  # noqa: S603
            [
                DBUS_RUN_SESSION,
                f"--config-file={bus}",
                "--",
                str(built.niri / "bin/niri"),
                "-c",
                str(config),
            ],
            env=environment,
        )
        if options.type is not None:
            if not ready(built.quickshell, shell, environment):
                print("the shell never answered", file=sys.stderr)
                niri.terminate()
                return 1
            type_into(built.quickshell, shell, environment, options.type)
            print(f"typed {options.type!r} into the launcher", file=sys.stderr)
        return niri.wait()


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
