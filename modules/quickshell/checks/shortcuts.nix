# The checks of the launcher's shortcuts and keyboards, which the folder's
# checks.nix gathers with every other place's; its opening comment says why each
# place keeps its own.
{
  account,
  bareNixos,
  graphical,
  lib,
  machine,
  noFonts,
  pkgs,
  qs,
  self,
  shellFolderLint,
  singleton,
  ...
}:
{
  # Shortcuts groups every layer's chords by what each acts on, as
  # tests/key-groups.qml holds launcher/KeyGroups.js to an account's own
  # binds, Neovim's, pi's, Chrome's and Slack's among them, in Qt's own
  # engine, each group named in the account's vocabulary; and steps through them in levels, as
  # tests/key-map.qml drives launcher/KeyMap.qml through Quickshell itself
  # over the same account's shortcuts, with niri stood in for. The picture
  # viewer is given a key of its own here, one no viewer would use, so its
  # layer is there to lead over its window whatever keys it has yet.
  the-launcher-groups-every-shortcut =
    let
      qt = pkgs.qt6.qtdeclarative;
      home = account (bareNixos [
        self.nixosModules.graphical
        {
          home-manager.users.someone = {
            imports = [
              self.homeModules.chrome
              self.homeModules.neovim
              self.homeModules.pi
              self.homeModules.slack
            ];
            jitsusama.keys.swayimg = [
              {
                chord = "Ctrl+Alt+F12";
                does = "Stand in for the viewer's own keys";
              }
            ];
          };
        }
      ]);
      fields = map (b: {
        inherit (b)
          chord
          keys
          does
          when
          run
          group
          ;
      });
      layers = pkgs.writeText "layers.json" (
        builtins.toJSON (lib.mapAttrs (_: fields) home.jitsusama.keys)
      );
      files = home.xdg.configFile;
      words = pkgs.writeText "words.json" files."quickshell/launcher/words.json".text;
      niri = pkgs.writeText "Niri.qml" ''
        pragma Singleton
        import Quickshell
        import QtQuick
        Singleton {
            property var spawned: []
            function spawn(command) { spawned = command; }
        }
      '';
    in
    pkgs.runCommandLocal "the-launcher-groups-every-shortcut" { } ''
      export QML_XHR_ALLOW_FILE_READ=1
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      mkdir groups shell
      cd $TMPDIR/groups
      cp ${../../../tests/key-groups.qml} key-groups.qml
      cp ${../launcher/KeyGroups.js} KeyGroups.js
      cp ${../launcher/KeyLayers.js} KeyLayers.js
      cp ${layers} layers.json
      cp ${words} words.json
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml ${qt}/bin/qml key-groups.qml
      cd $TMPDIR/shell
      mkdir -p runtime launcher && chmod 700 runtime
      cp ${../../../tests/key-map.qml} shell.qml
      cp ${../Fuzzy.js} Fuzzy.js
      cp ${../launcher/Provider.qml} launcher/Provider.qml
      cp ${../ProgramGlyphs.js} ProgramGlyphs.js
      cp ${../launcher/Asking.js} launcher/Asking.js
      cp ${../launcher/Numbers.js} launcher/Numbers.js
      cp ${../launcher/KeyMap.qml} launcher/KeyMap.qml
      cp ${../launcher/KeyGroups.js} launcher/KeyGroups.js
      cp ${../launcher/KeyLayers.js} launcher/KeyLayers.js
      cp ${files."quickshell/Theme.qml".source} Theme.qml
      cp ${files."quickshell/Shortcuts.qml".source} Shortcuts.qml
      cp ${../Words.qml} Words.qml
      cp ${words} launcher/words.json
      cp ${niri} Niri.qml
      export XDG_RUNTIME_DIR=$TMPDIR/shell/runtime
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';

  # The keyboard's backlight is heard when its firmware changes it, over a
  # folder laid out as the kernel's lights are, through Quickshell itself.
  # Kept in Nix because each line lays a light, copies a file Nix gives it
  # beside the cases, or sets what Quickshell runs them with.
  the-keyboard-light-is-heard = pkgs.runCommandLocal "the-keyboard-light-is-heard" { } ''
    mkdir -p runtime/leds/test::kbd_backlight runtime/leds/input0::capslock && chmod 700 runtime
    echo 3 > runtime/leds/test::kbd_backlight/max_brightness
    touch runtime/leds/test::kbd_backlight/brightness_hw_changed
    cp ${../Lights.qml} Lights.qml
    cp ${../Daemon.qml} Daemon.qml
    cp ${../Hearing.js} Hearing.js
    cp ${../../../tests/lights.qml} shell.qml
    export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
    export FONTCONFIG_FILE=${noFonts}
    ${shellFolderLint}
    timeout 60 ${qs} -p shell.qml
    touch $out
  '';

  # The launcher's Keyboard collection finds the ErgoDox's keys and actions
  # and shows what it finds on the layout, and the board's parts among the
  # keyboards plugged in hold its Layer and Lights, as tests/keyboard.qml
  # drives them through Quickshell itself, over the firmware's own
  # description of the board, locked on Numb, with niri, the shortcuts and
  # the shell's lights stood in for. Unplugged, as the kernel's list and
  # keyboard-lights' say, the laptop's own layout, keyd's, takes its place.
  # The machine daemon tells the keyboards and the layer from its files,
  # under a root of its own.
  the-launcher-finds-the-keyboards-keys =
    let
      firmware = import ../../ergodox/firmware.nix pkgs {
        firmware = self.inputs.qmk;
        inherit (self.inputs) lufa printf;
      };
      niri = singleton "Niri.qml" ''
        property var spawned: []
        function spawn(command) { spawned = command; }
      '';
      # No caps lock and no laptop backlight, as on a desk with the board
      # alone.
      lights = singleton "Lights.qml" ''
        property bool caps: false
        property string leds: ""
        property string backlight: ""
        signal turned
      '';
      # No shortcuts, and no key faces: the boards here count nothing.
      shortcuts = singleton "Shortcuts.qml" ''
        property var actions: []
        property var faces: ({})
      '';
      # The laptop's own layout, as keyd's module describes it.
      laptop =
        pkgs.writeText "laptop.json"
          (import ../../keyd/layout.nix { inherit lib; }).environment.etc."keyboard/layout.json".text;
    in
    pkgs.runCommandLocal "the-launcher-finds-the-keyboards-keys" { } ''
      mkdir -p runtime/daemons config/ergodox launcher && chmod 700 runtime
      mkdir -p root/proc/bus/input root/run/keyboard-lights root/run/user/1000
      echo "Numb locked" > root/run/user/1000/keyboard-layer
      printf '1\n9\n' > root/run/keyboard-lights/keyboards
      cp ${firmware}/shell.json config/ergodox/layout.json
      cp ${laptop} laptop.json
      cp ${../ConnectedKeyboards.qml} ConnectedKeyboards.qml
      cp ${../Daemon.qml} Daemon.qml
      cp ${../Hearing.js} Hearing.js
      cp ${../../../tests/keyboard.qml} shell.qml
      cp ${../KeyboardLayout.qml} KeyboardLayout.qml
      cp ${../KeyboardLayer.qml} KeyboardLayer.qml
      cp ${../Fuzzy.js} Fuzzy.js
      cp ${../launcher/Provider.qml} launcher/Provider.qml
      cp ${../ProgramGlyphs.js} ProgramGlyphs.js
      cp ${../launcher/Asking.js} launcher/Asking.js
      cp ${../launcher/Numbers.js} launcher/Numbers.js
      cp ${../launcher/Keyboard.qml} launcher/Keyboard.qml
      cp ${../launcher/Keyboards.qml} launcher/Keyboards.qml
      cp ${../launcher/Keyboards.js} launcher/Keyboards.js
      cp ${../launcher/KeyCounts.js} launcher/KeyCounts.js
      cp ${../launcher/Order.js} launcher/Order.js
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${../Words.qml} Words.qml
      cp ${../launcher/words.json} launcher/words.json
      cp ${niri} Niri.qml
      cp ${shortcuts} Shortcuts.qml
      cp ${lights} Lights.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime XDG_CONFIG_HOME=$TMPDIR/config
      export MACHINE_ROOT=$TMPDIR/root LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      XDG_RUNTIME_DIR=/run/user/1000 \
        ${lib.getExe machine} "$TMPDIR/runtime/daemons/machine.sock" "$MACHINE_ROOT" &
      daemon=$!
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      kill "$daemon"
      touch $out
    '';
}
