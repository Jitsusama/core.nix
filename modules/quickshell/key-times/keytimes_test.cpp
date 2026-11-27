// What KeyTimes says a key was pressed at, for a press the compositor
// timed and for a repeat Qt made of a held key, which carries the first
// press's time (keytimes.cpp). Each event is sent as Qt delivers one, so
// it passes the application's filters, and read while it's handled, as
// the launcher reads it. And how long the thread ran and waited for a
// CPU, read while it spins alone and then beside another thread held to
// the same CPU. And that a WallTimer ends on its time, once a start, from
// its latest start, and not at all once stopped. And that the thread's
// major page faults are read, leaving out its minor ones. It exits 0
// once all hold, and otherwise 1, saying which didn't.
#include "keytimes.h"
#include "walltimer.h"

#include <QEventLoop>
#include <QGuiApplication>
#include <QKeyEvent>
#include <QTimer>
#include <atomic>
#include <cstdint>
#include <cstdio>
#include <ctime>
#include <pthread.h>
#include <sched.h>
#include <sys/mman.h>
#include <sys/resource.h>
#include <thread>
#include <unistd.h>

namespace {

double monotonicNow() {
    timespec now{};
    clock_gettime(CLOCK_MONOTONIC, &now);
    return static_cast<double>(now.tv_sec) * 1000.0 + static_cast<double>(now.tv_nsec) / 1e6;
}

// How long before now the first press of a held key went down, as a key
// held for most of a second has.
constexpr double HELD_MS = 600;
// How far from what it should read a time may be, for the moments between
// stamping an event and reading what KeyTimes made of it.
constexpr double SLACK_MS = 50;

// How long ago KeyTimes says an arrow was pressed, sent stamped as going
// down HELD_MS ago, as a press or as a repeat. It's read before the event
// loop turns, which lets the press go, as the launcher's handler reads it.
double pressedAgo(const KeyTimes &times, QObject &target, bool repeat) {
    const double now = monotonicNow();
    QKeyEvent press(QEvent::KeyPress, Qt::Key_Down, Qt::NoModifier, QString(), repeat);
    press.setTimestamp(static_cast<std::uint32_t>(static_cast<std::uint64_t>(now - HELD_MS)));
    QCoreApplication::sendEvent(&target, &press);
    return monotonicNow() - times.pressed();
}

bool expect(const char *what, bool holds, double ago) {
    if (!holds)
        std::fprintf(stderr, "FAIL %s: read as pressed %.0f ms ago\n", what, ago);
    return holds;
}

// How long the thread spins, by the monotonic clock, for each reading.
constexpr double SPUN_MS = 200;

// How much more the thread's clocks say after a spin than before it, and
// how long the spin took by the monotonic clock, which is past SPUN_MS
// when the thread is held off as it ends.
struct Spin {
    double ran;
    double waited;
    double took;
};

// Keeps the calling thread busy for SPUN_MS, and gives how much more
// each of its clocks says after than before.
Spin spun(const KeyTimes &times) {
    const double ranBefore = times.ran();
    const double waitedBefore = times.waited();
    const double began = monotonicNow();
    const double until = began + SPUN_MS;
    while (monotonicNow() < until) {
    }
    const double ran = times.ran() - ranBefore;
    const double waited = times.waited() - waitedBefore;
    return {ran, waited, monotonicNow() - began};
}

// Holds a thread to one CPU, so two held to it take turns.
void holdTo(pthread_t thread, int cpu) {
    cpu_set_t one;
    CPU_ZERO(&one);
    CPU_SET(cpu, &one);
    pthread_setaffinity_np(thread, sizeof one, &one);
}

bool expectMs(const char *what, bool holds, double ms) {
    if (!holds)
        std::fprintf(stderr, "FAIL %s: read %.0f ms over %.0f ms spun\n", what, ms, SPUN_MS);
    return holds;
}

// What the thread's clocks say while it spins, then beside another thread
// spinning on its CPU, which the scheduler takes turns with. A spinning
// thread is only running or waiting for its CPU, so the two together
// cover most of the spin, and never more than it took. How they share it
// isn't held: on a builder other builds run on the same CPU, and a spin
// held to it by itself ran 78 ms of 200 there. Only most, since time a
// virtual machine's host takes is neither. Beside the other it waits
// about half of what it's given, whatever else the CPU carries, so an
// eighth of the spin leaves room for what a host or other builds take.
bool threadTimed(const KeyTimes &times) {
    const int cpu = sched_getcpu();
    holdTo(pthread_self(), cpu);
    const Spin alone = spun(times);
    const double covered = alone.ran + alone.waited;
    std::atomic<bool> going{true};
    std::thread other([&going] {
        while (going.load(std::memory_order_relaxed)) {
        }
    });
    holdTo(other.native_handle(), cpu);
    const Spin beside = spun(times);
    going = false;
    other.join();
    const bool ran = expectMs("a thread spinning is said to have run", alone.ran > 0, alone.ran);
    const bool whole = expectMs("a thread spinning is said to have run or waited most of it",
                                covered > alone.took / 2 && covered <= alone.took + SLACK_MS,
                                covered);
    const bool waited = expectMs("a thread taking turns with another is said to have waited",
                                 beside.waited > SPUN_MS / 8 && beside.waited < beside.took,
                                 beside.waited);
    return ran && whole && waited;
}

// How many fresh pages the thread touches, each a minor fault: a page
// never touched is handed out zeroed, with nothing to read in.
constexpr long TOUCHED_PAGES = 1024;

// The calling thread's minor faults so far, or -1 where they can't be read.
long minorFaults() {
    rusage usage{};
    if (getrusage(RUSAGE_THREAD, &usage) != 0)
        return -1;
    return usage.ru_minflt;
}

// Touches TOUCHED_PAGES fresh pages, and gives the minor faults the thread
// took doing it, or -1 where the pages couldn't be had. Huge pages are
// refused for them, as a kernel giving every program huge pages (core's
// own, modules/kernel/settings.nix) would fault them in 2 MiB at a time,
// a few faults for all of them.
long touchedFresh() {
    const long page = sysconf(_SC_PAGESIZE);
    const auto length = static_cast<std::size_t>(page * TOUCHED_PAGES);
    void *mapped = mmap(nullptr, length, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS,
                        -1, 0);
    if (mapped == MAP_FAILED)
        return -1;
    madvise(mapped, length, MADV_NOHUGEPAGE);
    const long before = minorFaults();
    auto *bytes = static_cast<volatile char *>(mapped);
    for (long at = 0; at < TOUCHED_PAGES; at++)
        bytes[at * page] = 1;
    const long after = minorFaults();
    munmap(mapped, length);
    return before < 0 || after < 0 ? -1 : after - before;
}

// The thread's faults are read, are never fewer later, and are the major
// ones alone: the touches take at least a thousand minor faults, which is
// checked rather than assumed, and don't raise the count a thousand, as a
// count of every fault would. That they're the calling thread's alone, and
// that a major one is counted at all, rests on getrusage's RUSAGE_THREAD
// and ru_majflt and isn't shown here: a major fault comes to order only
// from a file whose pages can be dropped, which a build's own folder may
// not hold, so a case for it could only skip. Where the build can't show
// the touches faulting a page at a time, it says so rather than blaming
// faulted().
bool faultsCounted(const KeyTimes &times) {
    const double before = times.faulted();
    const long minor = touchedFresh();
    const double after = times.faulted();
    if (minor < TOUCHED_PAGES) {
        std::fprintf(stderr, "FAIL the build shows fresh pages faulting a page at a time: %ld"
                     " minor faults over %ld pages, -1 where they couldn't be had\n", minor,
                     TOUCHED_PAGES);
        return false;
    }
    const bool holds = before >= 0 && after >= before && after - before < TOUCHED_PAGES;
    if (!holds)
        std::fprintf(stderr, "FAIL a thread's major faults are read: %.0f, then %.0f over %ld"
                     " fresh pages taking %ld minor faults\n", before, after, TOUCHED_PAGES,
                     minor);
    return holds;
}

// The launcher's bound on a key's hold for its answers (Launcher.qml's
// answersWaitedFor). There's no
// render loop here to hold QML's own Timer back, so this holds the type to
// its word rather than telling it from that one, which walltimer.h's
// sources and the keys lab do.
constexpr int HELD_FOR_MS = 8;
// How long a try waits for it to end at all. Only its lower bounds are
// held tight, since load can make a timer late but never early: this test
// failed once under a gate's load with a ceiling of 12 ms (2026-11-17), and
// a flake here fails the machine's Quickshell build.
constexpr int ENDS_WITHIN_MS = 200;
// Tries at the bound, spaced apart so one burst of load can't spoil them
// all; a restart is held to its word only in a try whose second start came
// before the first end, and none such is said rather than failed.
constexpr int TRIES = 10;
constexpr int TRIED_APART_MS = 20;

bool expectTimer(const char *what, bool holds, const char *said) {
    if (!holds)
        std::fprintf(stderr, "FAIL a wall timer %s: %s\n", what, said);
    return holds;
}

// Turns the event loop for `ms`.
void turnFor(int ms) {
    QEventLoop loop;
    QTimer::singleShot(ms, &loop, &QEventLoop::quit);
    loop.exec();
}

// How long after it's started `timer` says it's triggered, or -1 when it
// doesn't within `within` ms.
double endedAfter(WallTimer &timer, int within) {
    QEventLoop loop;
    double ended = -1;
    const double began = monotonicNow();
    const auto heard = QObject::connect(&timer, &WallTimer::triggered, &loop, [&] {
        ended = monotonicNow() - began;
        loop.quit();
    });
    QTimer::singleShot(within, &loop, &QEventLoop::quit);
    timer.restart();
    loop.exec();
    QObject::disconnect(heard);
    return ended;
}

bool endsOnTime(WallTimer &timer) {
    double soonest = -1;
    bool early = false;
    for (int tried = 0; tried < TRIES; ++tried) {
        const double ended = endedAfter(timer, ENDS_WITHIN_MS);
        early = early || (ended >= 0 && ended < HELD_FOR_MS - 1);
        if (ended >= 0 && (soonest < 0 || ended < soonest))
            soonest = ended;
        turnFor(TRIED_APART_MS);
    }
    char said[96];
    if (soonest < 0)
        std::snprintf(said, sizeof said, "never ended in %d tries", TRIES);
    else
        std::snprintf(said, sizeof said, "soonest %.1f ms of %d%s", soonest, HELD_FOR_MS,
                      early ? ", one before its time" : "");
    return expectTimer("ends on its time", soonest >= 0 && !early &&
                                               soonest < ENDS_WITHIN_MS,
                       said);
}

// How one try of a start, and a start again before it ends, went: how many
// times it ended, and how long after the second start it first did. A try
// whose second start came too late to be before the end is no try at all,
// since the builder's load, not the timer, decided it.
struct Restarted {
    bool tried;
    int ends;
    double ended;
};

Restarted restartedOnce(WallTimer &timer) {
    double again = -1;
    double ended = -1;
    int ends = 0;
    const auto heard = QObject::connect(&timer, &WallTimer::triggered, [&] {
        if (++ends == 1)
            ended = monotonicNow();
    });
    timer.restart();
    QTimer::singleShot(HELD_FOR_MS / 2, &timer, [&] {
        if (ends == 0) {
            again = monotonicNow();
            timer.restart();
        }
    });
    turnFor(HELD_FOR_MS * 6);
    QObject::disconnect(heard);
    return {again >= 0, ends, again >= 0 && ended >= 0 ? ended - again : -1};
}

// Started again before it ends, it ends its interval from then, and once.
bool restartedEndsLater(WallTimer &timer) {
    Restarted last{false, 0, -1};
    for (int tried = 0; tried < TRIES && !last.tried; ++tried) {
        last = restartedOnce(timer);
        turnFor(TRIED_APART_MS);
    }
    if (!last.tried) {
        std::fprintf(stderr, "a wall timer started again: untried, load came first %d times\n",
                     TRIES);
        return true;
    }
    char said[96];
    if (last.ends != 1)
        std::snprintf(said, sizeof said, "it ended %d times", last.ends);
    else
        std::snprintf(said, sizeof said, "ended %.1f ms after its second start", last.ended);
    return expectTimer("started again ends its interval from then",
                       last.ends == 1 && last.ended >= HELD_FOR_MS - 1, said);
}

// It ends once a start, and not at all once stopped.
bool endsOnceAndStops(WallTimer &timer) {
    int ends = 0;
    const auto heard = QObject::connect(&timer, &WallTimer::triggered, [&ends] { ++ends; });
    timer.restart();
    turnFor(HELD_FOR_MS * 6);
    const bool once = expectTimer("ends once a start", ends == 1,
                                  ends == 0 ? "it never ended" : "it ended again");
    const int endedOnce = ends;
    timer.restart();
    timer.stop();
    turnFor(HELD_FOR_MS * 4);
    QObject::disconnect(heard);
    const bool stopped = expectTimer("stopped never ends", ends == endedOnce, "it ended");
    return once && stopped;
}

bool wallTimed() {
    WallTimer timer;
    timer.setInterval(HELD_FOR_MS);
    const bool onTime = endsOnTime(timer);
    const bool restarted = restartedEndsLater(timer);
    const bool once = endsOnceAndStops(timer);
    return onTime && restarted && once;
}

} // namespace

int main(int argc, char **argv) {
    QGuiApplication app(argc, argv);
    KeyTimes times;
    QObject target;
    const double pressed = pressedAgo(times, target, false);
    const double repeated = pressedAgo(times, target, true);
    const bool fromPress =
        expect("a press is timed from when the compositor says it went down",
               pressed > HELD_MS - SLACK_MS && pressed < HELD_MS + SLACK_MS, pressed);
    const bool fromRepeat =
        expect("a held key's repeat is timed from when Qt made it, not its first press",
               repeated >= 0 && repeated < SLACK_MS, repeated);
    // Before the thread's clocks, which hold it to one CPU.
    const bool walled = wallTimed();
    const bool timed = threadTimed(times);
    const bool faults = faultsCounted(times);
    return fromPress && fromRepeat && timed && walled && faults ? 0 : 1;
}
