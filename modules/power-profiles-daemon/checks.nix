# The checks of power-profiles-daemon, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  bareNixos,
  holds,
  lib,
  pkgs,
  self,
  ...
}:
{
  # UPower waits 20 s before its action, and under load 1% of a laptop's
  # battery lasts about a minute, so acting at 2% acts after it's flat.
  batteries-leave-time-to-power-off =
    let
      inherit ((bareNixos [ self.nixosModules.power-profiles-daemon ]).config.services) upower;
    in
    holds "batteries-leave-time-to-power-off"
      "UPower acts below 5% or by sleeping, so a laptop under load goes flat first."
      (upower.enable && upower.percentageAction >= 5 && upower.criticalPowerAction == "PowerOff");

  # The power profile daemon starts whenever it's asked, without waiting for
  # the boot to finish, which waits for the network: the shell asks it for the
  # profile as it starts, so the boot's slowest network held the shell. Read
  # from the unit each machine installs, which keeps the rest of upstream's
  # and runs the daemon that machine chose, as the XPS 14 patches its own.
  the-power-profile-answers-before-the-network =
    let
      runs =
        machine:
        let
          inherit (machine.config) services jitsusama;
        in
        ''
          unit=${services.power-profiles-daemon.package}/lib/systemd/system/power-profiles-daemon.service
          if grep -E '^After=.*(multi-user|network)' "$unit"; then
            echo "The power profile daemon waits for the boot to finish." >&2
            exit 1
          fi
          grep -q '^BusName=org.freedesktop.UPower.PowerProfiles$' "$unit" ||
            { echo "The daemon's unit lost upstream's settings." >&2; exit 1; }
          grep -qxF 'ExecStart=${jitsusama.power-profiles-daemon.package}/libexec/power-profiles-daemon' "$unit" ||
            { echo "The unit doesn't run the daemon the machine chose." >&2; exit 1; }
        '';
      xps = bareNixos [ self.nixosModules.dell-xps-14-da14260 ];
      patched =
        xps.config.jitsusama.power-profiles-daemon.package.outPath != pkgs.power-profiles-daemon.outPath;
    in
    pkgs.runCommandLocal "the-power-profile-answers-before-the-network" { } ''
      ${runs (bareNixos [ self.nixosModules.power-profiles-daemon ])}
      ${runs xps}
      ${lib.optionalString (!patched) ''
        echo "The XPS 14 runs the daemon unpatched." >&2
        exit 1
      ''}
      touch $out
    '';

  # The profile follows the charger: performance on mains, balanced on
  # battery, and power saver once the battery falls below UPower's low mark,
  # staying there while the battery reads above it again, until the charger is
  # back. It acts only when that choice changes, so a profile picked by hand
  # holds, and a line from UPower that isn't a change does nothing. UPower and
  # the daemon are stand-ins: each time the service asks whether the machine
  # is on battery, UPower answers with the next of the states listed below.
  the-profile-saves-power-on-a-low-battery =
    let
      inherit ((bareNixos [ self.nixosModules.power-profiles-daemon ]).config) services systemd;
      unit = systemd.services.power-profile-follows-the-charger;
    in
    pkgs.runCommandLocal "the-profile-saves-power-on-a-low-battery" { } ''
      ${lib.optionalString (services.upower.percentageLow != 20) ''
        echo "UPower's low mark isn't the 20% the steps below expect." >&2
        exit 1
      ''}
      export STATE=$PWD
      mkdir bin
      cat > steps <<'EOF'
      false 79
      true 35
      true 20
      true 19
      true 21
      false 21
      true 15
      false 15
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
      cat > bin/powerprofilesctl <<'EOF'
      #!${pkgs.bash}/bin/bash
      case "$1" in
        list) printf '%s\n' '  performance:' '* balanced:' '  power-saver:' ;;
        set) echo "$(cat "$STATE/now"): $2" >> "$STATE/calls" ;;
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
        ]
      }
      if ${unit.serviceConfig.ExecStart or "false"}; then
        echo "The service ended without its monitor failing." >&2
        exit 1
      fi
      cat > expected <<'EOF'
      false 79: performance
      true 35: balanced
      true 19: power-saver
      false 21: performance
      true 15: power-saver
      false 15: performance
      EOF
      if [ -e errors ] || [ -s steps ] || ! diff expected calls; then
        echo "The power profile didn't follow the battery as listed." >&2
        exit 1
      fi
      touch $out
    '';
}
