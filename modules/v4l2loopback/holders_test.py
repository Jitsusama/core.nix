"""What holders.py makes of a USB camera, over a sysfs of its own: a holder
is named only while one of the camera's streaming interfaces has a nonzero
alternate setting, the file is named for the camera and made safe, and the
file goes when the camera's node does or the program is stopped."""

import os
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import unittest

import holders

HERE = os.path.dirname(os.path.abspath(__file__))


def scratch(test):
    """A folder of the test's own, gone when it ends."""
    root = tempfile.mkdtemp()
    test.addCleanup(shutil.rmtree, root, True)
    return root


def write(path, text):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w") as file:
        file.write(text)


class Camera:
    """A USB camera as sysfs shows one, with its control interface (which
    the node's device is), a streaming interface, and an audio interface
    that has an alternate setting of its own."""

    def __init__(self, root, name="MX Brio 705 for Business"):
        self.sysfs = os.path.join(root, "sys")
        usb = os.path.join(self.sysfs, "devices", "usb2", "2-1")
        self.interfaces = {}
        for interface, (cls, sub) in {
            "2-1:1.0": ("0e", "01"),
            "2-1:1.1": ("0e", "02"),
            "2-1:1.4": ("01", "02"),
        }.items():
            folder = os.path.join(usb, interface)
            write(os.path.join(folder, "bInterfaceClass"), f"{cls}\n")
            write(os.path.join(folder, "bInterfaceSubClass"), f"{sub}\n")
            write(os.path.join(folder, "bAlternateSetting"), " 0\n")
            self.interfaces[interface] = folder
        video = os.path.join(self.sysfs, "class", "video4linux", "video0")
        os.makedirs(video)
        os.symlink(self.interfaces["2-1:1.0"], os.path.join(video, "device"))
        write(os.path.join(video, "name"), f"{name}\n")
        # The node stands in as a folder: it exists and isn't a file, which
        # is all that's asked of it before it's watched.
        self.node = os.path.join(root, "dev", "video0")
        os.makedirs(self.node)

    def alternate(self, interface, setting):
        write(os.path.join(self.interfaces[interface], "bAlternateSetting"), f"{setting:2d}\n")


class Streaming(unittest.TestCase):
    def setUp(self):
        self.root = scratch(self)
        self.camera = Camera(self.root)

    def test_idle_camera_names_nobody(self):
        self.assertFalse(holders.streams(self.camera.sysfs, self.camera.node))
        self.assertEqual(holders.listed(["firefox"], self.camera.sysfs, self.camera.node), [])

    def test_streaming_camera_names_its_holders(self):
        self.camera.alternate("2-1:1.1", 9)
        self.assertTrue(holders.streams(self.camera.sysfs, self.camera.node))
        self.assertEqual(
            holders.listed(["firefox"], self.camera.sysfs, self.camera.node), ["firefox"]
        )

    def test_only_a_video_streaming_interface_counts(self):
        self.camera.alternate("2-1:1.4", 1)
        self.assertFalse(holders.streams(self.camera.sysfs, self.camera.node))

    def test_stopping_the_stream_names_nobody_again(self):
        self.camera.alternate("2-1:1.1", 9)
        self.camera.alternate("2-1:1.1", 0)
        self.assertEqual(holders.listed(["firefox"], self.camera.sysfs, self.camera.node), [])

    def test_a_loopback_names_whoever_holds_it(self):
        self.assertEqual(holders.listed(["firefox"], None, self.camera.node), ["firefox"])

    def test_a_camera_gone_from_sysfs_isnt_streaming(self):
        missing = os.path.join(self.root, "dev", "video9")
        self.assertFalse(holders.streams(self.camera.sysfs, missing))


class Naming(unittest.TestCase):
    def test_the_file_is_named_for_the_camera(self):
        root = scratch(self)
        camera = Camera(root)
        self.assertEqual(
            holders.camera_file(camera.sysfs, camera.node, "/run/camera-holders"),
            "/run/camera-holders/MX Brio 705 for Business",
        )

    def test_a_name_is_made_a_plain_file_name(self):
        root = scratch(self)
        camera = Camera(root, name="../Cam/Era\x07")
        self.assertEqual(
            holders.camera_file(camera.sysfs, camera.node, "/run/camera-holders"),
            "/run/camera-holders/-Cam-Era?",
        )


class Lifetime(unittest.TestCase):
    """The program itself, run as its unit runs it."""

    def setUp(self):
        self.root = scratch(self)
        self.camera = Camera(self.root)
        self.folder = os.path.join(self.root, "camera-holders")
        os.makedirs(self.folder)
        self.file = os.path.join(self.folder, "MX Brio 705 for Business")
        self.program = subprocess.Popen([
            sys.executable, os.path.join(HERE, "holders.py"),
            "--usb", "--sysfs", self.camera.sysfs, self.camera.node, self.folder,
        ])

    def tearDown(self):
        if self.program.poll() is None:
            self.program.kill()
            self.program.wait()

    def until(self, condition, what):
        for _ in range(100):
            if condition():
                return
            time.sleep(0.05)
        self.fail(what)

    def test_the_file_is_there_while_the_camera_is(self):
        self.until(lambda: os.path.exists(self.file), "no file while the camera's there")
        with open(self.file) as file:
            self.assertEqual(file.read(), "")

    def test_the_file_goes_with_the_node(self):
        self.until(lambda: os.path.exists(self.file), "no file while the camera's there")
        os.rmdir(self.camera.node)
        self.until(lambda: not os.path.exists(self.file), "the file outlived the camera")
        self.assertEqual(self.program.wait(timeout=5), 0)

    def test_the_file_goes_when_it_is_stopped(self):
        self.until(lambda: os.path.exists(self.file), "no file while the camera's there")
        self.program.send_signal(signal.SIGTERM)
        self.assertEqual(self.program.wait(timeout=5), 0)
        self.assertFalse(os.path.exists(self.file))


if __name__ == "__main__":
    unittest.main()
