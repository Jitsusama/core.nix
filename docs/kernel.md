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

- **The series:** 7.2, at whichever point release nixpkgs pins, or a newer
  one `package.nix` names in `stable` while nixpkgs catches up. Taking a new
  nixpkgs takes the latest 7.2 release with its fixes, and `stable` stops
  applying once nixpkgs's release is as new. It names 7.2.12, newer than the
  pinned nixpkgs's 7.2.9, for its fixes to what the XPS 14 runs. 7.2.12 adds
  the speaker amplifiers' OT25 DAI, without which 7.2.11's sound card never
  binds. 7.2.11 brought these: the Intel
  Bluetooth driver resetting the chip for a clean start and bounding its
  transmit ring, races in Bluetooth's RFCOMM, SCO and ISO sockets, the Xe
  graphics driver's register waits and shrinker, device-mapper reading freed
  memory on resume, Btrfs's extended attributes, and locked-memory leaks in
  mremap and io_uring. 7.2.10 brought fixes to Bluetooth's core, the SDCA
  codec's power management, the TPM driver and the slab allocator.
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

### The Patches

BORE is fetched from its author at a fixed commit with its hash, so it can be
traced to where it came from and can't change underneath the build. The others
are fixes written here, kept until the kernel has its own:

| Patch                     | What it does                   | From               |
| ------------------------- | ------------------------------ | ------------------ |
| `bore.patch`              | the BORE scheduler             | [BORE's author][1] |
| `irq-resend-nested.patch` | no bad IRQ trap on resume      | written here       |
| `edp-sdp-caps.patch`      | a single eDP modeset at boot   | written here       |
| `cfg80211-ssid-without-country.patch` | the joined network's name | written here; [linux-next][3] since |

BORE is a small change to the kernel's own EEVDF scheduler: it favours tasks
that run in short bursts, such as the compositor, terminal and editor, over
ones that run flat out, such as a build. Its author releases it for every
stable series, and CachyOS has made it its default.

nixpkgs's own two patches, applied to every kernel it builds, are kept.

The IRQ fix is for interrupts that run in their parent's thread. When one
arrives while it's disabled, enabling it resends it by retriggering its
parent. An SDCA codec's interrupt, as on the XPS 14, nests in its SoundWire
master's, which is itself a nested thread with no handler of its own, so after
a resume the resend ran `handle_bad_irq()` and logged "unexpected IRQ trap".
The patch climbs to the first parent that has a handler, and does nothing when
none has, as the kernel already does for a nested interrupt with no parent.
The patch file is the diff to send upstream.

The eDP fix is for the screen going dark twice as the graphics driver starts.
The driver commits a display state while it loads, before anything has asked
the panel what it supports, and the console's first commit comes after. A
panel that can vary its refresh rate takes the Adaptive-Sync SDP, but the
driver only learns so in its detect hook, so the first state leaves the SDP
out and the console's puts it in. The driver can't change that without a full
modeset, which on the XPS 14's panel powers it off for its half-second power
cycle and costs about 1.1 s, and the card isn't ready for the desktop until it
ends. The patch reads the SDP caps when the panel is first set up, beside the
compression caps read there already, so the console's commit matches and
keeps the mode. One modeset is left: the firmware drives the panel at 8 bits
with Display Stream Compression, and the driver forces a modeset over any
firmware state that uses it, a FIXME it keeps until it can read that state
back.

The Wi-Fi fix is for networkd not knowing which network a card joined.
wpa_supplicant joins through its own authentication and association steps
rather than the kernel's connect command, and for that kind of join the
kernel only records the network's name at the end of the function that
finishes it, after passing the access point's country to the regulatory
code. That part returns early when the access point names no country, so
the name was never recorded, and networkd couldn't match a saved network's
file on it, or tell it had moved to another network and needed a new
address. The patch records the name first. Most access points name their
country, but hostapd doesn't unless it's told to.

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
| build in what every boot loads    | AES-NI, vfat, the firewall's set    |
| decompress modules quickly        | `MODULE_COMPRESS_ZSTD`, not xz      |
| no allocation profiler            | `MEM_ALLOC_PROFILING` off           |
| reboot after a panic              | `PANIC_TIMEOUT` of 10 s             |
| a panic screen in the theme       | `DRM_PANIC_*_COLOR` from its roles  |

nixpkgs gives huge pages only to programs that ask for them, with
`TRANSPARENT_HUGEPAGE_MADVISE`, and the ones that would gain most no longer
ask: Go's runtime and the allocator rustc uses. Giving them to every program
saves address lookups across a build. Waiting for memory to be compacted into
huge pages stays reserved for programs that ask, so nothing else stalls for
it. Omarchy runs the same.

nixpkgs builds in memory allocation profiling and starts it at boot, so the
kernel counts every allocation by the line that made it, for
`/proc/allocinfo`. On the XPS 14 its own tables held 210 MiB: 129.5 MB of
page extensions and 91.1 MB of slab object vectors. Over four interleaved
rounds of stress-ng, turned off at run time, fork ran 0.2 to 12.9% faster
per CPU second, about 3% in the middle, and dentry churn about 2%. Turning
it off at boot, with `sysctl.vm.mem_profiling=never`, frees the page
extensions but still leaves a tag slot in every slab object's vector, so
it comes out of the build. A leak hunt can build it back in.

A panic otherwise waits forever, and on the XPS 14 [both crashes][crashes]
froze it until the power button was held. Ten seconds later the record is
already in pstore, so rebooting loses nothing.

The ten seconds before it show the panic's log, drawn by the kernel over
whatever was on the screen. It's in the theme's text on its background, the
colours of the account the machine signs in, as the console and the boot
wear them, rather than the kernel's white on black. The kernel takes its
colours only as it's built, so changing the theme builds the kernel again.
A machine with no such account keeps the kernel's.

Preemption stays switchable at boot, with `preempt=`, because nixpkgs keeps
`PREEMPT_DYNAMIC` on. The rest of what a developer needs is already in
nixpkgs's configuration: KVM, user namespaces and overlayfs for containers,
BTF, BPF, sched_ext, kprobes, uprobes and ftrace for tracing, PSI for
systemd-oomd, and multi-gen LRU for memory.

### Built In or a Module

A module costs a load at every boot that uses it: something runs modprobe,
which decompresses and links it, and whatever needs it waits. Built in, it's
there from the start, but its start-up runs on one CPU before `/init`, used or
not, and it can't be reloaded. Its code runs as fast either way: module code
sits on 2 MiB pages too, and instruction page walks cost 0.10 to 0.43% of the
XPS 14's cycles for all its code together, under fio on btrfs over dm-crypt.

So a module is built in here when every boot loads it, it loads no firmware,
and nobody needs to reload it. Measured on the XPS 14:

- **AES-NI's ciphers, built in.** As a module, `aesni_intel` isn't in the
  initrd, so the encrypted disk opened on the generic XTS code and kept it
  until the next boot. Through dm-crypt over a zero device, that read 7.2 to
  9.0 GB/s per CPU, against 18.1 to 22.3 GB/s for AES-NI's own XTS. Reading
  the disk in 1 MiB blocks, the generic code took a fifth of the kernel's
  cycles.
- **vfat and its two code pages, built in.** Mounting the EFI system
  partition took 0.19 to 2.03 s over four boots, and 5 to 44 ms with the
  modules already loaded. Everything after `local-fs.target` waits for it.
- **The firewall's netfilter set, built in.** `nf_tables`, conntrack,
  `nft_ct` and the fib lookups the reverse-path filter makes. On iptables,
  the firewall took 0.8 to 3.5 s at boot loading its set as its rules asked,
  and 0.4 s to reload with it loaded. On nftables, its first start on a
  running machine loaded these five modules and took 117 ms, against 82 ms
  with them loaded. The network waits for it.
- **iptables' matches, modules.** Nothing at boot loads them now the
  firewall, libvirt and Podman are on nftables. What still speaks iptables,
  such as k3s's network policies, loads them through `nft_compat`.
- **btrfs, dm-crypt and NVMe, modules.** They load in the initrd while the
  disk's PIN is typed, so the boot doesn't wait for them, and btrfs's 116 ms
  start-up, raid6's benchmark included, would only delay the PIN prompt.
  Built in, ThinLTO could inline across them and the kernel, but btrfs took
  at most a quarter of the kernel's cycles, under random writes, so the gain
  is too small to measure.
- **KVM, a module.** Loading `kvm_intel` takes 80 ms. Built in, every boot
  would wait for it before `/init`, and it couldn't be reloaded with other
  parameters.
- **Drivers that load firmware, modules.** The GPU, Wi-Fi, sound, camera and
  Bluetooth drivers. Built in, they'd ask for firmware before the root
  filesystem holds it, and a stuck device couldn't be reset by reloading.
- **What's loaded on demand, modules.** Overlayfs, FUSE, TUN, vhost, bridges,
  WireGuard and other filesystems. Built in, every boot pays their start-up:
  xfs's alone took 235 ms.

Modules are compressed with zstd rather than nixpkgs's xz. Loading xfs's
9.8 MB took 288 to 314 ms from xz, 236 to 255 ms from zstd and 232 to 242 ms
uncompressed. Decompressing all 288 modules a boot loads took about 1 s from
xz and 0.3 s from zstd, for 16.3 MB against 11.9 MB.

## Around It

The kernel module also turns on `intel_iommu=on`, so every device's DMA is
translated, where Intel's firmware opt-in translates only the Thunderbolt
ports. Five more modules tune the running kernel. The workstation role brings
in the first four, and the graphical role the last:

- **[`btrfs`][btrfs]:** the kernel gives nearly empty chunks back once
  unallocated space runs short, so metadata always has room to grow and a
  write never fails while space is free.
- **[`memory`][memory]:** swap to zstd-compressed RAM, kill a runaway process
  before the machine thrashes, and write to disk steadily rather than in
  stalls, so a linking build never freezes the desktop.
- **[`nix-daemon`][nix-daemon]:** builds run as batch work in a slice of
  their own, at a tenth of the weight of the user's programs, so they take
  the CPU only when nothing interactive wants it, systemd-oomd stops one
  that starves the machine of memory, and the daemon collects garbage before
  a build fills the disk.
- **[`perf`][perf]:** perf itself, and the settings that let Joel profile his
  own programs, kernel included, without root.
- **[`slices`][slices]:** the compositor and sound get ten times anyone
  else's share of the CPU, and a floor of memory a build can't reclaim; the
  work pi and terminals start, virtual machines and containers run behind
  what Joel uses; a call's audio may run in real time; and the work's disk
  writes give way while a save of his waits on the disk.

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
- **A fix written here:** put it beside `package.nix` as a patch whose header
  says what it fixes and why, so the file can go upstream as it is, and drop
  it once the kernel has its own.
- **A newer point release:** set `stable` in `package.nix` to its version
  and the tarball's hash, from kernel.org's `sha256sums.asc`, converted with
  `nix hash convert --hash-algo sha256`. The configuration check applies
  every patch to it.
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
  Kyber changed nothing. `io.cost` needs a model of the disk, and the one it
  guesses is out by an order of magnitude. Behind a build's writes, a
  synced write waits out the drive's cache, and an `io.latency` target,
  which needs no scheduler, holds the build back for it (the
  [`slices`][slices] module).
- **Building in every module a boot loads.** See
  [Built In or a Module](#built-in-or-a-module): only the ones the boot waits
  for earn it.
- **4096-byte sectors on an encrypted disk.** The XPS 14's disk was
  formatted with 512-byte sectors, so a 4 KiB read is eight XTS operations.
  Through dm-crypt over a zero device, 4096-byte sectors gave 9 to 20% more
  random 4 KiB operations a second, but from 820,000 to 1,050,000 already,
  two and a half to three times what the disk itself does. Not worth
  re-encrypting a disk for; free to choose when formatting one.
- **The TEO idle governor.** nixpkgs doesn't build it, and nothing measured
  here says menu, the one it builds, chooses idle states badly.
- **A sched_ext scheduler.** The kernel can load one, but the two meant for
  interactive use, scx_lavd and scx_bpfland, both lost to BORE's EEVDF on the
  XPS 14. With a kernel build on every core, another app's wakeups waited
  221 us at the 99th percentile under EEVDF, 3,409 us under scx_lavd and
  226 us under scx_bpfland, which made the compositor and sound wait 674 us
  against 106 us. The build took 158.8 s, against 164.8 s and 167.5 s. Idle
  terminal scopes, from the [`slices`][slices] module, did better than
  either: 68 us.

[decision]: decisions/0006-build-the-kernel-here.md
[module]: ../modules/kernel/
[settings]: ../modules/kernel/settings.nix
[btrfs]: ../modules/btrfs/nixos.nix
[memory]: ../modules/memory/nixos.nix
[nix-daemon]: ../modules/nix-daemon/nixos.nix
[perf]: ../modules/perf/nixos.nix
[slices]: ../modules/slices/nixos.nix
[hw]: hardware.md
[crashes]: hardware.md#the-two-crashes
[1]: https://github.com/firelzrd/bore-scheduler
[2]: https://git.kernel.org/torvalds/c/a6036a41bffb
[3]: https://git.kernel.org/pub/scm/linux/kernel/git/next/linux-next.git/commit/?id=577f690ce5dc
