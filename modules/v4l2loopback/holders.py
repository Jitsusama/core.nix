"""Says which programs have a camera open, in a file a shell can watch.

usage: loopback-holders [--card NAME] [--ignore-unit UNIT]... SOURCE OUTPUT
       loopback-holders --usb [--ignore-unit UNIT]... NODE FOLDER

SOURCE is the camera's V4L2 node, or a file naming it, as the relay writes
one. OUTPUT gets one program name per line, sorted, and nothing while no
program has it open; it's replaced whole, so a reader never sees half of it.
A process in an ignored unit, such as the relay that fills the camera, is
left out; a unit is a cgroup no program can put itself in, as it can take
any name. With --card, only a loopback of that name counts, so a node
number another camera has since been given isn't taken for this one.

It looks when something opens or closes the node, and every two seconds
while anyone holds it, since a holder can change with neither: by exec, by
handing its descriptor to another process, or by exiting and leaving it to
a child. While there's no node, as when the camera's module is reloaded, it
says nobody and waits for one, looking once a second.

With --usb, NODE is a USB camera's, and the file is FOLDER/<the camera's
name>, made and removed with the camera: it's written while the camera is
there and removed when its node goes or this is stopped. A program holding
the camera is listed only while the camera streams, since programs that
only look at a camera, such as PipeWire's, open it for a moment as it
comes; while anyone holds it, it looks once a second, as the stream shows
only in sysfs, which says nothing when it changes.
"""

import argparse
import ctypes
import os
import select
import signal
import struct
import sys
import time

IN_OPEN = 0x020
IN_CLOSE_WRITE = 0x008
IN_CLOSE_NOWRITE = 0x010
IN_DELETE_SELF = 0x400
IN_IGNORED = 0x8000
IN_CLOEXEC = 0o2000000

INTERPRETERS = {"bash", "sh", "dash", "perl", "ruby", "node", "python"}
LONGEST = 64


def node_of(source, card):
    """The node the source is, or names, once there is one of that card."""
    if os.path.exists(source) and not os.path.isfile(source):
        node = os.path.realpath(source)
    else:
        try:
            with open(source) as named:
                node = named.read().strip()
        except OSError:
            return None
    if not node or not os.path.exists(node):
        return None
    if card is not None:
        sysfs = f"/sys/class/video4linux/{os.path.basename(node)}/name"
        try:
            with open(sysfs, "rb") as name:
                if name.read().decode(errors="replace").strip() != card:
                    return None
        except OSError:
            return None
    return node


def video_folder(sysfs, node):
    """A node's folder in sysfs."""
    return os.path.join(sysfs, "class", "video4linux", os.path.basename(node))


def usb_interfaces(sysfs, node):
    """The sysfs folders of the interfaces of the USB device a node is
    on: its node's device is one of them, and they sit beside it."""
    device = os.path.join(video_folder(sysfs, node), "device")
    usb = os.path.dirname(os.path.realpath(device))
    return [entry.path for entry in os.scandir(usb) if entry.is_dir()]


def attribute(folder, name):
    with open(os.path.join(folder, name)) as value:
        return value.read().strip()


def streams(sysfs, node):
    """Whether a USB camera is streaming: one of its video streaming
    interfaces (class 0e, subclass 02) has an alternate setting other than
    zero, which uvcvideo picks to start a stream and puts back to stop it.
    A camera that streams over bulk endpoints stays at zero, so it never
    counts."""
    try:
        interfaces = usb_interfaces(sysfs, node)
    except OSError:
        return False
    for interface in interfaces:
        try:
            if attribute(interface, "bInterfaceClass") != "0e":
                continue
            if attribute(interface, "bInterfaceSubClass") != "02":
                continue
            if int(attribute(interface, "bAlternateSetting")) != 0:
                return True
        except (OSError, ValueError):
            continue
    return False


def listed(names, sysfs, node):
    """Who to name: everyone holding the camera, or for a USB camera,
    everyone only while it streams."""
    if sysfs is None or streams(sysfs, node):
        return names
    return []


def camera_file(sysfs, node, folder):
    """FOLDER/<the camera's name>, as its driver gives it, made a single
    plain file name."""
    path = os.path.join(video_folder(sysfs, node), "name")
    with open(path, "rb") as name:
        called = text(name.read()).replace("/", "-").lstrip(".")
    return os.path.join(folder, called or os.path.basename(node))


def text(raw):
    """A name as a line can carry it: printable, and not too long."""
    name = raw.decode(errors="replace").strip()
    name = "".join(c if c.isprintable() else "?" for c in name)
    return name[:LONGEST]


def unwrapped(name):
    """A name without the dot and -wrapped a Nix wrapper puts around it."""
    if name.startswith(".") and name.endswith("-wrapped"):
        return name[1:-len("-wrapped")]
    return name


def arguments(pid):
    with open(f"/proc/{pid}/cmdline", "rb") as line:
        return [part for part in line.read().split(b"\0") if part]


def program(pid):
    """A process's program by its own name. The kernel cuts that name at 15
    characters, so a Nix wrapper's, or one that long, is taken from the
    executable or the name it was run by. An interpreter is named by the
    script it runs."""
    with open(f"/proc/{pid}/comm", "rb") as comm:
        name = text(comm.read())
    if name.rstrip("0123456789.") in INTERPRETERS:
        args = arguments(pid)
        # Its script, the first argument that's a file: not an option, and
        # not the command a -c hands it.
        for arg in args[1:]:
            if arg.startswith(b"-"):
                if arg == b"-c":
                    break
                continue
            if os.path.isfile(arg):
                return unwrapped(text(os.path.basename(arg)))
            break
        return name
    if not name.startswith(".") and len(name) < 15:
        return name
    try:
        exe = os.path.basename(os.readlink(f"/proc/{pid}/exe"))
        exe = text(os.fsencode(exe))
    except OSError:
        exe = ""
    # The executable only when it's the same program, not an interpreter.
    if exe and exe.lstrip(".").startswith(name.lstrip(".")[:8]):
        return unwrapped(exe)
    if name.startswith("."):
        return unwrapped(name)
    args = arguments(pid)
    if args:
        return unwrapped(text(os.path.basename(args[0]))) or name
    return name


def ignored(pid, units):
    with open(f"/proc/{pid}/cgroup", "rb") as cgroup:
        for line in cgroup.read().decode(errors="replace").splitlines():
            parts = line.rsplit(":", 1)[-1].split("/")
            if any(unit in parts for unit in units):
                return True
    return False


def maps(pid, node, inode):
    """Whether the process still maps the node's buffers, as a reader that
    closed its descriptor and kept streaming would."""
    with open(f"/proc/{pid}/maps", "rb") as mapped:
        for line in mapped:
            fields = line.split(None, 5)
            if len(fields) == 6 and int(fields[4]) == inode:
                if fields[5].strip().decode(errors="replace") == node:
                    return True
    return False


def holders(node, units, before):
    """The programs holding the node, and their processes. Raises
    FileNotFoundError only when the node itself has gone."""
    status = os.stat(node)
    own = os.getpid()
    names, pids = set(), set()
    for entry in os.scandir("/proc"):
        if not entry.name.isdigit() or int(entry.name) == own:
            continue
        pid = int(entry.name)
        # A process can end at any point here; whatever it does, it costs
        # only itself.
        try:
            held = False
            with os.scandir(f"/proc/{pid}/fd") as fds:
                for fd in fds:
                    link = os.readlink(fd.path)
                    # The node under any name: a by-id link, or /dev/videoN.
                    if not link.startswith("/dev/"):
                        continue
                    if os.stat(fd.path).st_rdev == status.st_rdev:
                        held = True
                        break
            if not held and pid in before:
                held = maps(pid, node, status.st_ino)
            if held and not ignored(pid, units):
                names.add(program(pid))
                pids.add(pid)
        except (OSError, ValueError):
            continue
    return sorted(names), pids


def write(output, names):
    body = "".join(f"{name}\n" for name in names)
    temporary = f"{output}.{os.getpid()}"
    with open(temporary, "w", encoding="utf-8", errors="replace") as file:
        file.write(body)
    os.chmod(temporary, 0o644)
    os.replace(temporary, output)


def tidy(output):
    """Removes what a run killed mid-write left beside the output."""
    folder, base = os.path.split(output)
    for entry in os.scandir(folder or "."):
        rest = entry.name[len(base) + 1:]
        if entry.name.startswith(base + ".") and rest.isdigit():
            try:
                os.unlink(entry.path)
            except OSError:
                pass


def watch(libc, node, output, units, sysfs=None):
    """Says who holds the node until it goes. With sysfs, the node is a USB
    camera's, named only while it streams."""
    queue = libc.inotify_init1(IN_CLOEXEC)
    if queue < 0:
        sys.exit(f"inotify: {os.strerror(ctypes.get_errno())}")
    try:
        mask = IN_OPEN | IN_CLOSE_WRITE | IN_CLOSE_NOWRITE | IN_DELETE_SELF
        if libc.inotify_add_watch(queue, node.encode(), mask) < 0:
            error = os.strerror(ctypes.get_errno())
            print(f"can't watch {node}: {error}", file=sys.stderr, flush=True)
            time.sleep(5)
            return
        said, pids = None, set()
        while True:
            try:
                names, pids = holders(node, units, pids)
            except FileNotFoundError:
                return
            shown = listed(names, sysfs, node)
            if shown != said:
                write(output, shown)
                said = shown
            # Whether a USB camera streams shows only in sysfs, so while
            # anyone holds it, it's looked at every second.
            every = 1 if sysfs is not None else 2
            wait = every if names else None
            ready, _, _ = select.select([queue], [], [], wait)
            if not ready:
                continue
            # A program opens a camera several times as it starts; one
            # look after they settle answers for all of them.
            time.sleep(0.1)
            events = os.read(queue, 65536)
            offset = 0
            while offset < len(events):
                _, event, _, length = struct.unpack_from(
                    "iIII", events, offset
                )
                if event & (IN_DELETE_SELF | IN_IGNORED):
                    return
                offset += 16 + length
    finally:
        os.close(queue)


def usb(libc, node, folder, units, sysfs):
    """Says who streams from a USB camera while it's there, in a file of
    its own that goes with it."""
    output = camera_file(sysfs, node, folder)

    def stopped(number, frame):
        raise SystemExit(0)

    signal.signal(signal.SIGTERM, stopped)
    tidy(output)
    try:
        watch(libc, node, output, units, sysfs)
    finally:
        try:
            os.unlink(output)
        except FileNotFoundError:
            pass


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--card")
    parser.add_argument("--ignore-unit", action="append", default=[])
    parser.add_argument("--usb", action="store_true")
    # Where sysfs is, for the tests.
    parser.add_argument("--sysfs", default="/sys", help=argparse.SUPPRESS)
    parser.add_argument("source")
    parser.add_argument("output")
    options = parser.parse_args()
    libc = ctypes.CDLL(None, use_errno=True)
    units = set(options.ignore_unit)
    if options.usb:
        node = os.path.realpath(options.source)
        if os.path.exists(node):
            usb(libc, node, options.output, units, options.sysfs)
        return
    tidy(options.output)
    gone = False
    while True:
        node = node_of(options.source, options.card)
        if node is None:
            if not gone:
                write(options.output, [])
                gone = True
            time.sleep(1)
            continue
        gone = False
        watch(libc, node, options.output, units)
        write(options.output, [])
        # So a node it can't watch is tried again, not spun on.
        time.sleep(0.5)


if __name__ == "__main__":
    main()
