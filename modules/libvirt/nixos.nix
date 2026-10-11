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
#
# Neither daemon lets QEMU set up io_uring. Since 10.2, QEMU's main loop runs
# on io_uring when it can, and it loses wakeups: a firmware's TPM or CD-ROM
# command never completes, and the guest sits out a 30 to 90 second timeout.
# On a Core Ultra laptop on Linux 7.2, with QEMU 11.1, 9 of 10 UEFI boots
# with a TPM stalled; with io_uring refused, 16 of 16 booted in about six
# seconds, as 5 of 5 did on QEMU 10.1. Refused, QEMU falls back to epoll.
# This comes out once QEMU fixes
# https://gitlab.com/qemu-project/qemu/-/issues/4581.
{ config, pkgs, ... }:
let
  # Every architecture QEMU emulates, so Arm and RISC-V guests run too.
  inherit (pkgs) qemu;

  # Every process the daemon starts, QEMU among them, inherits the filter.
  refuseIoUring = {
    SystemCallFilter = [ "~io_uring_setup" ];
    SystemCallErrorNumber = "ENOSYS";
  };
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

  systemd.services.libvirtd.serviceConfig = refuseIoUring;

  # Session machines get a daemon of their own, started by its socket, which
  # is where virsh looks before starting one itself. A daemon virsh starts
  # would run in the cgroup of whatever terminal ran it, and its machines
  # would share that terminal's fate. Here they run in this service, and
  # stopping it leaves them running, as libvirt's system daemons do.
  systemd.user.sockets.virtqemud = {
    description = "libvirt QEMU daemon socket, for session machines";
    wantedBy = [ "sockets.target" ];
    listenStreams = [ "%t/libvirt/virtqemud-sock" ];
    socketConfig = {
      SocketMode = "0600";
      RemoveOnStop = true;
    };
  };

  systemd.user.services.virtqemud = {
    description = "libvirt QEMU daemon, for session machines";
    requires = [ "virtqemud.socket" ];
    after = [ "virtqemud.socket" ];
    # The filter makes a user service give up gaining privilege, so a session
    # machine can't use qemu-bridge-helper. Bridges are qemu:///system's job;
    # session machines use passt.
    serviceConfig = refuseIoUring // {
      Type = "notify";
      # It exits two minutes after its last machine and client are gone.
      ExecStart = "${config.virtualisation.libvirtd.package}/sbin/virtqemud --timeout 120";
      KillMode = "process";
      Restart = "on-failure";
    };
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
