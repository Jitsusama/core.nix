"""cargo's own filesystem is made, attached, kept, and made again when it's past repairing.

tests/cargo-builds.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), on a
machine whose /var/cache is Btrfs and whose image is asked to be 600 MB. Each test starts from
the service up, and the ones that stop it start it again, so any test runs alone with `-k`. The
tests need no value from Nix.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

import pytest
from vm_helpers import as_account

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine

SERVICE = "cargo-builds.service"
IMAGE = "/var/cache/cargo-builds.img"
FOLDER = "/var/cache/cargo-builds"


@pytest.fixture
def up(machine: BaseMachine) -> BaseMachine:
    """Give the machine with cargo's filesystem attached and mounted."""
    machine.wait_for_unit(SERVICE)
    return machine


def loop_of(machine: BaseMachine) -> str:
    """Name the loop device the folder is mounted from, as /sys names it."""
    source = machine.succeed(f"findmnt -n -o SOURCE --target {FOLDER}").strip()
    return source.removeprefix("/dev/")


def test_the_image_is_mounted_ext4_from_a_direct_loop(up: BaseMachine) -> None:
    """The folder is ext4 on a loop device reading the image with direct IO, at the size asked."""
    assert up.succeed(f"findmnt -n -o FSTYPE --target {FOLDER}").strip() == "ext4"
    loop = loop_of(up)
    assert up.succeed(f"cat /sys/block/{loop}/loop/backing_file").strip() == IMAGE
    assert up.succeed(f"cat /sys/block/{loop}/loop/dio").strip() == "1"
    assert up.succeed(f"stat -c %s {IMAGE}").strip() == str(600 * 1024 * 1024)


def test_the_image_never_copies_on_write(up: BaseMachine) -> None:
    """Btrfs holds the image without copy-on-write, so builds only overwrite its blocks."""
    flags = up.succeed(f"lsattr {IMAGE}").split()[0]
    assert "C" in flags


def test_each_account_has_a_folder_only_it_reads(up: BaseMachine) -> None:
    """Joel's folder is joel's alone, and joel can build in it."""
    assert up.succeed(f"stat -c '%U %a' {FOLDER}/joel").strip() == "joel 700"
    as_account(up, f"touch {FOLDER}/joel/built")


def test_what_is_built_is_kept_across_a_detach_and_attach(up: BaseMachine) -> None:
    """Stopping detaches the image and starting attaches the same one, builds and all."""
    as_account(up, f"echo kept > {FOLDER}/joel/kept")
    up.succeed(f"systemctl stop {SERVICE}")
    up.fail(f"mountpoint -q {FOLDER}")
    up.succeed(f"systemctl start {SERVICE}")
    assert up.succeed(f"cat {FOLDER}/joel/kept").strip() == "kept"


def test_a_reload_leaves_a_build_running(up: BaseMachine) -> None:
    """A switch's reload keeps the folder mounted under a file a build holds open."""
    as_account(up, f"touch {FOLDER}/joel/held")
    # Working in the folder holds the mount, as a build does.
    up.succeed(
        f"systemd-run --uid=joel --unit=build -p Type=exec "
        f"-p WorkingDirectory={FOLDER}/joel sleep infinity"
    )
    up.succeed(f"systemctl reload {SERVICE}")
    assert up.succeed(f"systemctl show -P ActiveState {SERVICE}").strip() == "active"
    up.succeed(f"mountpoint -q {FOLDER} && test -e {FOLDER}/joel/held")
    up.succeed("systemctl stop build")


def test_an_image_past_repairing_is_made_again(up: BaseMachine) -> None:
    """An image whose filesystem can't be read is replaced by an empty one, mounted as before."""
    as_account(up, f"touch {FOLDER}/joel/lost")
    up.succeed(f"systemctl stop {SERVICE}")
    # Every superblock is zeroed, the backups too, so e2fsck has nothing to repair from.
    up.succeed(f"dd if=/dev/zero of={IMAGE} bs=1M count=600 conv=notrunc")
    up.succeed(f"systemctl start {SERVICE}")
    up.succeed(f"mountpoint -q {FOLDER}")
    up.fail(f"test -e {FOLDER}/joel/lost")
    assert up.succeed(f"stat -c '%U %a' {FOLDER}/joel").strip() == "joel 700"
    assert "C" in up.succeed(f"lsattr {IMAGE}").split()[0]
