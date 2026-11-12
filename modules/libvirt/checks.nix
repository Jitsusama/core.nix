# The checks of libvirt, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  bareNixos,
  holds,
  lib,
  onLinux,
  pkgs,
  self,
  shellFolderLint,
  workstation,
  ...
}:
{
  # Session machines join virbr0 without a password, which needs the bridge
  # helper to allow it and the default network to be up from boot.
  session-machines-have-a-bridge =
    let
      machine = bareNixos [ self.nixosModules.libvirt ];
      inherit (machine.config) systemd virtualisation;
      network = systemd.services.libvirt-default-network;
    in
    holds "session-machines-have-a-bridge"
      "Session machines can't join virbr0: the bridge helper doesn't allow it, or nothing starts libvirt's default network at boot."
      (
        lib.elem "virbr0" virtualisation.libvirtd.allowedBridges
        && lib.elem "multi-user.target" network.wantedBy
        && lib.hasInfix "net-start default" network.script
      );
}
# The launcher's VMs are the Linux desktop's shell's, which a Mac never runs.
// onLinux {
  # A workstation with a desktop gives its account's launcher the VMs: the
  # provider, reading through the daemon's own virsh, the library beside
  # it, and the program's entry, hidden, which the launcher lists only
  # while a VM exists. A workstation without the shell gets none of it,
  # so nothing on it lists a program nobody can open. It may still hold
  # the providers option, which Neovim's, Chrome's and Reader's modules
  # declare so they can give the launcher their acts without the shell.
  vms-reach-the-launcher =
    let
      machine = bareNixos [
        self.nixosModules.workstation
        self.nixosModules.graphical
      ];
      home = account machine;
      bare = account workstation;
      provider = home.jitsusama.quickshell.launcher.providers.vms or { };
      daemon = machine.config.virtualisation.libvirtd.package;
      entry = home.xdg.dataFile."applications/vms.desktop".text or "";
    in
    holds "vms-reach-the-launcher"
      "The workstation's launcher lacks its VMs, or one without the shell gets them."
      (
        (provider.source or null) == ./Machines.qml
        && (provider.virsh or "") == lib.getExe' daemon "virsh"
        && (provider.program or "") == "vms"
        && home.xdg.configFile ? "quickshell/launcher/Machines.js"
        && lib.hasInfix "\nNoDisplay=true\n" entry
        && !(bare.xdg.dataFile ? "applications/vms.desktop")
        && !((bare.jitsusama.quickshell.launcher.providers or { }) ? vms)
      );

  # How Programs › VMs reads what virsh says, its lists, the one batch
  # asking dominfo of every name, every figure dominfo gives and why any
  # is missing, each state's word, what can be done in each state and
  # what virsh is told for it, the order and count it shows, and an event
  # heard, are Machines.js's, run in Qt's own engine over the launcher's
  # own words, beside the shell's formatter it says sizes with and what
  # that imports (tests/machines.qml).
  vms-read-as-virsh-says =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "vms-read-as-virsh-says" { } ''
      cp ${./tests/machines.qml} machines.qml
      cp ${./Machines.js} Machines.js
      cp ${../quickshell/launcher/HomeNetwork.js} HomeNetwork.js
      cp ${../quickshell/launcher/Order.js} Order.js
      cp ${../quickshell/launcher/Numbers.js} Numbers.js
      cp ${../quickshell/launcher/Interfaces.js} Interfaces.js
      cp ${(import ../quickshell/launcher/words.nix).file} words.json
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${pkgs.makeFontsConf { fontDirectories = [ ]; }}
      # The case reads the words beside it, which Qt's engine reads from a
      # file only when told it may.
      export QML_XHR_ALLOW_FILE_READ=1
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml machines.qml
      touch $out
    '';
}
