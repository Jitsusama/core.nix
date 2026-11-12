# The admin VM tests' checks, which checks.nix gathers with every other
# folder's; README.md beside this says what each test proves.
{
  self,
  onLinux,
  vmHostPkgs,
  ...
}:
let
  pkgs = vmHostPkgs;
in
onLinux {
  # Whether libvirt's daemon stops in time shows only with the daemon
  # running and its threads short of CPU, as at a busy shutdown.
  vm-libvirt-stops-quickly = import ./libvirt.nix { inherit self pkgs; };
  # Whether the kernel took a slice's disk latency shows only with the
  # account's manager starting it on a real drive.
  vm-apps-come-first = import ./slices.nix { inherit self pkgs; };
  # Whether a check's VM goes ahead of the builds shows only with a build
  # user's process in the daemon's own cgroup and the service's real rights.
  vm-checks-go-first = import ./vm-first.nix { inherit self pkgs; };
  # Whether polkit lets an account switch the machine shows only with
  # polkit, systemd and a system to switch to.
  vm-admins-switch-unasked = import ./switch-system.nix { inherit self pkgs; };
  # Whether a machine updates itself to signed main, and goes back when it
  # isn't whole, shows only with systemd, a real switch and Nix building.
  vm-updates-itself-to-signed-main = import ./update-to-main.nix {
    inherit self pkgs;
  };
  # Whether polkit lets an administrator tune the work slices shows only
  # with polkit and systemd.
  vm-admins-tune-the-work-slices = import ./work-slices.nix { inherit self pkgs; };
  # Whether polkit lets an administrator tune the kernel's memory, and
  # only to the values listed, shows only with polkit, systemd and a kernel.
  vm-admins-tune-memory = import ./memory-tune.nix { inherit self pkgs; };
  # Whether a focused window's memory comes back from swap, and nobody
  # else's, shows only with polkit, systemd, swap and a kernel to hint.
  vm-a-focused-window-reads-its-swap-back = import ./swap-in.nix { inherit self pkgs; };
  # Whether polkit asks an administrator before firmware is written, even
  # of the seat's session, shows only with polkit, logind and fwupd's rules.
  vm-firmware-asks-an-administrator = import ./firmware-asks.nix { inherit self pkgs; };
  # Whether a niri's kernel stacks are traced for its account and nobody
  # else shows only with polkit, bpftrace and a kernel to trace.
  vm-niri-is-traced-by-its-own = import ./niri-off-cpu.nix { inherit self pkgs; };
  # Whether root is given only the builders nix-offload's lines trust,
  # in files the lender's account can't write, shows only with systemd
  # running the service as its own account.
  vm-root-builds-only-where-it-trusts = import ./offload-lines.nix { inherit self pkgs; };
  # Whether a pi host's unit restarts a wedged host, and leaves one that
  # keeps crashing down, shows only with the account's own systemd running it.
  vm-a-wedged-pi-starts-again = import ./pi-hosts-recover.nix { inherit self pkgs; };
  # Whether a pi host gives back what it held after a kill -9 and a reboot
  # shows only with the account's own systemd starting it, and a boot.
  vm-pi-conversations-come-back =
    let
      # pi's build names its folder with a word only a checks.nix may say.
      folder = "pi-monorepo";
      hostedPi = import ../../../modules/pi/hosted-pi.nix { inherit pkgs folder; };
    in
    import ./pi-conversations-come-back.nix {
      inherit self pkgs hostedPi;
      packageDir = "${hostedPi}/lib/node_modules/${folder}";
    };
}
