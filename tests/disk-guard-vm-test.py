"""The disk guards, the machine's and an account's, driven through their bands in a VM.

tests/disk-guard.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and
its opening comment says what the test proves. Each test asks for the state it starts from as a
fixture: the account up, then its guard on the test's bands with targets and a cache to take.
What a test fills or starts it takes back as it ends, and the guard has to call /home healthy
again before the next begins, so any test runs alone with `-k`. The drop-ins that give each
guard the test's config, and the build script that waits, are disk-guard.nix's, given as the
`values` fixture.
"""

from __future__ import annotations

import re
import shlex
from datetime import timedelta
from itertools import count
from typing import TYPE_CHECKING, Any

import pytest
from vm_helpers import as_account

if TYPE_CHECKING:
    from collections.abc import Callable, Iterator

    from test_driver.machine import BaseMachine

# The account's guard's status, which the shell reads too.
STATUS = "/run/user/1000/disk-guard/status.json"
# The guard, on the machine's path.
GUARD = "/run/current-system/sw/bin/disk-guard"
# A runaway's rate, a mebibyte a second over the last minute, as the guard's daemon.rs has it.
RUNAWAY = 1 << 20
# How long a band or a deletion may take to follow the disk: a pass, its heartbeat and a step.
FOLLOWING = timedelta(minutes=5)
# How long a freeze or a thaw may take to show: a few of the guard's five-second steps.
STEPS = timedelta(minutes=2)


class Account:
    """Joel's account on the machine, and the files the test fills its disks with."""

    def __init__(self, machine: BaseMachine) -> None:
        """Act on Joel's account on that machine."""
        self.machine = machine
        # Each piece of fill gets the next number, across every fill, so no piece takes
        # another's name.
        self.fills = count(1)

    def run(self, command: str, *, succeed: bool = True) -> str:
        """Run a command as Joel in his home, giving its output; unless succeed, it must fail."""
        return as_account(self.machine, command, succeed=succeed)

    def wait_for_band(self, path: str, band: str) -> None:
        """Wait until the account's guard puts the filesystem at the path in the band."""
        self.machine.wait_until_succeeds(
            f'jq -e \'.filesystems[] | select(.path == "{path}") | .band == "{band}"\' {STATUS}',
            timeout=FOLLOWING,
        )

    def fill_to(self, share: float) -> None:
        """Fill /home until its free space is that share of it, with files no source names.

        The fill goes in while the account's guard is stopped, so it starts on a fresh rate
        rather than forecasting a worse band from a jump only a test makes. Btrfs counts
        unallocated space as free that one allocation can't wholly take, so it goes in
        pieces, and its free space only shows a piece once the transaction commits.
        """
        self.run("systemctl --user stop disk-guard")
        while True:
            free, total = (
                int(word) for word in self.run("df -B1 --output=avail,size /home | tail -1").split()
            )
            wanted = int(free - share * total)
            piece = f"fallocate -l {min(wanted, 256 << 20)} ~/fill-{next(self.fills)} && sync -f ~"
            if wanted <= 0 or self.run(piece + " && echo ok || true").strip() != "ok":
                break
        self.run("systemctl --user start disk-guard")

    def built(self, name: str, megabytes: int) -> None:
        """Build a crate of that name in ~/src, with that many mebibytes more in its target."""
        self.run(f"cargo new -q --lib --vcs none ~/src/{name}")
        self.run(f"cd ~/src/{name} && cargo build -q --offline")
        self.run(f"head -c {megabytes}M /dev/urandom > ~/src/{name}/target/debug/bulk")

    def gone(self, path: str) -> None:
        """Wait until nothing is at the path."""
        self.machine.wait_until_succeeds(f"! test -e {path}", timeout=FOLLOWING)

    def there(self, path: str) -> None:
        """Check something is at the path."""
        self.machine.succeed(f"test -e {path}")

    def frozen_unit(self) -> str:
        """Wait until the account's guard has frozen one unit, and give its name."""
        self.machine.wait_until_succeeds(f"jq -e '.frozen | length == 1' {STATUS}", timeout=STEPS)
        return self.machine.succeed(f"jq -r '.frozen[0].unit' {STATUS}").strip()

    def seen_running_away(self, pid: str) -> None:
        """Wait until the account's guard looks again and sees pid writing at a runaway's rate.

        While a disk is short, each step names who writes most of the last minute's writes,
        at a rate that changes with every sample, so each look writes the status afresh under
        a later heartbeat. It names them by the rule a freeze goes by, so a look naming pid at
        a runaway's rate is one at which the guard would freeze it, had it not let it go.
        """
        seen = self.machine.succeed(f"jq -r .heartbeat {STATUS}").strip()
        running_away = f".writer.pid == {pid} and .writer.rate >= {RUNAWAY}"
        self.machine.wait_until_succeeds(
            f"jq -e '.heartbeat > {seen} and {running_away}' {STATUS}", timeout=STEPS
        )

    def freezes_tried(self) -> int:
        """Give how many times the account's guard has tried to freeze a writer, as it journals."""
        tried = "jq -s '[.[] | select(.kind == \"freeze\")] | length'"
        return int(self.run(f"{GUARD} journal --json | {tried}"))

    def freezer_state(self, unit: str) -> str:
        """Give whether the account's unit is frozen or running, as systemd says."""
        return self.run(f"systemctl --user show -P FreezerState {unit}").strip()


@pytest.fixture(scope="module")
def account(machine: BaseMachine) -> Account:
    """Give Joel's account, its service manager running without a login."""
    machine.wait_for_unit("multi-user.target")
    machine.succeed("loginctl enable-linger joel")
    machine.wait_for_unit("user@1000.service")
    return Account(machine)


@pytest.fixture(scope="module")
def on_test_bands(account: Account, values: dict[str, Any]) -> Account:
    """Give the account with its guard on the test's bands, and something for it to take.

    ~/src holds a target built three days ago and one built now, ~/.cache/thing a cache, and
    /srv/small is the account's, all on a healthy /home.
    """
    account.machine.succeed("chown joel:users /srv/small")
    account.run("mkdir -p ~/src ~/.cache/thing ~/.config/systemd/user/disk-guard.service.d")
    drop_in = values["userDropIn"]
    account.run(f"cp {drop_in} ~/.config/systemd/user/disk-guard.service.d/test.conf")
    account.run("head -c 300M /dev/urandom > ~/.cache/thing/blob")
    account.built("idle", 700)
    account.built("fresh", 300)
    # Built three days ago, and nothing since.
    account.run("find ~/src/idle -exec touch -h -d '3 days ago' {} +")
    account.run("systemctl --user daemon-reload && systemctl --user restart disk-guard")
    account.machine.wait_until_succeeds(f"jq -e '.filesystems | length == 2' {STATUS}")
    account.wait_for_band("/home", "healthy")
    return account


@pytest.fixture
def filling(on_test_bands: Account) -> Iterator[Callable[[float], None]]:
    """Give a way to fill /home to a share of free space; the fill goes once the test ends.

    The guard has to call /home healthy again once it goes, so each test leaves the disk
    as it found it, and the band is seen to follow the disk up as well as down.
    """
    yield on_test_bands.fill_to
    on_test_bands.run("rm -f ~/fill-*")
    on_test_bands.wait_for_band("/home", "healthy")


@pytest.fixture
def running_away(on_test_bands: Account) -> Iterator[Callable[[str], None]]:
    """Give a way to start a writer of about 4 MiB a second; each stops once the test ends.

    It writes to ~/name from a scope of that name, as an app the desktop starts is: what su
    starts sits in a session, and nothing in a session is ever frozen.
    """
    started: list[str] = []

    def run_away(name: str) -> None:
        writes = "exec 3>>~/" + name + "; while :; do printf '%1048576s' x >&3; sleep 0.25; done"
        on_test_bands.run(
            f"setsid -f systemd-run --user --scope --quiet --unit={name} bash -c "
            + shlex.quote(writes)
            + " > /dev/null 2>&1"
        )
        started.append(name)

    yield run_away
    for name in started:
        # Thawed first, since a frozen process can't take the signal that ends it.
        on_test_bands.run(f"systemctl --user thaw {name}.scope disk-guard-freeze-*.scope || true")
        on_test_bands.run(f"systemctl --user stop {name}.scope 'disk-guard-freeze-*' || true")
        on_test_bands.run(f"rm -f ~/{name}")


def test_both_guards_run_on_the_configs_their_modules_render(account: Account) -> None:
    """Both guards run on the configs their modules render."""
    machine = account.machine
    machine.wait_for_unit("disk-guard.service")
    machine.wait_for_unit("disk-guard.service", "joel")
    assert machine.succeed("stat -c %a /run/disk-guard/socket").strip() == "666"
    machine.wait_until_succeeds("jq -e '.filesystems | length > 0' /run/disk-guard/status.json")
    machine.wait_until_succeeds(f"jq -e '.filesystems | length > 0' {STATUS}")


def test_the_machines_guard_leaves_its_status_while_its_down(account: Account) -> None:
    """The machine's guard leaves its status while it's down, and takes its socket back.

    The shell tells a stopped guard by its last heartbeat ageing, so the status stays while
    the guard is down.
    """
    machine = account.machine
    machine.wait_for_unit("disk-guard.service")
    machine.succeed("systemctl stop disk-guard")
    machine.succeed("jq -e '.filesystems | length > 0' /run/disk-guard/status.json")
    machine.succeed("systemctl start disk-guard")
    machine.wait_for_unit("disk-guard.service")
    machine.wait_until_succeeds("test -S /run/disk-guard/socket")
    assert machine.succeed("stat -c %a /run/disk-guard/socket").strip() == "666"


def test_on_the_tests_config_the_accounts_guard_watches_both_disks(on_test_bands: Account) -> None:
    """On the test's config the account's guard watches /home and /srv/small, and nothing else."""
    paths = on_test_bands.machine.succeed(f"jq -r '.filesystems[].path' {STATUS}").split()
    assert sorted(paths) == ["/home", "/srv/small"]


def test_the_accounts_guard_watches_who_opens_each_targets_builds(on_test_bands: Account) -> None:
    """The account's guard watches who opens each target's builds."""
    uses = "/home/joel/.local/state/disk-guard/uses.json"
    on_test_bands.machine.wait_until_succeeds(
        f"jq -e '.watched | has(\"/home/joel/src/fresh/target/debug/deps\")' {uses}"
    )


def test_a_target_in_a_subvolume_is_offered_and_one_in_a_read_only_snapshot_isnt(
    on_test_bands: Account, values: dict[str, Any]
) -> None:
    """A target in a Btrfs subvolume is offered; one in a read-only snapshot of it isn't.

    A subvolume has a device number of its own, as a mount does, and the work broker clones
    each tree as one, so a walk that stopped at every change of device never saw their builds.
    A read-only snapshot can't be emptied, so it's left out. The subvolumes go once it's seen.
    """
    machine, clone, frozen = on_test_bands.machine, "/home/joel/src/clone", "/home/joel/src/frozen"
    explain = f"{GUARD} explain {{}}/target --config {values['userConfig']}"
    machine.succeed(f"btrfs subvolume create {clone} && chown joel:users {clone}")
    try:
        on_test_bands.run(f"cargo init -q --lib --vcs none {clone}")
        on_test_bands.run(f"cd {clone} && cargo build -q --offline")
        machine.succeed(f"btrfs subvolume snapshot -r {clone} {frozen}")
        offered = on_test_bands.run(explain.format(clone))
        assert offered.startswith(f"{clone}/target: "), offered
        left = on_test_bands.run(explain.format(frozen))
        assert left.startswith("No source offers"), left
    finally:
        for subvolume in (frozen, clone):
            machine.succeed(f"! test -e {subvolume} || btrfs subvolume delete {subvolume}")


# Mebibytes of noise a footprint is asked about, which no compression shrinks.
NOISE = 8
# Mebibytes 64 MiB of one repeated word may take once compressed, copy and all.
TEXT_TAKES_AT_MOST = 16


def test_the_machines_guard_says_what_an_accounts_files_take_on_the_disk(account: Account) -> None:
    """The machine's guard says what an account's files take on the disk, and only its own.

    /home compresses as it writes, so 64 MiB of text takes a few, and a copy made by reference
    takes nothing more; 8 MiB of noise doesn't compress, so it takes all of its 8. The files
    are synced first, since Btrfs only gives a write its runs once it lands. A folder of
    root's, and a link of joel's that leads to one, are never walked for him.
    """
    account.machine.wait_until_succeeds("test -S /run/disk-guard/socket")
    account.run("mkdir -p ~/footprint && yes footprint | head -c 64M > ~/footprint/text")
    account.run("cp --reflink=always ~/footprint/text ~/footprint/copy")
    account.run(f"head -c {NOISE}M /dev/urandom > ~/footprint/noise && sync -f ~")
    try:
        said = account.run(f"{GUARD} footprint ~/footprint").strip()
        disk = r"/home/joel/footprint: ([\d.]+) MiB on the disk, for 136\.0 MiB of files\."
        found = re.fullmatch(disk, said)
        assert found, said
        assert NOISE <= float(found[1]) < NOISE + TEXT_TAKES_AT_MOST, said
        account.run("ln -s /etc ~/to-etc")
        not_his = ["/etc", "~/to-etc"]
        refused = account.run(f"{GUARD} footprint {' '.join(not_his)}", succeed=False)
        assert refused.count("can't be read") == len(not_his), refused
    finally:
        account.run("rm -rf ~/footprint ~/to-etc")


def test_at_pressure_an_idle_target_goes_and_a_fresh_one_and_a_cache_stay(
    on_test_bands: Account, filling: Callable[[float], None]
) -> None:
    """At pressure an idle target goes, and a fresh one and a cache stay.

    The idle target alone leaves the pass short of its target, so only the fresh one's window
    keeps it.
    """
    filling(0.26)
    on_test_bands.gone("/home/joel/src/idle/target/debug/bulk")
    on_test_bands.wait_for_band("/home", "healthy")
    on_test_bands.there("/home/joel/src/fresh/target/debug/bulk")
    on_test_bands.there("/home/joel/.cache/thing/blob")
    on_test_bands.there("/home/joel/src/idle/Cargo.toml")


def test_at_critical_a_build_holding_its_locks_keeps_its_target_until_it_ends(
    on_test_bands: Account, filling: Callable[[float], None], values: dict[str, Any]
) -> None:
    """At critical a build holding its locks keeps its target until it ends."""
    joel = on_test_bands
    joel.run("cargo new -q --lib --vcs none ~/src/busy")
    joel.run(f"cp {values['waiting']} ~/src/busy/build.rs")
    joel.run("mkdir -p ~/src/busy/target/debug")
    joel.run("head -c 200M /dev/urandom > ~/src/busy/target/debug/bulk")
    joel.run("cd ~/src/busy && setsid -f cargo build -q --offline > ~/busy.log 2>&1")
    joel.machine.wait_until_succeeds("pgrep -u joel -f build-script-build")
    filling(0.05)
    joel.gone("/home/joel/.cache/thing/blob")
    joel.gone("/home/joel/src/fresh/target/debug/bulk")
    joel.there("/home/joel/src/busy/target/debug/bulk")
    joel.machine.succeed("pgrep -u joel -f build-script-build")
    joel.run("touch ~/release")
    joel.machine.wait_until_fails("pgrep -u joel -x cargo", timeout=STEPS)
    # Built a moment ago, it waits for critical.
    filling(0.05)
    joel.gone("/home/joel/src/busy/target/debug/bulk")


def test_a_target_on_a_disk_too_full_for_a_trash_is_deleted_in_place(
    on_test_bands: Account,
) -> None:
    """A target on a disk too full for a trash is deleted in place."""
    joel = on_test_bands
    joel.run("cargo new -q --lib --vcs none /srv/small/tiny")
    joel.run("cd /srv/small/tiny && cargo build -q --offline")
    joel.run("head -c 40M /dev/urandom > /srv/small/tiny/target/debug/bulk")
    # Held open, so it's kept until the disk is full.
    joel.run("setsid -f sleep infinity < /srv/small/tiny/target/debug/bulk > /dev/null 2>&1")
    # ext4 gives back what delayed allocation over-reserved once written out, so the fill goes
    # on until a write after a sync finds nothing.
    joel.run(
        "for piece in 1M 4k 4k 4k; do"
        " dd if=/dev/zero of=/srv/small/fill-$piece-$RANDOM bs=$piece 2>/dev/null;"
        " sync -f /srv/small; done; true"
    )
    joel.run("mkdir /srv/small/room", succeed=False)
    joel.machine.succeed("pkill -u joel -x sleep")
    joel.gone("/srv/small/tiny/target/debug/bulk")
    joel.machine.succeed("ls /srv/small/fill-*")
    joel.run("! ls -a /srv/small/tiny | grep -q disk-guard-trash")


def test_a_runaway_writer_at_critical_is_frozen_and_stays_thawed_once_thawed_by_hand(
    on_test_bands: Account,
    filling: Callable[[float], None],
    running_away: Callable[[str], None],
) -> None:
    """A runaway writer at critical is frozen, and stays thawed once thawed by hand.

    The guard lets it go in the look that empties its list; a later one sees it running away
    again. Its scope is still there, so a second freeze would fail and leave it running: it's
    the try, journaled, that shows the guard leaving it alone.
    """
    joel = on_test_bands
    filling(0.03)
    running_away("runaway")
    unit = joel.frozen_unit()
    writer = joel.machine.succeed(f"jq -r '.frozen[0].pid' {STATUS}").strip()
    freezes = joel.freezes_tried()
    assert joel.freezer_state(unit) == "frozen"
    assert "Thawed " + unit in joel.run(f"{GUARD} thaw")
    joel.machine.wait_until_succeeds(f"jq -e '.frozen | length == 0' {STATUS}", timeout=STEPS)
    joel.seen_running_away(writer)
    assert joel.freezes_tried() == freezes, "the guard tried to freeze a writer thawed by hand"
    assert joel.freezer_state(unit) == "running"


def test_a_frozen_writer_thaws_by_itself_once_its_disk_has_room(
    on_test_bands: Account,
    filling: Callable[[float], None],
    running_away: Callable[[str], None],
) -> None:
    """A frozen writer thaws by itself once its disk has room."""
    joel = on_test_bands
    filling(0.03)
    running_away("again")
    unit = joel.frozen_unit()
    assert joel.freezer_state(unit) == "frozen"
    joel.run("rm ~/fill-*")
    joel.machine.wait_until_succeeds(f"jq -e '.frozen | length == 0' {STATUS}", timeout=STEPS)
    assert joel.freezer_state(unit) == "running"


@pytest.fixture
def store_pressed(account: Account, values: dict[str, Any]) -> Iterator[BaseMachine]:
    """Give the machine with its guard's store under pressure; its own config comes back after."""
    machine = account.machine
    drop_in = "/run/systemd/system/disk-guard.service.d/test.conf"
    machine.succeed(
        f"mkdir -p {drop_in.rsplit('/', 1)[0]}",
        f"cp {values['systemDropIn']} {drop_in}",
    )
    yield machine
    machine.succeed(f"rm {drop_in}", "systemctl daemon-reload", "systemctl restart disk-guard")


def test_with_its_store_pressed_the_machine_expires_generations_and_collects_a_dead_build(
    account: Account, store_pressed: BaseMachine
) -> None:
    """With its store pressed the machine expires generations, collects and clears a dead build.

    It keeps the newest five of a profile's generations.
    """
    machine = store_pressed
    # Left by a build process that's gone, as a crashed one leaves it.
    dead_build = "/nix/var/nix/builds/nix-4000000-1"
    machine.succeed(f"mkdir -p -m 0700 {dead_build} && head -c 1M /dev/zero > {dead_build}/out")
    paths: list[str] = []
    for n in range(1, 9):
        machine.succeed(f"mkdir -p /root/gen-{n} && echo {n} > /root/gen-{n}/n")
        paths.append(machine.succeed(f"nix-store --add /root/gen-{n}").strip())
        machine.succeed(f"nix-env -p /nix/var/nix/profiles/test --set {paths[-1]}")
    machine.succeed("echo junk > /root/junk")
    junk = machine.succeed("nix-store --add /root/junk").strip()
    machine.succeed("systemctl daemon-reload", "systemctl restart disk-guard")
    account.gone(junk)
    account.gone(dead_build)
    for n in range(1, 4):
        account.gone(f"/nix/var/nix/profiles/test-{n}-link")
        account.gone(paths[n - 1])
    for n in range(4, 9):
        account.there(f"/nix/var/nix/profiles/test-{n}-link")
        account.there(paths[n - 1])
    assert machine.succeed("readlink /nix/var/nix/profiles/test").strip() == "test-8-link"
