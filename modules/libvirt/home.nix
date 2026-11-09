# The VMs in the launcher, where the account's shell is loaded: Programs ›
# VMs, hidden until a VM exists on the session's connection or the
# system's, then each machine inside it with its state, its figures and
# what can be done to it (Machines.qml). The machine runs the daemons and
# brings virsh and virt-manager (nixos.nix); this only shows them.
#
# The program is an entry of this module's own, VMs, rather than
# virt-manager's, which the machine keeps out of the launcher: that one's
# Return would start its manager, and its name says the name the glossary
# replaces with VM. This one hides itself, so nothing lists it but the
# launcher, and only while Machines.qml says a machine exists, its Return
# stepping in to them. Its one keyword is the name the glossary replaces,
# which still finds it, and no implementation's name is typed for it.
{
  lib,
  options,
  osConfig ? { },
  pkgs,
  ...
}:
let
  # virsh from the daemon's own package where the machine runs one, so the
  # client is the version it talks to.
  libvirt = osConfig.virtualisation.libvirtd.package or pkgs.libvirt;
in
{
  # Only where the shell is: the providers option is declared without it
  # too, so Neovim, Chrome and Reader can give it their acts
  # (../quickshell/providers.nix), but a VMs entry there would list a
  # program nobody can open. The shell's surfaces option is its own.
  config = lib.optionalAttrs (options ? jitsusama.quickshell.surfaces) {
    # After Meet's and Slack's own acts, since VMs are a program's too.
    jitsusama.quickshell.launcher.providers = {
      vms = {
        order = 52;
        source = ./Machines.qml;
        virsh = lib.getExe' libvirt "virsh";
        virtManager = lib.getExe' pkgs.virt-manager "virt-manager";
        stdbuf = lib.getExe' pkgs.coreutils "stdbuf";
        program = "vms";
      };
      # The theme's mark for a VM, rather than the program mark every
      # program without a logo shares.
      applications.glyphs.vms = "mark:vm";
    };

    xdg.configFile."quickshell/launcher/Machines.js".source = ./Machines.js;

    xdg.dataFile."applications/vms.desktop".text = ''
      [Desktop Entry]
      Type=Application
      Name=VMs
      Comment=Computers run as programs on this one
      Keywords=virtual machine;virtual machines;
      Exec=${lib.getExe' pkgs.virt-manager "virt-manager"}
      Icon=virt-manager
      NoDisplay=true
    '';
  };
}
