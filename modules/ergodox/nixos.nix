# The ErgoDox EZ on Linux: its USB devices, open to whoever is at the
# machine, and a watcher that says which layer it's on.
#
# The devices are the board itself, which the shell and `ergodox` talk to
# over raw HID, and the bootloader it restarts into to be flashed. Without
# these rules only root can reach either, and home.nix's `ergodox` can't
# flash it.
#
# The watcher (ergodox-watch.rs) runs as the person at the machine, since
# the rules open the board to them alone, and writes the layer to
# keyboard-layer in their runtime directory, where the shell reads it
# (modules/quickshell/KeyboardLayer.qml). It asks the board for layers and
# never keys, so it needs nothing but the board and that file.
{ lib, pkgs, ... }:
let
  # The standard library alone, so it builds with rustc and no crates.
  watch = pkgs.runCommandCC "ergodox-watch" { nativeBuildInputs = [ pkgs.rustc ]; } ''
    mkdir -p $out/bin
    rustc --edition 2024 -C opt-level=3 -D warnings \
      -o $out/bin/ergodox-watch ${./ergodox-watch.rs}
  '';
in
{
  hardware.keyboard.zsa.enable = true;

  systemd.user.services.ergodox-watch = {
    description = "Says which layer the ErgoDox is on, for the shell";
    wantedBy = [ "default.target" ];
    serviceConfig = {
      ExecStart = lib.escapeShellArgs (
        [
          "${watch}/bin/ergodox-watch"
          "%t/keyboard-layer"
        ]
        ++ import ./layers.nix
      );
      Restart = "on-failure";
      NoNewPrivileges = true;
      PrivateNetwork = true;
      RestrictAddressFamilies = "none";
      ProtectSystem = "strict";
      ProtectHome = "read-only";
      ReadWritePaths = [ "%t" ];
      PrivateTmp = true;
      CapabilityBoundingSet = "";
      RestrictNamespaces = true;
      RestrictRealtime = true;
      LockPersonality = true;
      MemoryDenyWriteExecute = true;
      SystemCallArchitectures = "native";
      SystemCallFilter = [
        "@system-service"
        "~@privileged @resources"
      ];
      UMask = "0022";
    };
  };
}
