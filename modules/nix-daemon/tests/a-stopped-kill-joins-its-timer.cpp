// The daemon's process for a connection stops its build hook with SIGTERM and a 500 ms timer
// that sends SIGKILL after it, and a stop that comes while it waits for
// the hook throws out of that wait. Stock Nix leaves the timer's thread
// running then, and the Pid's destructor, killing again, assigns a new
// thread over the joinable one, which calls std::terminate and ends the
// build mid-way. This stops a child that ignores SIGTERM the same way,
// interrupted mid-wait, and passes only if it lives through the
// destructor.
#include <nix/util/processes.hh>
#include <nix/util/signals.hh>

#include <csignal>
#include <cstdio>
#include <sys/time.h>
#include <unistd.h>

using namespace std::chrono_literals;

// A stop, marked as the daemon's own signal handling marks one.
static void stop(int)
{
    nix::setInterrupted(true);
}

int main()
{
    // Ignored before the fork, so the child ignores it from its first
    // instruction and only the timer's SIGKILL ends it.
    std::signal(SIGTERM, SIG_IGN);
    pid_t child = fork();
    if (child == 0) {
        for (;;)
            pause();
    }
    // Without SA_RESTART, so the alarm breaks the wait with EINTR.
    struct sigaction action{};
    action.sa_handler = stop;
    sigaction(SIGALRM, &action, nullptr);
    struct itimerval soon{};
    soon.it_value.tv_usec = 100000;
    {
        nix::Pid pid(child);
        pid.setKillSignal(SIGTERM);
        pid.setKillTimeout(500ms);
        setitimer(ITIMER_REAL, &soon, nullptr);
        try {
            pid.kill();
            std::fputs("The wait wasn't interrupted, so this proves nothing.\n", stderr);
            return 1;
        } catch (nix::Interrupted &) {
        }
        nix::setInterrupted(false);
    }
    std::puts("stopped mid-wait, and the process lived on");
    return 0;
}
