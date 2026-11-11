# Keeps the machine responsive when memory runs short, as it does when a big
# Rust or Go build links: memory swaps to a compressed pool in RAM, and only
# its coldest pages on to the disk, systemd-oomd stops a runaway group before the machine thrashes, and writes
# reach the disk steadily rather than in stalls. The kernel's own
# documentation gives each value's reasoning: Documentation/admin-guide/
# sysctl/vm.rst and mm/multigen_lru.rst.
{ pkgs, ... }:
let
  # A unit that sets one of the kernel's memory settings to its instance.
  tune = what: command: {
    description = "Set the kernel's ${what} to %i";
    serviceConfig = {
      Type = "oneshot";
      ExecStart = command;
    };
  };
  write = file: "${pkgs.bash}/bin/bash -c 'echo %i > ${file}'";
  space = "${pkgs.procps}/bin/sysctl -w vm.watermark_scale_factor=%i";
  swappiness = "${pkgs.procps}/bin/sysctl -w vm.swappiness=%i";
  pageCluster = "${pkgs.procps}/bin/sysctl -w vm.page-cluster=%i";
  hugepages = "/sys/kernel/mm/transparent_hugepage/enabled";
  lruGen = "/sys/kernel/mm/lru_gen/enabled";
in
{
  # Swap through a zstd-compressed pool in memory, held to a fifth of it,
  # in front of the swap file: what doesn't fit, the pool's coldest pages,
  # goes on to the disk. zram at all of memory held what it swapped in
  # memory it took as it went, charged to no group: from 4 to 6 November a
  # 62 GiB laptop had 40 kills by the kernel's own OOM killer, every one
  # with swap free (39 GiB of 126 in use on average, 106 at the worst) and
  # zram's pages holding 13 to 16 GB of memory beside the slices' floors,
  # so reclaim gave up with 13 GB of file cache unreclaimed. The pool can't
  # grow past its fifth, and its shrinker writes back the coldest pages as
  # memory runs short, which zram, full, did with the newest instead.
  boot.kernelParams = [
    "zswap.enabled=1"
    "zswap.compressor=zstd"
    "zswap.max_pool_percent=20"
    "zswap.shrinker_enabled=1"
  ];

  boot.kernel.sysctl = {
    # A swapped page read back from the pool costs about a tenth of reading
    # the disk, which the kernel's formula makes 180: 10x + x = 200. Those
    # that reach the disk are the coldest, read back least.
    "vm.swappiness" = 180;

    # The pool has no seek to amortize, so read back only the page that was
    # asked for.
    "vm.page-cluster" = 0;

    # Start writing dirty pages at 64 MiB and make writers wait at 256 MiB,
    # rather than at a tenth and a fifth of memory. Those let gigabytes build
    # up, and flushing them stalls every program that syncs.
    "vm.dirty_background_bytes" = 64 * 1024 * 1024;
    "vm.dirty_bytes" = 256 * 1024 * 1024;

    # Keep directory and inode caches longer, which large source trees and
    # the tools that walk them depend on.
    "vm.vfs_cache_pressure" = 50;

    # Wake kswapd 0.25% of memory before it runs out, so it frees pages
    # ahead of the builds' allocations rather than leaving them to reclaim
    # for themselves, which every program then waits behind. 0.1% woke it
    # too late: against it, 2% took allocation stalls from 50 to 154 a
    # second to 0 to 16. But kswapd must free up to the high watermark, twice
    # the spacing over the minimum, before it may rest, and under builds it
    # fails sixteen times running and gives up. Since 7.2 only free memory
    # over that high watermark brings it back, while programs reclaiming for
    # themselves keep it near the minimum, so the wider the spacing, the
    # longer it stays given up. In eight interleaved four-minute rounds, the
    # five under real pressure, 0.5% against 2% took those give-ups from half
    # the time to 3%, stalls from 430 a second to 61 (the worst one in twenty
    # from 3,410 to 264), and every program's full pressure from 4.6 to 3.2,
    # while apps' own stayed at 0.06 to 0.07. On 10 November, with 83 agents'
    # sessions open and a load of 95, three interleaved rounds of 100 s each
    # left kswapd given up 100% of the time at 1%, 79% at 0.5% and 40% at
    # 0.25%, with allocation stalls at 2,767, 2,504 and 1,407 a second.
    "vm.watermark_scale_factor" = 25;

    # Never raise those watermarks after a fragmentation event. The boost
    # is a share of the high watermark, 150% by default, so at the spacing
    # above it lifted the floor every allocation must keep free from 64 MB
    # to 3.9 GB. Free memory then sat on that floor, so every program that
    # allocated, Chrome, Slack and the compositor among them, reclaimed for
    # itself: on 30 October, programs' own reclaim scanned three pages for
    # every one kswapd did, memory stalled every task 12.7% of a minute,
    # and reclaim that couldn't free 3.9 GB in time had the OOM killer end
    # Slack with 34.7 GB of file cache and 123 GB of swap free.
    "vm.watermark_boost_factor" = 0;
  };

  # Evict what reclaim reaches however recently it was used, as the kernel
  # does by default. Keeping the last second resident instead made the OOM
  # killer end a program whenever reclaim found nothing older, and it chose
  # by badness, which a Chromium app's renderers set to 300 themselves: it
  # killed Slack four times on 26 October and once on 28 October, each time
  # with 85 to 128 GiB of swap free and up to 27.5 GB of file cache. A
  # runaway group is stopped by systemd-oomd below instead. Written rather
  # than left out, so a running kernel that kept a second learns it too.
  #
  # Give huge pages only to memory a program asks to have them for, not to
  # every allocation that could take one, as the kernel is built to. Under
  # the same load proof, twenty builds at most with the line full, every
  # allocation taking one left kswapd given up 22% of the time, 479 stalls
  # a second and apps' memory fully stalled 0.75% of the time, up to 4.4%,
  # with seven missed turns of sound. Only those asked for took none of
  # the first, 9 stalls a second, and 0.04%, up to 0.54%, with none missed.
  systemd.tmpfiles.rules = [
    "w /sys/kernel/mm/lru_gen/min_ttl_ms - - - - 0"
    "w ${hugepages} - - - - madvise"
  ];

  # The machine's administrators, agents included since they work as Joel's
  # account, may change five of the kernel's memory settings for as long as
  # the machine runs without a password (memory-tune.rules), so an A/B of
  # them under load waits for no one. On 30 October kswapd gave up on the
  # whole machine under builds, and the settings that might have caused it,
  # the watermarks' spacing, transparent huge pages and the multi-gen LRU,
  # could each be tried only by a switch; so could swappiness, once every
  # kill that day came with half of memory in clean cache, and so could how
  # many neighbouring pages a swap-in reads with the one asked for, once a
  # pi window coming back read its heap from disk a fault at a time on 10
  # November. Each unit writes its instance and only a value the rules list:
  # `systemctl start memory-watermarks@100`, `memory-hugepages@madvise`,
  # `memory-lru-gen@n`, `memory-swappiness@60` or `memory-page-cluster@3`
  # (2^3 pages at most). The most one can do is
  # make reclaim worse until it's put back, never reach another setting. A
  # switch that changes the sysctls writes them back; otherwise start the
  # unit again with the value this file sets. At boot huge pages return to
  # this file's choice, and the LRU to the kernel's own.
  security.polkit.enable = true;
  security.polkit.extraConfig = builtins.readFile ./memory-tune.rules;
  systemd.services."memory-watermarks@" = tune "watermark spacing" space;
  systemd.services."memory-hugepages@" = tune "transparent huge pages" (write hugepages);
  systemd.services."memory-lru-gen@" = tune "multi-gen LRU" (write lruGen);
  systemd.services."memory-swappiness@" = tune "swappiness" swappiness;
  systemd.services."memory-page-cluster@" = tune "swap read-ahead" pageCluster;

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
