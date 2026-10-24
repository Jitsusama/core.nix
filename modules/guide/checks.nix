# The checks of guide, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  clippyConfig,
  clippyLints,
  lib,
  onLinux,
  passesClippy,
  pkgs,
  rust-overlay,
  ...
}:
onLinux {
  # The guide's tests run as it builds: each step proven by observing it
  # again, each parser reading what real machines printed, each stick it
  # writes read back by sfdisk and fsck.fat, and each signature compared
  # with sbsign's. Its install check then finds every program it runs at
  # the path it was built with.
  guide-proves-its-steps = pkgs.callPackage ./package.nix { };

  # clippy finds nothing in the guide, its tests included.
  guide-passes-clippy =
    (passesClippy "guide-passes-clippy" (pkgs.callPackage ./package.nix { })).overrideAttrs
      { doInstallCheck = false; };

  # A Mac writes sticks too, through its own raw disks and cache, and no
  # check runs on a Mac, so clippy reads the guide as macOS would compile it.
  guide-compiles-for-macos =
    let
      toolchain = (pkgs.extend rust-overlay.overlays.default).rust-bin.stable.latest.minimal.override {
        targets = [ "aarch64-apple-darwin" ];
        extensions = [ "clippy" ];
      };
    in
    (pkgs.callPackage ./package.nix { }).overrideAttrs {
      pname = "guide-compiles-for-macos";
      CLIPPY_CONF_DIR = clippyConfig;
      buildPhase = ''
        export PATH=${toolchain}/bin:$PATH
        cargo clippy --target aarch64-apple-darwin --all-targets --offline -- \
          ${lib.escapeShellArgs clippyLints}
      '';
      doCheck = false;
      doInstallCheck = false;
      installPhase = ''
        touch $out
      '';
    };
}
