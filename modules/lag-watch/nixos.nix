# lag-watch's system half. The kernel keeps how long each thread was
# blocked, asleep on a lock, a page read back from swap or reclaim, only
# while its scheduler statistics are on, and that's how lag-watch sees
# Slack or Chrome hang without waiting for a CPU at all (src/kernel.rs,
# `blocked`). Its wait for a CPU leans on them too: the fair scheduler's
# own count of it is kept only while they're on, and without it lag-watch
# falls back to the schedstat alone, which counts some sleeps as waits
# (src/kernel.rs, `Sched::waited_since`). The kernel is built with them,
# off until asked.
{
  boot.kernel.sysctl."kernel.sched_schedstats" = 1;
}
