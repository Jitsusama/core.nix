# Keeps the machine responsive when memory runs short, as it does when a big
# Rust or Go build links: memory swaps to compressed RAM rather than disk, the
# kernel kills before it thrashes, and writes reach the disk steadily rather
# than in stalls. The kernel's own documentation gives each value's reasoning:
# Documentation/admin-guide/sysctl/vm.rst and mm/multigen_lru.rst.
{ pkgs, ... }:
{
  # Swap to zstd-compressed RAM, which NixOS sizes at half of memory.
  zramSwap.enable = true;
  # Taking the swap down resets its device, which the kernel refuses while
  # anything holds it open, and udev opens it to probe it as swapoff closes
  # it. One shutdown in 18 failed that way, and a restart by hand would
  # leave the device at its old size, since the kernel refuses a new one;
  # NixOS doesn't restart it on a switch for that reason. Upstream's
  # systemd/zram-generator#226 is still open. zram-reset-waits.patch
  # retries while it's busy; take it out once zram-generator does that
  # itself.
  #
  # Patching it means CI builds it, and its tests skip only where user
  # namespaces are refused outright. GitHub's runners make one but refuse to
  # set it up, so the tests panicked there and every vm-swap run failed.
  # zram-tests-skip-cleanly.patch skips there too; the tests still run
  # wherever a namespace can be set up, this laptop included.
  services.zram-generator.package = pkgs.zram-generator.overrideAttrs (old: {
    patches = (old.patches or [ ]) ++ [
      ./zram-reset-waits.patch
      ./zram-tests-skip-cleanly.patch
    ];
  });

  boot.kernel.sysctl = {
    # Swapping to zram costs about a tenth of reading the disk, which the
    # kernel's formula makes 180: 10x + x = 200.
    "vm.swappiness" = 180;

    # zram has no seek to amortize, so read back only the page that was asked
    # for.
    "vm.page-cluster" = 0;

    # Start writing dirty pages at 64 MiB and make writers wait at 256 MiB,
    # rather than at a tenth and a fifth of memory. Those let gigabytes build
    # up, and flushing them stalls every program that syncs.
    "vm.dirty_background_bytes" = 64 * 1024 * 1024;
    "vm.dirty_bytes" = 256 * 1024 * 1024;

    # Keep directory and inode caches longer, which large source trees and
    # the tools that walk them depend on.
    "vm.vfs_cache_pressure" = 50;
  };

  # Never evict what was used in the last second: past that, the kernel's OOM
  # killer ends a program instead of the whole machine thrashing. The kernel
  # documentation's own value, from the ~100 ms lag people notice.
  systemd.tmpfiles.rules = [ "w /sys/kernel/mm/lru_gen/min_ttl_ms - - - - 1000" ];

  # When pressure lasts, systemd-oomd stops the whole runaway group, such as a
  # build, rather than whichever process the kernel picks.
  systemd.oomd.enableUserSlices = true;

  # systemd 261 watches CPU and IO pressure as well as memory, and hands each
  # service the cgroup file to watch. Its user@.service turns that off for
  # memory alone, so a user manager is handed IO pressure for every program in
  # its account, can't put its own threshold on it, and warns each time it
  # starts or reloads. It does nothing when CPU or IO pressure rises, so it
  # watches only itself, as it already does for memory. The change upstream's
  # units/user@.service.in needs:
  #
  #      MemoryPressureWatch=skip
  #     +CPUPressureWatch=skip
  #     +IOPressureWatch=skip
  systemd.services."user@".serviceConfig = {
    CPUPressureWatch = "skip";
    IOPressureWatch = "skip";
  };
}
