# The checks of the launcher's Accessories, the Bluetooth devices it pairs and
# connects, which the folder's checks.nix gathers with every other place's; its
# opening comment says why each place keeps its own.
{
  account,
  copied,
  graphical,
  noFonts,
  pkgs,
  qs,
  shellFolderLint,
  ...
}:
{
  # BlueZ says a new device is connected once its link is up, before it's
  # paired, so the launcher's Bluetooth would stop following a pairing early
  # and leave the device untrusted. The VM has no radio, so this drives it
  # through Quickshell itself with stand-in devices
  # (tests/bluetooth-pairing.qml), with the facts desk-facts would write
  # for a connected mouse, and how its link and some headphones' are doing,
  # with their last reads and how each is held, and the adapter's traffic
  # and version, as bluetooth-links would write it: the headphones'
  # skipping channels 22 to 39, where a Wi-Fi network on its channel 6 sits.
  # A stand-in Facts says the headphones are running low, as the devices
  # piece would. A stand-in adapter and launcher hold Nearby to looking
  # only while it's open, with devices heard as the shell's patch to
  # Quickshell says them (bluetooth-rssi-and-failures.patch).
  bluetooth-pairs-before-it-connects =
    pkgs.runCommandLocal "bluetooth-pairs-before-it-connects" { }
      ''
        mkdir -p runtime/desk launcher && chmod 700 runtime
        cat > runtime/desk/bluetooth.json <<'EOF'
        {
          "C3:9C:E3:2A:F3:6E": [
            {"name": "connection", "value": "LE, HID over GATT"},
            {"name": "buttons", "value": "5"}
          ]
        }
        EOF
        map=$(printf '1%.0s' {1..22}; printf '0%.0s' {1..18}; printf '1%.0s' {1..39})
        cat > runtime/bluetooth-links.json <<EOF
        {"at": 1761350400,
         "adapters": [{"name": "hci9", "address": "02:00:00:00:00:01", "up": true,
           "version": 14, "maker": 2, "received": 1200, "sent": 300,
           "traffic": [0, 750, 1500]}],
         "links": [
          {"address": "00:1B:66:AA:BB:CC", "rssi": 0, "quality": 230, "power": 4,
           "channels": 61, "map": "$map", "le": false,
           "held": {"central": true, "outgoing": false, "security": "secure"},
           "history": {"rssi": [-20, null, 0], "quality": [204, 230]}},
          {"address": "C3:9C:E3:2A:F3:6E", "rssi": -58, "quality": null, "power": 4,
           "channels": null, "map": null, "le": true,
           "held": {"central": false, "outgoing": true, "security": "none"}}
        ]}
        EOF
        ${copied [
          "Words.qml"
          "launcher/Accessories.qml"
          "launcher/Provider.qml"
          "launcher/words.json"
        ]}
        cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
        cp ${../../../tests/bluetooth-pairing.qml} shell.qml
        cp ${pkgs.writeText "Niri.qml" ''
          pragma Singleton
          import Quickshell
          import QtQuick
          Singleton { function spawn(command) {} }
        ''} Niri.qml
        # The headphones running low, as the devices piece would say them.
        cp ${pkgs.writeText "Facts.qml" ''
          pragma Singleton
          import Quickshell
          import QtQuick
          Singleton {
            readonly property var all: [
              { entry: { mark: "bluetooth:00:1B:66:AA:BB:CC" }, piece: { place: "bluetooth" } }
            ]
          }
        ''} Facts.qml
        export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
        export FONTCONFIG_FILE=${noFonts}
        ${shellFolderLint}
        timeout 60 ${qs} -p shell.qml
        touch $out
      '';
}
