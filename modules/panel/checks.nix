# The checks of panel, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  bareNixos,
  lib,
  pkgs,
  self,
  ...
}:
{
  # The XPS 14's panel stays fast at 30%, drops to 60 Hz once the battery falls
  # below it, stays there while the battery reads 31% again, and comes back to
  # its fastest rate on the charger. Only the panel's modes at the size it shows
  # count, and a line from UPower that isn't a change does nothing. UPower and
  # niri are stand-ins: each time the service asks whether the machine is on
  # battery, UPower answers with the next of the states listed below.
  the-panel-slows-on-a-low-battery =
    let
      xps = bareNixos [
        self.nixosModules.graphical
        self.nixosModules.dell-xps-14-da14260
      ];
      unit = xps.config.systemd.user.services.panel-refresh or null;
      started = unit != null && lib.elem "graphical-session.target" unit.wantedBy;
      mode = width: height: refresh_rate: { inherit width height refresh_rate; };
      outputs = builtins.toFile "outputs.json" (
        builtins.toJSON {
          HDMI-A-1 = {
            current_mode = 0;
            modes = [
              (mode 1920 1080 144000)
              (mode 1920 1080 60000)
            ];
          };
          eDP-1 = {
            current_mode = 0;
            modes = [
              (mode 2880 1800 120001)
              (mode 2880 1800 60000)
              (mode 2880 1800 48000)
              (mode 1920 1200 165000)
            ];
          };
        }
      );
    in
    pkgs.runCommandLocal "the-panel-slows-on-a-low-battery" { } ''
      ${lib.optionalString (!started) ''
        echo "The XPS 14 has no panel-refresh service started with the session." >&2
        exit 1
      ''}
      export STATE=$PWD
      mkdir bin
      cat > steps <<'EOF'
      false 79
      true 35
      true 30
      true 29
      true 31
      true 27
      false 27
      true 25
      EOF
      cat > bin/busctl <<'EOF'
      #!${pkgs.bash}/bin/bash
      case "$*" in
        *OnBattery*)
          if [ ! -s "$STATE/steps" ]; then
            echo "asked once too often" >> "$STATE/errors"
            exit 1
          fi
          head -n 1 "$STATE/steps" > "$STATE/now"
          tail -n +2 "$STATE/steps" > "$STATE/rest"
          mv "$STATE/rest" "$STATE/steps"
          read -r battery _ < "$STATE/now"
          echo "b $battery"
          ;;
        *Percentage*)
          read -r _ percent < "$STATE/now"
          echo "d $percent.6"
          ;;
      esac
      EOF
      cat > bin/niri <<'EOF'
      #!${pkgs.bash}/bin/bash
      case "$1 $2" in
        "msg --json") cat ${outputs} ;;
        "msg output") echo "$(cat "$STATE/now"): $3 $4 $5" >> "$STATE/calls" ;;
      esac
      EOF
      cat > bin/gdbus <<'EOF'
      #!${pkgs.bash}/bin/bash
      changed="org.freedesktop.DBus.Properties.PropertiesChanged"
      echo "The name org.freedesktop.UPower is owned by :1.12"
      for _ in 1 2 3 4 5 6 7; do
        echo "/org/freedesktop/UPower/devices/DisplayDevice: $changed ('org.freedesktop.UPower.Device', ...)"
      done
      EOF
      chmod +x bin/*
      export PATH=$PWD/bin:${
        lib.makeBinPath [
          pkgs.coreutils
          pkgs.diffutils
          pkgs.gnugrep
          pkgs.jq
        ]
      }
      if ${unit.serviceConfig.ExecStart or "false"}; then
        echo "The service ended without its monitor failing." >&2
        exit 1
      fi
      cat > expected <<'EOF'
      true 29: eDP-1 mode 2880x1800@60.000
      false 27: eDP-1 mode 2880x1800@120.001
      true 25: eDP-1 mode 2880x1800@60.000
      EOF
      if [ -e errors ] || [ -s steps ] || ! diff expected calls; then
        echo "The panel's rate didn't follow the battery as listed." >&2
        exit 1
      fi
      touch $out
    '';
}
