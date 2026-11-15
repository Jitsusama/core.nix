# The checks of the windows daemon, which checks.nix gathers with every
# other folder's; its opening comment says what each kind of check proves.
{
  account,
  bareNixos,
  graphical,
  holds,
  lib,
  onLinux,
  passesClippy,
  pkgs,
  self,
  ...
}:
let
  daemon = pkgs.callPackage ./package.nix { };
in
# It reads niri and Linux's inotify, and builds only there.
onLinux {
  # Its tests run as it builds, over niri's events as its stream says
  # them and over a stand-in for niri's socket: each window named by its
  # program and placed as its row says it, a retitle telling only its
  # window and those it reads alike with, a focus moving only the two it
  # moved between, a resize or the launcher taking the keys nothing, a
  # window closed gone, an entry added naming its windows' program, niri
  # gone and back describing the desktop again, and an ask or kind it
  # doesn't know refused.
  windows-hears-niris-windows = daemon;
  # The daemon words the launcher's Screens from the vocabulary the shell
  # hands it, so its tests read the launcher's own here: a word renamed or
  # gone there fails this rather than blanking a row. The daemon itself
  # isn't built with it, or every change to the words would start it again.
  windows-reads-the-launchers-words = pkgs.callPackage ./package.nix {
    words = (import ../quickshell/launcher/words.nix).file;
  };
  # Every window core opens says what it holds, rather than only which
  # program it is, which reads alike among the launcher's windows and in a
  # line said about one: the account's niri, nested in the sandbox
  # (tests/nested-niri.sh), opens the account's reader on a page and a
  # kitty running the account's shell, and each title must say more than
  # its app id or any process in it (tests/every-window-says-what-it-holds.py).
  # The camera needs a device, so its real viewfinder's title is held in
  # the-viewfinder-keeps-its-own-keys instead.
  every-window-says-what-it-holds =
    let
      home = account graphical;
      shell = (account (bareNixos [ self.nixosModules.workstation ])).home-files;
      reader = lib.findFirst (package: lib.getName package == "reader") null home.home.packages;
      files = lib.filterAttrs (
        name: _:
        lib.any (folder: lib.hasPrefix folder name) [
          "kitty/"
          "niri/"
          "reader/"
        ]
      ) home.xdg.configFile;
      inherit (pkgs) mesa;
    in
    pkgs.runCommandLocal "every-window-says-what-it-holds"
      {
        nativeBuildInputs = [
          graphical.config.programs.niri.package
          pkgs.dbus
          pkgs.kitty
          pkgs.python3
          pkgs.sway-unwrapped
          pkgs.zsh
          reader
        ];
        # Everything draws in software, as the desktop's test machines do.
        __EGL_VENDOR_LIBRARY_FILENAMES = "${mesa}/share/glvnd/egl_vendor.d/50_mesa.json";
        LIBGL_DRIVERS_PATH = "${mesa}/lib/dri";
        GBM_BACKENDS_PATH = "${mesa}/lib/gbm";
        LIBGL_ALWAYS_SOFTWARE = 1;
        FONTCONFIG_FILE = pkgs.makeFontsConf { fontDirectories = graphical.config.fonts.packages; };
      }
      ''
        export HOME=$PWD XDG_CONFIG_HOME=$PWD XDG_STATE_HOME=$PWD/state SHELL=${lib.getExe pkgs.zsh}
        ${lib.concatLines (
          lib.mapAttrsToList (name: file: "install -D -m 644 ${file.source} ${lib.escapeShellArg name}") files
        )}
        ln -s ${shell}/.zshrc ${shell}/.zshenv .
        printf '# Tides\n\nWhen the water comes in.\n' > tides.md
        ${lib.getExe pkgs.bash} ${../../tests/nested-niri.sh} niri/config.kdl logs \
          python3 ${./tests/every-window-says-what-it-holds.py} "reader $PWD/tides.md" kitty
        touch $out
      '';
  # clippy finds nothing in the windows daemon, its tests included.
  windows-passes-clippy = passesClippy "windows-passes-clippy" daemon;
  # The shell brings the daemon, on the socket the shell's daemons share,
  # held by systemd whether or not the daemon runs, handed the launcher's
  # words, which its Screens are said in.
  windows-runs-beside-the-shell =
    let
      shell = {
        home-manager.sharedModules = [ self.homeModules.quickshell ];
      };
      home = account (bareNixos [ shell ]);
      service = home.systemd.user.services.windows;
      started = lib.concatStringsSep " " (lib.toList service.Service.ExecStart);
    in
    holds "windows-runs-beside-the-shell"
      "The windows daemon doesn't run beside the shell on its socket."
      (
        lib.hasInfix "%t/daemons/windows.sock" started
        && lib.hasInfix "--words /nix/store/" started
        && service.Unit.X-SwitchMethod == "restart"
        && builtins.elem "windows.socket" service.Unit.Requires
        && home.systemd.user.sockets.windows.Socket.ListenStream == "%t/daemons/windows.sock"
      );
}
