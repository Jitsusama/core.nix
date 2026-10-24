# The checks of disk-guard, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  bareDarwin,
  bareNixos,
  clippyConfig,
  clippyLints,
  holds,
  lib,
  macWorkstation,
  onLinux,
  passesClippy,
  pkgs,
  rust-overlay,
  self,
  workstation,
  ...
}:
{
  # The disk guard's own memory ceiling never stops it, and both guards
  # yield: the account's to whatever the account runs, the machine's to what
  # every account runs, at the builds' weight. Past the ceiling the kernel
  # kills in the unit, and systemd's default then stops the whole unit, a
  # guard that was only waiting on a command among it.
  disk-guard-stays-in-the-background =
    let
      inherit (workstation.config) systemd;
      system = systemd.services.disk-guard.serviceConfig;
      user = (account workstation).systemd.user.services.disk-guard.Service;
    in
    holds "disk-guard-stays-in-the-background"
      "A command overrunning the disk guard's memory would stop the guard, or a guard runs outside a background.slice at the builds' weight."
      (
        system.OOMPolicy == "continue"
        && user.OOMPolicy == "continue"
        && user.Slice == "background.slice"
        && (system.Slice or null) == "background.slice"
        && (systemd.slices.background.sliceConfig.CPUWeight or null) == 10
      );

  # A Mac's guards run under launchd, the machine's as a daemon and the
  # account's as an agent in the background, with a Mac's own paths and
  # nothing to confine a linking run to, since a Mac has no scopes.
  the-mac-guards-its-disk =
    let
      daemon = macWorkstation.config.launchd.daemons.disk-guard.serviceConfig;
      agent = (account macWorkstation).launchd.agents.disk-guard;
      system = macWorkstation.config.jitsusama.disk-guard.settings;
      user = (account macWorkstation).jitsusama.disk-guard.settings;
      # Homebrew comes with the graphical role.
      brewing =
        (account (bareDarwin [
          self.darwinModules.workstation
          self.darwinModules.graphical
        ])).jitsusama.disk-guard.settings.sources;
    in
    holds "the-mac-guards-its-disk"
      "A Mac's guards don't run under launchd, their config names Linux's paths or systemd-run, a cache is looked for where a Mac's programs don't keep it, or a Mac's own sources are missing."
      (
        builtins.elemAt daemon.ProgramArguments 1 == "system"
        && daemon.KeepAlive
        && agent.enable
        && builtins.elemAt agent.config.ProgramArguments 1 == "user"
        && agent.config.ProcessType == "Background"
        && system.dedupe.systemd_run == null
        && system.sources.tmp.path == "/private/tmp"
        && user.sources.trash.path == "/Users/someone/.Trash"
        && user.sources.derived-data.kind == "cache"
        && user.sources.go-build.path == "/Users/someone/Library/Caches/go-build"
        && user.sources.sccache.path == "/Users/someone/Library/Caches/Mozilla.sccache"
        && user.sources.pnpm.path == "/Users/someone/Library/pnpm/store"
        && user.sources.go-mod.path == "/Users/someone/go/pkg/mod"
        && user.sources.lima.path == "/Users/someone/Library/Caches/lima"
        && !(user.sources ? homebrew)
        && builtins.elemAt brewing.homebrew.command 1 == "cleanup"
        && !((account macWorkstation).systemd.user.services ? disk-guard)
      );

  # A machine's own programs, which its own sources run, come first on the
  # guard's PATH, under systemd and launchd alike, with core's git after.
  disk-guard-finds-a-machines-programs-first =
    let
      first = [ pkgs.hello ];
      given =
        machine:
        account (machine [
          { home-manager.sharedModules = [ { jitsusama.disk-guard.path = first; } ]; }
        ]);
      onLinux =
        (given (modules: bareNixos ([ self.nixosModules.workstation ] ++ modules)))
        .systemd.user.services.disk-guard.Service.Environment;
      onMac =
        (given (modules: bareDarwin ([ self.darwinModules.workstation ] ++ modules)))
        .launchd.agents.disk-guard.config.EnvironmentVariables.PATH;
      leads = path: lib.hasPrefix "${pkgs.hello}/bin:" path && lib.hasInfix "-git-" path;
    in
    holds "disk-guard-finds-a-machines-programs-first"
      "A machine's programs for the disk guard aren't first on its PATH, or core's git is gone from it."
      (leads (lib.removePrefix "PATH=" (builtins.head onLinux)) && leads onMac);
}
// onLinux {
  # The disk guard reads the configs a workstation and its account get:
  # each is refused at build by the guard's own check-config, which builds
  # the guard and runs its scenarios on the way.
  disk-guard-accepts-its-configuration =
    pkgs.runCommandLocal "disk-guard-accepts-its-configuration"
      {
        system = workstation.config.environment.etc."disk-guard/config.json".source;
        user = (account workstation).xdg.configFile."disk-guard/config.json".source;
      }
      ''
        touch $out
      '';

  # clippy finds nothing in the guard, its scenarios included.
  disk-guard-passes-clippy = passesClippy "disk-guard-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );

  # The macOS guard builds from the same crate and no check runs on a Mac,
  # so clippy reads the crate, its scenarios included, as macOS would compile it.
  disk-guard-compiles-for-macos =
    let
      toolchain = (pkgs.extend rust-overlay.overlays.default).rust-bin.stable.latest.minimal.override {
        targets = [ "aarch64-apple-darwin" ];
        extensions = [ "clippy" ];
      };
    in
    (pkgs.callPackage ./package.nix { }).overrideAttrs {
      pname = "disk-guard-compiles-for-macos";
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
