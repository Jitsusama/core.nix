# Virtual machines on QEMU and KVM, through libvirt. Every one of QEMU's
# features passes through it (PCI and SR-IOV passthrough, CPU pinning, huge
# pages, NUMA, a TPM, Secure Boot firmware, virtiofs, nested KVM), each machine
# is a plain XML document, and virsh drives the lot from a terminal, so a
# script or an agent can do anything a person can. qemu-system-* stays on the
# path for whatever libvirt won't express.
#
# virsh connects to qemu:///session by default, where machines run as Joel and
# need no privilege. They join libvirt's NAT network through QEMU's setuid
# bridge helper, with --network bridge=virbr0, for an address the host and
# other machines reach, DHCP and DNS. qemu:///system, for passthrough and raw
# disks, asks for his password through polkit, and polkit keeps the answer
# for the process that asked, not the login, so every virsh run asks again:
# put a batch in one virsh, its commands split by semicolons. Nobody joins
# the libvirtd group, which would hand out root without one.
#
# QEMU is refused io_uring. Since 10.2, QEMU's main loop runs
# on io_uring when it can, and it loses wakeups: a firmware's TPM or CD-ROM
# command never completes, and the guest sits out a 30 to 90 second timeout.
# On a Core Ultra laptop on Linux 7.2, with QEMU 11.1, 9 of 10 UEFI boots
# with a TPM stalled; with io_uring refused, 16 of 16 booted in about six
# seconds, as 5 of 5 did on QEMU 10.1. Refused, QEMU falls back to epoll.
# The refusal wraps each QEMU binary rather than filtering the daemons: a
# filter on a user service also stops it gaining privilege, and virtiofsd
# needs newuidmap's to map the guest's users. This comes out once QEMU fixes
# https://gitlab.com/qemu-project/qemu/-/issues/4581.
{ config, pkgs, ... }:
let
  # libvirt 12.7.0's device driver joins its udev thread at shutdown while
  # holding the lock that thread needs to see it should stop. When the
  # thread loses that race, both wait until the daemon gives up 30 seconds
  # later and exits as failed, and the machine's shutdown waits with them.
  # The fix is upstream from 12.8.0, so this comes out with that release.
  joinUnlocked = pkgs.fetchpatch {
    name = "libvirt-nodedev-join-unlocked.patch";
    url = "https://gitlab.com/libvirt/libvirt/-/commit/0c053ffd2c44ef0820886ab79c1f812775b2cd15.patch";
    hash = "sha256-aiSUJmQaMZu3vGGzPUK34rUjRQlyB3EyUpF2FkyGZiA=";
  };
  libvirt =
    if pkgs.libvirt.version == "12.7.0" then
      pkgs.libvirt.overrideAttrs (old: {
        patches = (old.patches or [ ]) ++ [ joinUnlocked ];
      })
    else
      pkgs.libvirt;

  refuseIoUring = pkgs.runCommandCC "refuse-io-uring" { buildInputs = [ pkgs.libseccomp ]; } ''
    mkdir -p $out/bin
    $CC -O2 -Wall -Werror -o $out/bin/refuse-io-uring ${./refuse-io-uring.c} -lseccomp
  '';

  # Every architecture QEMU emulates, so Arm and RISC-V guests run too, each
  # emulator run through the refusal.
  qemu = pkgs.symlinkJoin {
    name = "qemu-${pkgs.qemu.version}";
    inherit (pkgs.qemu) version;
    paths = [ pkgs.qemu ];
    postBuild = ''
      for emulator in $out/bin/qemu-system-* $out/bin/qemu-kvm; do
        [ -e "$emulator" ] || continue
        real=$(readlink -f "$emulator")
        rm "$emulator"
        printf '#!%s\nexec %s %s "$@"\n' ${pkgs.runtimeShell} \
          ${refuseIoUring}/bin/refuse-io-uring "$real" > "$emulator"
        chmod +x "$emulator"
      done
      # libvirt's module finds the firmware by this package's own path.
      for descriptor in $out/share/qemu/firmware/*.json; do
        real=$(readlink -f "$descriptor")
        rm "$descriptor"
        substitute "$real" "$descriptor" --replace-quiet ${pkgs.qemu} $out
      done
    '';
  };
in
{
  virtualisation.libvirtd = {
    enable = true;
    package = libvirt;
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
    # A unit's path is its own, not the session's, so the daemon is handed
    # what it looks for there: QEMU, swtpm, passt, and the setuid newuidmap
    # virtiofsd maps the guest's users with.
    path = [
      qemu
      pkgs.passt
      pkgs.swtpm
      "/run/wrappers"
    ];
    serviceConfig = {
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

  # The bridge session machines join, virbr0, exists only while libvirt's
  # default network runs, and nothing starts it but a system client. Started
  # once at boot, off the login's path; the network outlives the daemon.
  systemd.services.libvirt-default-network = {
    description = "libvirt's default NAT network, for session machines";
    requires = [ "libvirtd.service" ];
    after = [ "libvirtd.service" ];
    wantedBy = [ "multi-user.target" ];
    path = [ config.virtualisation.libvirtd.package ];
    serviceConfig = {
      Type = "oneshot";
      RemainAfterExit = true;
    };
    script = ''
      virsh --connect qemu:///system net-info default | grep -q '^Active: *yes' ||
        virsh --connect qemu:///system net-start default
    '';
  };
}
