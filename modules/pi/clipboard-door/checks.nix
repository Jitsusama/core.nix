# The checks of pi-clipboard-door, the clipboard pi's fenced commands
# reach, and of the sandboxed display beside it (../fence-display), beside
# pi's own in ../checks.nix.
{
  account,
  graphical,
  holds,
  lib,
  macWorkstation,
  onLinux,
  passesClippy,
  pkgs,
  workstation,
  ...
}:
onLinux {
  # The fence's clipboard door is started by the first wl-copy or wl-paste
  # that asks, on a socket in a folder only the account can reach, with
  # its journal lines tagged as its own; its shims stand in a folder of
  # their own the fence puts on PATH; and the sandboxed display runs with
  # the graphical session at %t/pi-fence/wayland, in a folder only the
  # account reaches that outlives a restart. A Mac gets none.
  pi-clipboard-door-reaches-only-the-fence =
    let
      home = account workstation;
      mac = account macWorkstation;
      socket = home.systemd.user.sockets.pi-clipboard-door.Socket;
      door = home.systemd.user.services.pi-clipboard-door;
      display = home.systemd.user.services.pi-fence-display;
      shims = home.home.file.".local/share/pi-fence/bin".source;
    in
    holds "pi-clipboard-door-reaches-only-the-fence"
      "pi's clipboard door listens elsewhere than %t/pi-clipboard/door.sock or to more than the account, isn't started by its socket or tagged in the journal, its shims aren't linked from ~/.local/share/pi-fence/bin, the sandboxed display isn't made at %t/pi-fence/wayland with the graphical session in a folder only the account reaches and that outlives a restart, or a Mac gets any of them."
      (
        lib.head (lib.toList socket.ListenStream) == "%t/pi-clipboard/door.sock"
        && lib.head (lib.toList socket.SocketMode) == "0600"
        && lib.head (lib.toList socket.DirectoryMode) == "0700"
        && door.Unit.Requires == [ "pi-clipboard-door.socket" ]
        && lib.hasSuffix "/bin/pi-clipboard-door" (lib.head (lib.toList door.Service.ExecStart))
        && door.Service.SyslogIdentifier == "pi-clipboard-door"
        && lib.hasSuffix "/share/pi-fence/bin" "${shims}"
        && lib.hasInfix "/bin/pi-fence-display --socket %t/pi-fence/wayland" (
          lib.head (lib.toList display.Service.ExecStart)
        )
        && display.Service.RuntimeDirectory == "pi-fence"
        && display.Service.RuntimeDirectoryMode == "0700"
        && display.Service.RuntimeDirectoryPreserve == "yes"
        && display.Install.WantedBy == [ "graphical-session.target" ]
        && display.Unit.PartOf == [ "graphical-session.target" ]
        && !(mac.systemd.user.sockets ? pi-clipboard-door)
        && !(mac.home.file ? ".local/share/pi-fence/bin")
      );

  # pi-clipboard-door builds with its tests run (src/*.rs): a client is
  # answered only when the kernel puts it in a pid namespace below a
  # bubblewrap of this account in the door's own, for the session that
  # bubblewrap was started with, never the one the client claims or a
  # bubblewrap inside the fence names; a password manager's secret is
  # never read, by any of the marks KeePassXC, KDE, Bitwarden or a Mac's
  # pasteboard set; a request unread, more than 64 MiB or a type not
  # offered is refused; text is offered as every text type and pasted as
  # UTF-8 first; the shims read wl-clipboard's options as getopt does,
  # refuse any other by name and exit as wl-clipboard does; and each copy,
  # paste, listing, clearing and refusal is said in one line of a fixed
  # shape, never with what was copied.
  pi-clipboard-door-keeps-secrets = pkgs.callPackage ./package.nix { };

  # clippy finds nothing in pi-clipboard-door, its tests included.
  pi-clipboard-door-passes-clippy = passesClippy "pi-clipboard-door-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );

  # On the account's real niri, nested in the sandbox (tests/nested-niri.sh),
  # the sandboxed display offers windows' protocols but none of the
  # privileged ones niri's own socket offers; a command bubblewrap fences
  # copies and pastes both selections through the door and its shims, is
  # refused a password manager's secret and an option it doesn't take, and
  # a copy served once is served once; a command outside the fence is
  # refused; and the door's journal names the bubblewrap's session, not the
  # one the command claims, and never what was copied; and a nested niri,
  # as tools/shell-preview starts one from a fenced command, opens on the
  # sandboxed display and takes wtype's keys, which the display itself
  # refuses (../tests/pi-fence-clipboard-on-niri.sh).
  pi-fence-clipboard-holds-on-niri =
    let
      files = lib.filterAttrs (name: _: lib.hasPrefix "niri/" name) (account graphical).xdg.configFile;
      door = pkgs.callPackage ./package.nix { };
      display = pkgs.callPackage ../fence-display/package.nix { };
      inherit (pkgs) mesa;
    in
    pkgs.runCommandLocal "pi-fence-clipboard-holds-on-niri"
      {
        nativeBuildInputs = [
          graphical.config.programs.niri.package
          pkgs.bubblewrap
          pkgs.dbus
          pkgs.sway-unwrapped
          pkgs.wayland-utils
          pkgs.wtype
        ];
        # niri draws in software, as the desktop's machine does.
        __EGL_VENDOR_LIBRARY_FILENAMES = "${mesa}/share/glvnd/egl_vendor.d/50_mesa.json";
        LIBGL_DRIVERS_PATH = "${mesa}/lib/dri";
        GBM_BACKENDS_PATH = "${mesa}/lib/gbm";
        LIBGL_ALWAYS_SOFTWARE = 1;
      }
      ''
        export HOME=$PWD XDG_CONFIG_HOME=$PWD
        ${lib.concatLines (
          lib.mapAttrsToList (name: file: "install -D -m 644 ${file.source} ${lib.escapeShellArg name}") files
        )}
        ${lib.getExe pkgs.bash} ${../../../tests/nested-niri.sh} niri/config.kdl logs \
          ${lib.getExe pkgs.bash} ${../tests/pi-fence-clipboard-on-niri.sh} \
            ${lib.getExe door} ${lib.getExe display} ${door}/share/pi-fence/bin logs
        touch $out
      '';
}
