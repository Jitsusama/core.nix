# Who gets the CPU and memory first when everything wants them. systemd runs
# each account's compositor, sound, portals and message bus in its
# session.slice, and every program Joel starts, a terminal and the build in it
# included, in app.slice, a scope each. A build on every core then shares the
# CPU with Chrome or Slack scope by scope, and session.slice's tenfold weight
# gets the compositor and sound onto a CPU first: under sixteen spinning
# threads, a wakeup in session.slice waited at most 751 us at this weight
# against 1,962 us at the default.
#
# A terminal's work yields to everything else Joel runs. kitty starts what
# each window runs in a scope of its own, kitty-<pid>-<n>.scope, apart from
# the scope that draws it, and those scopes are idle: a sibling that wants a
# CPU gets it at once. With a kernel build from a terminal on every core, a
# wakeup in another app's scope waited at most 2.0 to 4.6 ms over three
# runs, against 6.7 to 18.2 ms at the default weight, and the build took the
# same 171 to 174 s. Terminal work still shares the CPU evenly with other
# terminal work, so an editor beside a build is as it was. The cost is when
# the rest saturates every core, a virtual machine's build say: a light task
# in a terminal then waited at most 7 ms, and a heavy one gets what is left.
#
# session.slice peaks at about 400 MiB, so a 1 GiB floor keeps it in memory
# however hard a build pushes. The floor is a hard one, MemoryMin: the kernel
# takes a soft one, MemoryLow, back whenever it finds nothing else to take, and
# when an evaluation filled a 61 GiB laptop it did so 215,001 times, leaving
# two thirds of niri in swap. niri then read keys up to 281 ms late, and past
# the 200 ms repeat delay a program takes a key that was let go as held. In a
# 4 GiB virtual machine with zram and a swapfile behind it, a program reading
# keys in session.slice, with 200 MiB it touches a little of on each one, read
# them at most 728 ms late under a 5 GB hog and 5.3 s under a 6 GB one with
# the soft floor, and at most 7.3 ms under either with the hard one. The
# kernel protects a slice only as far as each slice above it is protected, so
# every one on the way down claims the same floor, and none of it goes
# anywhere else.
{
  systemd.user.slices.session.sliceConfig = {
    CPUWeight = 1000;
    MemoryMin = "1G";
  };

  # A drop-in for every kitty-<pid>-<n>.scope, which kitty makes as it opens
  # a window.
  systemd.user.units."kitty-.scope" = {
    overrideStrategy = "asDropin";
    text = ''
      [Scope]
      CPUWeight=idle
    '';
  };

  systemd.slices.user.sliceConfig.MemoryMin = "1G";

  # Every account's user-<uid>.slice, which logind makes as it logs in.
  systemd.slices."user-" = {
    overrideStrategy = "asDropin";
    sliceConfig.MemoryMin = "1G";
  };

  systemd.services."user@".serviceConfig.MemoryMin = "1G";
}
