# The checks of v4l2loopback, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  bareNixos,
  holds,
  lib,
  onLinux,
  passesClippy,
  pkgs,
  self,
  ...
}:
let
  # The type a camera's ignoreUnits is held to, read from the module's own
  # option, so what's checked is what a machine naming a unit meets.
  ignoreUnits =
    (
      (bareNixos [ self.nixosModules.v4l2loopback ])
      .options.jitsusama.v4l2loopback.holders.type.getSubOptions
        [ ]
    ).ignoreUnits.type;
  # Whether a machine naming `unit` builds, as a module setting it would:
  # a refused unit throws as the list is merged, which tryEval sees.
  takes =
    unit:
    (builtins.tryEval (
      builtins.deepSeq
        (lib.evalModules {
          modules = [
            {
              options.ignoreUnits = lib.mkOption { type = ignoreUnits; };
              config.ignoreUnits = [ unit ];
            }
          ];
        }).config.ignoreUnits
        true
    )).success;
  # A person's manager, a session's scope and init.scope hold everything
  # their people run, a slice holds no process of its own, and a path is
  # no unit's name.
  refused = [
    "user@1000.service"
    "session-3.scope"
    "init.scope"
    "a.slice"
    "a/b.service"
  ];
  # A relay's own service or scope, and a system service whose name only
  # starts like a person's manager.
  kept = [
    "camera.service"
    "v4l2-relayd.service"
    "run-u12.scope"
    "user-runtime-dir@1000.service"
  ];
  wronglyTaken = lib.filter takes refused;
  wronglyRefused = lib.filter (unit: !takes unit) kept;
  misread = lib.concatStringsSep "; " (
    lib.optional (wronglyTaken != [ ]) "takes ${lib.concatStringsSep ", " wronglyTaken}"
    ++ lib.optional (wronglyRefused != [ ]) "refuses ${lib.concatStringsSep ", " wronglyRefused}"
  );
in
# The watch is built for Linux alone, where its cameras are.
onLinux {
  # A machine naming a unit that holds everyone, a person's manager, a
  # session's scope or init.scope, never builds, nor one naming a slice or a
  # path, while a relay's own service or scope does. The watch refuses the
  # same at its start, but only this says a machine never gets that far.
  a-camera-watch-refuses-a-unit-holding-everyone =
    holds "a-camera-watch-refuses-a-unit-holding-everyone" "A camera watch's ignoreUnits ${misread}"
      (wronglyTaken == [ ] && wronglyRefused == [ ]);

  # The camera watch's tests, which run as it builds. Over a sysfs of its
  # own, a USB camera's holder is named only while one of its streaming
  # interfaces has a nonzero alternate setting, its file is named for the
  # camera, made a plain file name, and goes when its node does or the
  # watch is stopped; a name is one printable line, a wrapper's named for
  # what it wraps, and only a system unit a process's cgroup is in is
  # ignored, never a person's own unit of the same name. While
  # it streams, it's said every ten seconds with the frames uvcvideo
  # counted over the stretch. What takes real holders is
  # vm-loopback-says-who-holds-it's.
  usb-camera-holders-follow-the-stream = pkgs.callPackage ./holders/package.nix { };

  # clippy finds nothing in the cameras' holders watcher, its tests included.
  v4l2loopback-holders-passes-clippy = passesClippy "v4l2loopback-holders-passes-clippy" (
    pkgs.callPackage ./holders/package.nix { }
  );
}
