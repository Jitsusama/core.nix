# The checks of v4l2loopback, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
{
  # The camera watch's tests, which run as it builds. Over a sysfs of its
  # own, a USB camera's holder is named only while one of its streaming
  # interfaces has a nonzero alternate setting, its file is named for the
  # camera, made a plain file name, and goes when its node does or the
  # watch is stopped; a name is one printable line, a wrapper's named for
  # what it wraps, and only a unit in a process's cgroup is ignored. What
  # takes real holders is vm-loopback-says-who-holds-it's.
  usb-camera-holders-follow-the-stream = pkgs.callPackage ./holders/package.nix { };
}
// onLinux {
  # clippy finds nothing in the cameras' holders watcher, its tests included.
  v4l2loopback-holders-passes-clippy = passesClippy "v4l2loopback-holders-passes-clippy" (
    pkgs.callPackage ./holders/package.nix { }
  );
}
