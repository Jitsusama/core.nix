# What Joel's kernel is configured for, one intent at a time. Each entry is a
# Kconfig option set on top of nixpkgs's common configuration, and checks.nix
# reads the finished configuration back to prove each one held.
{ lib }:
let
  inherit (lib.kernel)
    yes
    no
    option
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

  # Back all of a program's memory with huge pages where possible, which
  # saves address lookups for compilers and language runtimes: Go's and
  # rustc's allocators no longer ask for them, so nixpkgs's choice, huge pages
  # only on request, leaves them out. Only programs that ask still wait for
  # memory to be compacted into huge pages, so nothing else stalls for them.
  TRANSPARENT_HUGEPAGE_ALWAYS = yes;
  TRANSPARENT_HUGEPAGE_MADVISE = no;

  # The kernel can't build Rust with LTO while it keeps BTF type information,
  # which sched_ext and BPF tools need. No driver these machines use is
  # written in Rust, so Rust goes, with the options nixpkgs sets beside it.
  # They're optional because the kernel stops offering them without Rust.
  RUST = option no;
  DRM_PANIC_SCREEN_QR_CODE = option no;
  NOVA_CORE = option no;
  DRM_NOVA = option no;
}
