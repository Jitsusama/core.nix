# What the harness's agent fence lets a fenced command reach of the desktop:
# the clipboard door, with the wl-copy and wl-paste that ask it, and the
# sandboxed display. Each sits in a runtime folder of its own, which the
# fence binds read-only, so a fence holding the folder still reaches a socket
# made anew, and no fenced command replaces one for another session's to
# reach. Nothing here is reached until the harness binds it in.
{ lib, pkgs, ... }:
let
  clipboardDoor = pkgs.callPackage ./clipboard-door/package.nix { };
  fenceDisplay = pkgs.callPackage ./fence-display/package.nix { };
in
{
  config = lib.mkIf pkgs.stdenv.hostPlatform.isLinux {
    # The clipboard the agent fence lets in, at %t/pi-clipboard/door.sock:
    # the door sets and reads both selections through the compositor's
    # data-control protocol, so a fenced command's copy or paste never
    # takes the focus or opens a window, answers only commands the kernel
    # puts in a fence of this account, never reads a password manager's
    # secret, and says each copy, paste and refusal in the journal with the
    # session it came from, never what was copied (clipboard-door/src).
    # Started by the first wl-copy or wl-paste that asks. The fence puts the
    # shims first on a fenced command's PATH from
    # ~/.local/share/pi-fence/bin, and keeps that folder read-only.
    systemd.user.sockets.pi-clipboard-door = {
      Unit.Description = "The clipboard pi's fenced commands reach, never a password manager's secret";
      Socket = {
        ListenStream = "%t/pi-clipboard/door.sock";
        SocketMode = "0600";
        DirectoryMode = "0700";
        Service = "pi-clipboard-door.service";
      };
      Install.WantedBy = [ "sockets.target" ];
    };
    systemd.user.services.pi-clipboard-door = {
      Unit = {
        Description = "The clipboard pi's fenced commands reach, never a password manager's secret";
        Requires = [ "pi-clipboard-door.socket" ];
        After = [ "pi-clipboard-door.socket" ];
      };
      Service = {
        ExecStart = lib.getExe clipboardDoor;
        SyslogIdentifier = "pi-clipboard-door";
      };
    };
    home.file.".local/share/pi-fence/bin".source = "${clipboardDoor}/share/pi-fence/bin";

    # The display the agent fence is to let in, at %t/pi-fence/wayland: a
    # socket niri accepts as a sandbox's through wp-security-context-v1, so
    # its clients may open windows but never reach data-control,
    # screencopy, the virtual keyboard and pointer or the other privileged
    # protocols (fence-display/src/main.rs). The harness doesn't bind it in
    # yet: niri 26.04 focuses a sandboxed client's new window as it does any
    # other's, on the focused workspace, and no window rule can tell a
    # sandboxed client from one naming the same app id. It lasts as long as
    # the graphical session, and its folder outlives a restart.
    systemd.user.services.pi-fence-display = {
      Unit = {
        Description = "The sandboxed display pi's fenced commands are to open windows on";
        PartOf = [ "graphical-session.target" ];
        After = [ "graphical-session.target" ];
      };
      Service = {
        ExecStart = "${lib.getExe fenceDisplay} --socket %t/pi-fence/wayland";
        RuntimeDirectory = "pi-fence";
        RuntimeDirectoryMode = "0700";
        RuntimeDirectoryPreserve = "yes";
        Restart = "on-failure";
        RestartSec = 2;
      };
      Install.WantedBy = [ "graphical-session.target" ];
    };
  };
}
