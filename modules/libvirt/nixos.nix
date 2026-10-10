# Virtual machines on QEMU and KVM, through libvirt. Every one of QEMU's
# features passes through it (PCI and SR-IOV passthrough, CPU pinning, huge
# pages, NUMA, a TPM, Secure Boot firmware, virtiofs, nested KVM), each machine
# is a plain XML document, and virsh drives the lot from a terminal, so a
# script or an agent can do anything a person can. qemu-system-* stays on the
# path for whatever libvirt won't express.
#
# virsh connects to qemu:///session by default, where machines run as Joel and
# need no privilege. qemu:///system, for bridges and passthrough, asks for his
# password through polkit. Nobody joins the libvirtd group, which would hand
# out root without one.
{ pkgs, ... }:
let
  # Every architecture QEMU emulates, so Arm and RISC-V guests run too.
  inherit (pkgs) qemu;
in
{
  virtualisation.libvirtd = {
    enable = true;
    qemu = {
      package = qemu;
      # System machines run as libvirt's own qemu account, not root.
      runAsRoot = false;
      swtpm.enable = true;
      # virtiofs, for a guest to mount a host directory at the disk's speed.
      vhostUserPackages = [ pkgs.virtiofsd ];
    };
    # Machines start only when asked, and shut down cleanly with the host.
    onBoot = "ignore";
    onShutdown = "shutdown";
  };

  environment.systemPackages = [
    # Session machines find QEMU, swtpm and passt, their network, on the path.
    qemu
    pkgs.passt
    pkgs.swtpm
    # virt-install makes a machine from the command line; the package's
    # graphical manager comes along unused.
    pkgs.virt-manager
  ];

  environment.sessionVariables.LIBVIRT_DEFAULT_URI = "qemu:///session";
}
