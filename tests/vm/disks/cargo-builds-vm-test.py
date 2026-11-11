"""cargo's own filesystem is made, attached, kept, and made again when it's past repairing.

cargo-builds.nix, beside this, runs this through pytest inside the driver (tests/lib/vm_pytest.py),
on a machine whose /var/cache is Btrfs and whose image is asked to be 600 MB. Each test starts from
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


def allocated(machine: BaseMachine, image: str = IMAGE) -> int:
    """Say how many bytes of the system's disk an image holds, rather than its size."""
    blocks, block = machine.succeed(f"stat -c '%b %B' {image}").split()
    return int(blocks) * int(block)


def test_the_image_holds_only_what_its_builds_use(up: BaseMachine) -> None:
    """The image's size is a ceiling: it holds what's written, and hands back what's deleted.

    On 9 November a 48 GB image held 48 GB of a disk at 83% while its builds used 21.
    """
    assert "discard" in up.succeed(f"findmnt -n -o OPTIONS --target {FOLDER}")
    empty = allocated(up)
    assert empty < 600 * 1024 * 1024 / 4, f"an empty image holds {empty} bytes"
    as_account(up, f"head -c 200M /dev/urandom > {FOLDER}/joel/built && sync")
    assert allocated(up) > empty + 150 * 1024 * 1024, "a build's bytes went nowhere"
    as_account(up, f"rm {FOLDER}/joel/built && sync")
    # ext4 hands freed blocks back as its journal commits, every 5 s at most.
    up.wait_until_succeeds(
        f"test $(($(stat -c '%b * %B' {IMAGE}))) -lt {empty + 50 * 1024 * 1024}", timeout=60
    )


def test_an_image_made_whole_hands_back_what_is_free_once_attached(up: BaseMachine) -> None:
    """An image made before, every byte reserved, is trimmed down to what it holds when attached."""
    script = up.succeed(f"systemctl show -P ExecStart {SERVICE}").split("path=")[1].split()[0]
    image, folder = "/var/cache/whole.img", "/mnt/whole"
    up.succeed(f"touch {image} && chattr +C {image} && fallocate -l 300M {image}")
    up.succeed(f"mkfs.ext4 -q -E nodiscard {image}")
    assert allocated(up, image) >= 300 * 1024 * 1024
    up.succeed(f"{script} attach {image} 300M {folder} joel:users")
    held = allocated(up, image)
    assert held < 300 * 1024 * 1024 / 4, f"attached, it still holds {held} bytes"
    up.succeed(f"{script} detach {folder} && rm {image}")


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


def test_an_image_bigger_than_the_disk_takes_half_what_is_free(up: BaseMachine) -> None:
    """Asked for more than the disk holds, the image is half what's free, and stays so.

    A stick's 61 GB disk couldn't hold the 48G default, and the boot failed on it.
    """
    script = up.succeed(f"systemctl show -P ExecStart {SERVICE}").split("path=")[1].split()[0]
    image, folder = "/var/cache/big.img", "/mnt/big"
    blocks, block = up.succeed("stat -f -c '%a %S' /var/cache").split()
    free = int(blocks) * int(block)
    up.succeed(f"{script} attach {image} 100G {folder} joel:users")
    made = int(up.succeed(f"stat -c %s {image}").strip())
    assert free * 0.4 < made <= free / 2, f"{made} bytes made with {free} free"
    up.succeed(f"mountpoint -q {folder} && test -d {folder}/joel")
    up.succeed(f"{script} detach {folder} && {script} attach {image} 100G {folder} joel:users")
    assert int(up.succeed(f"stat -c %s {image}").strip()) == made, "attaching again grew it"
    up.succeed(f"{script} detach {folder} && rm {image}")


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
