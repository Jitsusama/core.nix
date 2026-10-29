# The checks of pi-packages, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  account,
  clippyConfig,
  clippyLints,
  holds,
  lib,
  macWorkstation,
  onLinux,
  passesClippy,
  pkgs,
  rust-overlay,
  workstation,
  ...
}:
{
  # Each of pi's git packages is updated at a switch, before pi's settings
  # name its link, and in the background every fifteen minutes: by a user
  # timer in background.slice on Linux, which a switch doesn't restart, and
  # by a launchd agent on a Mac. Each change to what a pi says it loaded
  # starts a prune, on Linux by a path unit watching every package's
  # `loaded/` folder and on a Mac by a launchd agent watching the same.
  pi-packages-swap-in-the-background =
    let
      home = account workstation;
      mac = account macWorkstation;
      service = home.systemd.user.services.pi-packages;
      timer = home.systemd.user.timers.pi-packages.Timer;
      settings = home.home.activation.piTakesCoresSettings;
      prune = home.systemd.user.services.pi-packages-prune;
      watched = home.systemd.user.paths.pi-packages-prune.Path.PathChanged;
      harness = "${home.jitsusama.pi-packages.root}/github.com/Jitsusama/agentic-harness.pi/loaded";
    in
    holds "pi-packages-swap-in-the-background"
      "pi's packages aren't updated before pi's settings are written, or not in the background on Linux and on a Mac, or a pi's start or end doesn't remove the versions no pi runs."
      (
        service.Service.Slice == "background.slice"
        && service.Unit.X-RestartIfChanged == false
        && timer.OnUnitActiveSec == "15min"
        && home.home.activation ? piPackagesInstall
        && lib.elem "piPackagesInstall" settings.after
        && mac.launchd.agents.pi-packages.config.StartInterval == 900
        && prune.Service.Slice == "background.slice"
        && lib.hasInfix " prune --config " (lib.concatStringsSep " " (lib.toList prune.Service.ExecStart))
        && lib.elem harness watched
        && mac.launchd.agents.pi-packages-prune.config.WatchPaths != [ ]
      );
}
// onLinux {
  # clippy finds nothing in pi-packages, its tests included, which run as it
  # builds: a live process keeps its version, files unchanged, through an
  # update, and a build that fails or leaves a native module unbuilt is never
  # swapped in.
  pi-packages-passes-clippy = passesClippy "pi-packages-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );

  # A Mac asks libproc what each process uses (src/inuse/macos.rs), which no
  # Linux build compiles, so clippy reads the crate, its tests included, as
  # macOS would compile it.
  pi-packages-compiles-for-macos =
    let
      toolchain = (pkgs.extend rust-overlay.overlays.default).rust-bin.stable.latest.minimal.override {
        targets = [ "aarch64-apple-darwin" ];
        extensions = [ "clippy" ];
      };
    in
    (pkgs.callPackage ./package.nix { }).overrideAttrs {
      pname = "pi-packages-compiles-for-macos";
      CLIPPY_CONF_DIR = clippyConfig;
      buildPhase = ''
        export PATH=${toolchain}/bin:$PATH
        cargo clippy --target aarch64-apple-darwin --all-targets --offline -- \
          ${lib.escapeShellArgs clippyLints}
      '';
      doCheck = false;
      installPhase = ''
        touch $out
      '';
    };
}
