# Keeps the machine responsive when memory runs short, as it does when a big
# Rust or Go build links: memory swaps to compressed RAM rather than disk,
# systemd-oomd stops a runaway group before the machine thrashes, and writes
# reach the disk steadily rather than in stalls. The kernel's own
# documentation gives each value's reasoning: Documentation/admin-guide/
# sysctl/vm.rst and mm/multigen_lru.rst.
{ pkgs, ... }:
{
  # Swap to zstd-compressed RAM, as much as memory holds. At NixOS's half,
  # a 62 GiB laptop's builds filled zram's 31 GiB and sent 0.42 of 2.02 TiB
  # swapped in a 13-hour day to the swapfile on its SSD, which wears it and
  # reads back in milliseconds rather than microseconds; a full zram sends
  # the newest pages there, not the coldest. Full, zram held its 31 GiB in
  # 9.7 GiB of memory, so all of memory's worth costs about a third of it.
  zramSwap.enable = true;
  zramSwap.memoryPercent = 100;
  # Taking the swap down resets its device, which the kernel refuses while
  # anything holds it open, and udev opens it to probe it as swapoff closes
  # it. One shutdown in 18 failed that way, and a restart by hand would
  # leave the device at its old size, since the kernel refuses a new one;
  # NixOS doesn't restart it on a switch for that reason. Upstream's
  # systemd/zram-generator#226 is still open. zram-reset-waits.patch
  # retries while it's busy; take it out once zram-generator does that
  # itself.
  services.zram-generator.package = pkgs.zram-generator.overrideAttrs (old: {
    patches = (old.patches or [ ]) ++ [
      ./zram-reset-waits.patch
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

    # Wake kswapd 2% of memory before it runs out, not 0.1%, so it frees
    # pages ahead of the builds' allocations rather than leaving them to
    # reclaim and compact for themselves, which every program then waits
    # behind. In seven interleaved five-minute rounds under agents' builds,
    # 200 against 10 took allocation stalls from 50 to 154 a second to 0 to
    # 16, slow touches of an app's memory (8 ms or more) from 19 to 2, and
    # lag-watch's marks for waits on memory from 11 to 3. kswapd then keeps
    # 1.2 to 2.5 GiB more free, two of its 2% steps at most.
    "vm.watermark_scale_factor" = 200;
  };

  # Evict what reclaim reaches however recently it was used, as the kernel
  # does by default. Keeping the last second resident instead made the OOM
  # killer end a program whenever reclaim found nothing older, and it chose
  # by badness, which a Chromium app's renderers set to 300 themselves: it
  # killed Slack four times on 26 October and once on 28 October, each time
  # with 85 to 128 GiB of swap free and up to 27.5 GB of file cache. A
  # runaway group is stopped by systemd-oomd below instead. Written rather
  # than left out, so a running kernel that kept a second learns it too.
  systemd.tmpfiles.rules = [ "w /sys/kernel/mm/lru_gen/min_ttl_ms - - - - 0" ];

  # When pressure lasts, systemd-oomd stops the whole runaway group, such as a
  # build, rather than whichever process the kernel picks.
  systemd.oomd.enableUserSlices = true;

  # Pressure alone misses memory that fills without stalling: work that
  # swaps out steadily under its own MemoryHigh, as a git repack or an
  # evaluation does, until swap and memory are both gone and the kernel's
  # OOM killer ends whichever program it picks. Once swap is 90% used and
  # less than a tenth of memory is left, systemd-oomd ends the cgroup holding
  # the most swap instead, which is the work that filled it. Fedora sets
  # this on the root slice by default; NixOS's enableRootSlice watches
  # pressure there and leaves swap unwatched.
  systemd.slices."-".sliceConfig.ManagedOOMSwap = "kill";

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
