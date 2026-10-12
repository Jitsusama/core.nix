# What Joel's kernel is configured for, one intent at a time. Each entry is a
# Kconfig option set on top of nixpkgs's common configuration, and checks.nix
# reads the finished configuration back to prove each one held.
{ lib }:
let
  inherit (lib.kernel)
    yes
    no
    option
    freeform
    ;
in
{
  # BORE favours tasks that run in short bursts, such as the compositor and
  # the terminal, over long-running ones such as a build. From bore.patch.
  SCHED_BORE = yes;

  # Preempt the kernel anywhere, so input never waits for a kernel task to
  # yield. nixpkgs prefers lazy preemption, which waits for the next tick.
  PREEMPT = yes;
  PREEMPT_LAZY = no;

  # Tick at 1000 Hz on busy cores, and not at all on idle ones or ones
  # running a single task.
  HZ_1000 = yes;
  NO_HZ_FULL = yes;

  # Optimize across the whole kernel at link time, with Clang's ThinLTO.
  LTO_CLANG_THIN = yes;

  # Build the kernel so it can be profiled as it runs. A profile recorded on
  # the machine then lets Clang optimize the next build for what the kernel
  # really does there. Without one, this adds only debug information.
  AUTOFDO_CLANG = yes;

  # Back all of a program's memory with huge pages where possible, which
  # saves address lookups for compilers and language runtimes: Go's and
  # rustc's allocators no longer ask for them, so nixpkgs's choice, huge pages
  # only on request, leaves them out. Only programs that ask still wait for
  # memory to be compacted into huge pages, so nothing else stalls for them.
  TRANSPARENT_HUGEPAGE_ALWAYS = yes;
  TRANSPARENT_HUGEPAGE_MADVISE = no;

  # Build in what every boot loads, so it's there before anything asks for
  # it. The rest stay modules, since built in they'd cost their start-up at
  # every boot whether used or not; docs/kernel.md has the measurements.
  #
  # AES-NI's ciphers. As a module it isn't in the initrd, so the encrypted
  # disk opens on the generic XTS code instead, which reads 2.4 times slower
  # per CPU, and keeps it until the next boot.
  CRYPTO_AES_NI_INTEL = yes;

  # What mounting the EFI system partition needs. As modules, loading them
  # while every device's driver loads held the mount for up to two seconds,
  # and everything after local-fs.target with it.
  FAT_FS = yes;
  VFAT_FS = yes;
  NLS_CODEPAGE_437 = yes;
  NLS_ISO8859_1 = yes;

  # What the NixOS firewall's rules need, which it otherwise loads one
  # module at a time while the network waits for it. The rpfilter matches
  # sit inside the legacy iptables menus, so those come too.
  NETFILTER_XTABLES = yes;
  NF_TABLES = yes;
  NFT_COMPAT = yes;
  NF_CONNTRACK = yes;
  NETFILTER_XT_MATCH_CONNTRACK = yes;
  NETFILTER_XT_MATCH_PKTTYPE = yes;
  IP_NF_IPTABLES = yes;
  IP6_NF_IPTABLES = yes;
  IP_NF_MATCH_RPFILTER = yes;
  IP6_NF_MATCH_RPFILTER = yes;

  # Don't count every allocation by the line that made it. nixpkgs counts
  # from boot, and the counting's own tables then hold about 210 MiB and add a
  # hook to every slab allocation and free: turned off, fork ran about 3%
  # faster per CPU second. Nothing here reads /proc/allocinfo, and a leak hunt
  # can turn it back on. The kernel stops offering the default without it.
  MEM_ALLOC_PROFILING = no;
  MEM_ALLOC_PROFILING_ENABLED_BY_DEFAULT = option no;

  # Reboot ten seconds after a panic, rather than freeze until the power
  # button is held. The panic's record is in pstore by then.
  PANIC_TIMEOUT = freeform "10";

  # Compress modules with zstd, which the kernel decompresses about six
  # times faster than nixpkgs's xz, for about a third more disk space.
  MODULE_COMPRESS_ZSTD = yes;
  MODULE_COMPRESS_XZ = no;

  # The kernel can't build Rust with LTO while it keeps BTF type information,
  # which sched_ext and BPF tools need. No driver these machines use is
  # written in Rust, so Rust goes, with the options nixpkgs sets beside it.
  # They're optional because the kernel stops offering them without Rust.
  RUST = option no;
  DRM_PANIC_SCREEN_QR_CODE = option no;
  NOVA_CORE = option no;
  DRM_NOVA = option no;
}
