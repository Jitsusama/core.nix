# The Kernel

Joel's NixOS machines run a kernel core.nix builds itself: kernel.org's stable
series as nixpkgs pins it, compiled with Clang and ThinLTO for each machine's
own CPU, with the BORE scheduler and a handful of settings chosen for a
development laptop that must never feel slow. [Decision 0006][decision] says
why it's built here rather than taken from a distribution.

Everything about it is in [`modules/kernel/`][module]:

| File           | What it holds                                                        |
| -------------- | -------------------------------------------------------------------- |
| `package.nix`  | the kernel: its series, compiler, CPU, profile, patch and strictness |
| `settings.nix` | the Kconfig settings, each with the intent behind it                 |
| `nixos.nix`    | the NixOS module, with the `jitsusama.kernel` options                |

`flake.nix` offers the module as `nixosModules.kernel`, and the kernel for any
x86-64 machine as `packages.x86_64-linux.kernel`.

## What It Is

- **The series:** 7.2, at whichever point release nixpkgs pins. Taking a new
  nixpkgs takes the latest 7.2 release with its fixes.
- **The compiler:** Clang 22, linked with LLD, which link-time optimization
  needs. Not nixpkgs's default LLVM 21: its LLD, compiled with GCC 16 as
  nixpkgs compiles it, writes objects the kernel's objtool rejects.
- **The CPU:** whichever one the machine's [hardware][hw] names in
  `jitsusama.kernel.cpu`, as Clang's `-march` names it: `pantherlake` for the
  XPS 14, `znver2` for a Zen 2 laptop. So each machine gets a kernel of its own,
  scheduled and tuned for its cores. Without one, the kernel runs on any x86-64
  machine, as upstream builds it.
- **The profile:** whichever AutoFDO profile the machine recorded of the
  kernel at work, in `jitsusama.kernel.profile`. Clang uses it to lay out and
  inline the code the machine actually runs. [Profiling It](#profiling-it)
  says how to record one.

### The Patch

One patch, fetched from its author at a fixed commit with its hash, so it can
be traced to where it came from and can't change underneath the build:

| Patch        | What it adds                     | From                         |
| ------------ | -------------------------------- | ---------------------------- |
| `bore.patch` | the BORE scheduler, `SCHED_BORE` | [firelzrd/bore-scheduler][1] |

BORE is a small change to the kernel's own EEVDF scheduler: it favours tasks
that run in short bursts, such as the compositor, terminal and editor, over
ones that run flat out, such as a build. Its author releases it for every
stable series, and CachyOS has made it its default.

nixpkgs's own two patches, applied to every kernel it builds, are kept.

### The Settings

[`settings.nix`][settings] sets each option on top of nixpkgs's common
configuration, and its own setting wins where the two differ. In short:

| Intent                            | Settings                            |
| --------------------------------- | ----------------------------------- |
| favour short bursts of work       | `SCHED_BORE`                        |
| preempt the kernel anywhere       | `PREEMPT`, not `PREEMPT_LAZY`       |
| tick at 1000 Hz, or not at all    | `HZ_1000`, `NO_HZ_FULL`             |
| optimize across the whole kernel  | `LTO_CLANG_THIN`                    |
| be ready to be profiled           | `AUTOFDO_CLANG`                     |
| huge pages for every program      | `TRANSPARENT_HUGEPAGE_ALWAYS`       |
| no Rust, which LTO and BTF forbid | `RUST` and the options that need it |

nixpkgs gives huge pages only to programs that ask for them, with
`TRANSPARENT_HUGEPAGE_MADVISE`, and the ones that would gain most no longer
ask: Go's runtime and the allocator rustc uses. Giving them to every program
saves address lookups across a build. Waiting for memory to be compacted into
huge pages stays reserved for programs that ask, so nothing else stalls for
it. Omarchy runs the same.

Preemption stays switchable at boot, with `preempt=`, because nixpkgs keeps
`PREEMPT_DYNAMIC` on. The rest of what a developer needs is already in
nixpkgs's configuration: KVM, user namespaces and overlayfs for containers,
BTF, BPF, sched_ext, kprobes, uprobes and ftrace for tracing, PSI for
systemd-oomd, and multi-gen LRU for memory.

## Around It

The kernel module also turns on `intel_iommu=on`, so every device's DMA is
translated, where Intel's firmware opt-in translates only the Thunderbolt
ports. Four more modules tune the running kernel. The workstation role brings
in the first three, and the graphical role the last:

- **[`memory`][memory]:** swap to zstd-compressed RAM, kill a runaway process
  before the machine thrashes, and write to disk steadily rather than in
  stalls, so a linking build never freezes the desktop.
- **[`nix-daemon`][nix-daemon]:** builds run as batch work, so they take the
  CPU only when nothing interactive wants it, and systemd-oomd stops one that
  starves the machine of memory.
- **[`slices`][slices]:** the compositor and sound get ten times anyone
  else's share of the CPU, and a floor of memory a build can't reclaim.
- **[`perf`][perf]:** perf itself, and the settings that let Joel profile his
  own programs, kernel included, without root.

## How It's Checked

Two things catch a kernel that has drifted from what it says:

- **Strict configuration.** `package.nix` sets `ignoreConfigErrors = false`,
  so building the configuration fails when nixpkgs or `settings.nix` names an
  option this kernel doesn't offer, rather than dropping it silently.
- **The `kernel-settings-hold` check.** Kconfig also drops a setting whose
  dependencies aren't met, without complaint. The check builds the finished
  configuration and confirms every setting in `settings.nix` is in it. A
  setting marked `option` may instead be missing altogether, when the kernel
  stops offering it.

Building the configuration applies the patches, so a patch that no longer
applies fails the check too. It takes minutes, which is why CI runs it rather
than building the kernel, which takes far longer. The CPU doesn't change the
configuration, so one check covers every machine.

## Building It

```bash
nix build .#packages.x86_64-linux.kernel
```

That's the kernel for any x86-64 machine. A machine's own, for its CPU, is the
one its configuration builds, from the machine repository:

```bash
nix build .#nixosConfigurations.<machine>.config.boot.kernelPackages.kernel
```

Either builds every module nixpkgs's configuration enables, so expect it to
take an hour or so on a laptop.

## Profiling It

The kernel is always built ready to be profiled, so a machine can record what
its kernel does in a day's work and have the next build optimized for it, with
Clang's AutoFDO. It needs a CPU that records the branches it takes: Intel's
last-branch records, which Panther Lake keeps on every core, or AMD's, from Zen
3 on. A Zen 2 machine can't, so it builds without a profile.

Record while the machine does its usual work, a build included. `-c` samples
one branch in every 500009, the period the kernel's own guide suggests:

```bash
sudo perf record -e BR_INST_RETIRED.NEAR_TAKEN:k -a -N -b -c 500009 \
  -o kernel.data -- sleep 1800
```

Turn the samples into a profile against the running kernel's `vmlinux`, with
the LLVM that built it:

```bash
dev=$(nix build --no-link --print-out-paths \
  .#nixosConfigurations.<machine>.config.boot.kernelPackages.kernel.dev)
nix shell nixpkgs#llvmPackages_22.llvm --command llvm-profgen --kernel \
  --binary="$dev/vmlinux" --perfdata=kernel.data --output=kernel.afdo
```

Commit `kernel.afdo` beside the machine's configuration and set
`jitsusama.kernel.profile = ./kernel.afdo;`. Clang matches a profile to code
by function and line, so an older profile still helps after a point release,
if less; record a new one after moving to a new series.

## Changing It

- **A setting:** add it to `settings.nix` with a comment saying why. Mark it
  `option` only when some kernels won't offer it, and say which.
- **A patch:** add it to `package.nix`, fetched from its author at a commit,
  with a comment saying what it adds. It has to earn its place: a clear
  purpose, a maintainer who releases it for every stable series, and other
  distributions shipping it.
- **A machine's CPU:** set `jitsusama.kernel.cpu` in its hardware module, to a
  name from `clang --print-supported-cpus`.
- **A patch one model needs:** add it to that model's `boot.kernelPatches`,
  under the same bar; [the hardware guide][hw] has one.
- **A new series:** change `linux_7_2` in `package.nix` to the new series,
  point BORE at its patch for that series, and update the hash and this
  document. The configuration check fails until the patch applies and every
  setting holds.

## What's Left Out, and Why

- **`-O3`.** Upstream removed the option in 2022 ([a6036a41bffb][2]), and asks
  anyone bringing it back for a benchmark and disassembly showing a kernel
  loop that gains from it. Nobody has, and built here with Clang it tripped
  the kernel's own `FORTIFY_SOURCE` checks.
- **`-march=native`.** It ties the build to whichever machine runs it, so the
  same configuration builds different kernels. Naming the CPU builds the same
  kernel everywhere.
- **Rust.** The kernel can't build Rust with LTO while it keeps BTF type
  information, and BTF is what sched_ext schedulers and BPF tools read. No
  driver these machines use is written in Rust. Rust programs are unaffected.
- **Propeller.** It reorders the code within each function from a second
  profile, taken of a kernel AutoFDO already optimized. Its profile tool,
  Google's `create_llvm_prof`, isn't in nixpkgs, and AutoFDO should prove its
  worth on the XPS 14 first.
- **Turning off CPU mitigations.** The speed isn't worth the exposure.
- **An I/O scheduler.** NVMe runs without one. On the XPS 14, BFQ cut a
  desktop read's p99 from 3 ms to 0.15 ms behind a build's I/O, but lost up
  to nine tenths of the disk's throughput when uncontended; mq-deadline and
  Kyber changed nothing, nor did `io.latency`. `io.cost` needs a model of the
  disk, and the one it guesses is out by an order of magnitude.
- **The TEO idle governor.** nixpkgs doesn't build it, and nothing measured
  here says menu, the one it builds, chooses idle states badly.

[decision]: decisions/0006-build-the-kernel-here.md
[module]: ../modules/kernel/
[settings]: ../modules/kernel/settings.nix
[memory]: ../modules/memory/nixos.nix
[nix-daemon]: ../modules/nix-daemon/nixos.nix
[perf]: ../modules/perf/nixos.nix
[slices]: ../modules/slices/nixos.nix
[hw]: hardware.md
[1]: https://github.com/firelzrd/bore-scheduler
[2]: https://git.kernel.org/torvalds/c/a6036a41bffb
