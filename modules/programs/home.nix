# The programs daemon: what's installed, each desktop entry whole, and
# what each program had open lately and has open now, held ready over
# $XDG_RUNTIME_DIR/daemons/programs.sock in the shell's daemon interface,
# for the launcher's Programs and each program's drawer and Recent
# (src/main.rs says how). The shell imports it, since those are what it
# holds. Its sources are the lists each program's module adds to
# recents.nix, given to it as a file.
{
  config,
  lib,
  osConfig ? { },
  pkgs,
  ...
}:
let
  programs = pkgs.callPackage ./package.nix { };
  sources = pkgs.writeText "programs-sources.json" (
    builtins.toJSON config.jitsusama.quickshell.launcher.recents
  );
  # The launcher's vocabulary, which the rows it serves are said in.
  words = pkgs.writeText "words.json" (builtins.toJSON config.jitsusama.quickshell.launcher.words);
in
{
  imports = [ ../quickshell/recents.nix ];

  # It isn't in background.slice: what it holds is what the launcher
  # shows as Joel opens it. It reads nothing while nobody watches, so it
  # costs nothing idle. It's part of the graphical session, whose
  # environment says where the desktop entries are and finds each
  # program's command.
  systemd.user.services.programs = {
    Unit = {
      Description = "Holds what's installed and what each program had open ready for the shell";
      PartOf = [ "graphical-session.target" ];
      After = [
        "graphical-session.target"
        "programs.socket"
      ];
      # Restarted after a switch reloads the units, it comes back as the
      # new one, reading the new entries and sources.
      X-SwitchMethod = "restart";
      Requires = [ "programs.socket" ];
    };
    Service = {
      ExecStart = "${lib.getExe programs} %t/daemons/programs.sock ${sources} ${words}";
      # The git it lists a repository's files with, as Neovim's Open a
      # File by Name steps into one (src/repositories.rs), and the
      # file-preview it keeps for drawers that preview a file
      # (src/preview.rs), and the niri and kitten that go to one of
      # kitty's tabs in Tabs and close it (src/tabs.rs).
      Environment = [
        "PROGRAMS_GIT=${lib.getExe pkgs.git}"
        "PROGRAMS_PREVIEW=${lib.getExe (pkgs.callPackage ../open-with/file-preview.nix { })}"
        "PROGRAMS_NIRI=${lib.getExe (osConfig.programs.niri.package or pkgs.niri)}"
        "PROGRAMS_KITTEN=${lib.getExe' pkgs.kitty "kitten"}"
      ];
      Restart = "on-failure";
      RestartSec = 5;
    };
    Install.WantedBy = [ "graphical-session.target" ];
  };

  # Its socket, held by systemd whether or not the daemon runs, so the
  # shell asking while it starts waits on it rather than finding none.
  systemd.user.sockets.programs = {
    Unit.Description = "Where the shell asks the programs daemon";
    Socket = {
      ListenStream = "%t/daemons/programs.sock";
      SocketMode = "0600";
      DirectoryMode = "0700";
    };
    Install.WantedBy = [ "sockets.target" ];
  };
}
