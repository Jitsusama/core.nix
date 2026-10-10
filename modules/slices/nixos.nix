# Who gets the CPU and memory first when everything wants them. systemd runs
# each account's compositor, sound, portals and message bus in its
# session.slice, and every program Joel starts, a terminal and the build in it
# included, in app.slice, a scope each. A build on every core then shares the
# CPU with Chrome or Slack scope by scope, and session.slice's tenfold weight
# gets the compositor and sound onto a CPU first: under sixteen spinning
# threads, a wakeup in session.slice waited at most 751 us at this weight
# against 1,962 us at the default.
#
# session.slice holds about 350 MiB, so a 1 GiB floor keeps it in memory
# however hard a build pushes. The kernel protects a slice only as far as each
# slice above it is protected, so every one on the way down claims the same
# floor, and none of it goes anywhere else.
{
  systemd.user.slices.session.sliceConfig = {
    CPUWeight = 1000;
    MemoryLow = "1G";
  };

  systemd.slices.user.sliceConfig.MemoryLow = "1G";

  # Every account's user-<uid>.slice, which logind makes as it logs in.
  systemd.slices."user-" = {
    overrideStrategy = "asDropin";
    sliceConfig.MemoryLow = "1G";
  };

  systemd.services."user@".serviceConfig.MemoryLow = "1G";
}
