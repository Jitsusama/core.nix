# lag-watch's system half. The kernel keeps two counts lag-watch leans on
# only while its scheduler statistics are on. One is how long each thread
# was blocked, asleep on a lock, a page read back from swap or reclaim,
# which is how lag-watch sees Slack or Chrome hang without waiting for a
# CPU at all (src/kernel.rs, `blocked`). The other is the fair scheduler's
# own count of each thread's waits for a CPU, `wait_sum`, which lag-watch
# takes the lesser of with the schedstat's (src/kernel.rs, `waited`).
# Without it each wait falls back to the schedstat's alone, which counts
# some sleeps as waits of up to 4 s, and nothing says so. The kernel is
# built with them, off until asked.
{
  boot.kernel.sysctl."kernel.sched_schedstats" = 1;
}
