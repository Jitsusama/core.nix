"""Says which programs have a camera open, in a file a shell can watch.

usage: loopback-holders NODE-OR-FILE OUTPUT [IGNORE...]

NODE-OR-FILE is the camera's V4L2 node, or a file naming it, as the relay
writes one. OUTPUT gets one program name per line, sorted, and nothing while
no program has it open; it's replaced whole, so a reader never sees half of
it. IGNORE names programs left out, such as the relay that fills the camera.
It wakes only when something opens or closes the node. While there's no
node, as when the camera's module is reloaded, it says nobody and waits for
one, looking once a second.
"""

import ctypes
import os
import select
import struct
import sys
import time

IN_OPEN = 0x020
IN_CLOSE_WRITE = 0x008
IN_CLOSE_NOWRITE = 0x010
IN_DELETE_SELF = 0x400
IN_IGNORED = 0x8000
IN_CLOEXEC = 0o2000000


def node_of(path):
    """The node the path is, or names, once there is one."""
    if os.path.exists(path) and not os.path.isfile(path):
        return path
    try:
        with open(path) as named:
            node = named.read().strip()
    except OSError:
        return None
    return node if node and os.path.exists(node) else None


def unwrapped(name):
    """A name without the dot and -wrapped a Nix wrapper puts around it."""
    if name.startswith(".") and name.endswith("-wrapped"):
        return name[1:-len("-wrapped")]
    return name


def program(pid):
    """A process's program by its own name, which a script has too, where
    its executable is the interpreter. The kernel cuts that name at 15
    characters, so a Nix wrapper's, or one that long, is taken from the
    executable, or failing that from the name it was run by."""
    with open(f"/proc/{pid}/comm") as comm:
        name = comm.read().strip()
    if not name.startswith(".") and len(name) < 15:
        return name
    try:
        exe = os.path.basename(os.readlink(f"/proc/{pid}/exe"))
    except OSError:
        exe = ""
    # The executable only when it's the same program, not an interpreter.
    if exe.lstrip(".").startswith(name.lstrip(".")[:8]):
        return unwrapped(exe)
    if name.startswith("."):
        return unwrapped(name)
    try:
        with open(f"/proc/{pid}/cmdline", "rb") as line:
            first = line.read().split(b"\0")[0].decode(errors="replace")
    except OSError:
        return name
    return unwrapped(os.path.basename(first)) or name


def holders(node, ignore):
    own = os.getpid()
    target = os.stat(node).st_rdev
    names = set()
    for entry in os.scandir("/proc"):
        if not entry.name.isdigit() or int(entry.name) == own:
            continue
        try:
            fds = os.scandir(f"/proc/{entry.name}/fd")
        except OSError:
            continue
        with fds:
            for fd in fds:
                try:
                    link = os.readlink(fd.path)
                    # The node under any name: a by-id link, or /dev/videoN.
                    if not link.startswith("/dev/"):
                        continue
                    if os.stat(fd.path).st_rdev != target:
                        continue
                    name = program(entry.name)
                except OSError:
                    continue
                if name not in ignore:
                    names.add(name)
                break
    return sorted(names)


def write(output, names):
    text = "".join(f"{name}\n" for name in names)
    temporary = f"{output}.{os.getpid()}"
    with open(temporary, "w") as file:
        file.write(text)
    os.chmod(temporary, 0o644)
    os.replace(temporary, output)


def watch(libc, node, output, ignore):
    """Says who holds the node until it goes."""
    queue = libc.inotify_init1(IN_CLOEXEC)
    if queue < 0:
        sys.exit(f"inotify: {os.strerror(ctypes.get_errno())}")
    try:
        mask = IN_OPEN | IN_CLOSE_WRITE | IN_CLOSE_NOWRITE | IN_DELETE_SELF
        if libc.inotify_add_watch(queue, node.encode(), mask) < 0:
            return
        said = None
        while True:
            try:
                names = holders(node, ignore)
            except FileNotFoundError:
                return
            if names != said:
                write(output, names)
                said = names
            select.select([queue], [], [])
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


def main():
    source, output, *ignore = sys.argv[1:]
    libc = ctypes.CDLL(None, use_errno=True)
    gone = False
    while True:
        node = node_of(source)
        if node is None:
            if not gone:
                write(output, [])
                gone = True
            time.sleep(1)
            continue
        gone = False
        watch(libc, node, output, set(ignore))
        write(output, [])
        # So a node it can't watch is tried again, not spun on.
        time.sleep(0.5)


if __name__ == "__main__":
    main()
