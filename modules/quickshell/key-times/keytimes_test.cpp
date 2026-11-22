// What KeyTimes says a key was pressed at, for a press the compositor
// timed and for a repeat Qt made of a held key, which carries the first
// press's time (keytimes.cpp). Each event is sent as Qt delivers one, so
// it passes the application's filters, and read while it's handled, as
// the launcher reads it. And how long the thread ran and waited for a
// CPU, read while it spins alone and then beside another thread held to
// the same CPU. And that a WallTimer ends on its time, once a start, from
// its latest start, and not at all once stopped. It exits 0
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

// Keeps the calling thread busy for SPUN_MS, and gives how much more
// `read` says after than before.
double spun(const KeyTimes &times, double (KeyTimes::*read)() const) {
    const double before = (times.*read)();
    const double until = monotonicNow() + SPUN_MS;
    while (monotonicNow() < until) {
    }
    return (times.*read)() - before;
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

// What the thread's clocks say while it spins alone, then beside another
// thread spinning on its CPU, which the scheduler takes turns with: alone
// it runs nearly all the time, and beside the other it waits about half.
bool threadTimed(const KeyTimes &times) {
    const int cpu = sched_getcpu();
    holdTo(pthread_self(), cpu);
    const double ranAlone = spun(times, &KeyTimes::ran);
    std::atomic<bool> going{true};
    std::thread other([&going] {
        while (going.load(std::memory_order_relaxed)) {
        }
    });
    holdTo(other.native_handle(), cpu);
    const double waitedBeside = spun(times, &KeyTimes::waited);
    going = false;
    other.join();
    const bool ran = expectMs("a thread spinning alone is said to have run most of it",
                              ranAlone > SPUN_MS / 2 && ranAlone <= SPUN_MS + SLACK_MS, ranAlone);
    const bool waited = expectMs("a thread taking turns with another is said to have waited",
                                 waitedBeside > SPUN_MS / 4 && waitedBeside < SPUN_MS,
                                 waitedBeside);
    return ran && waited;
}

// How many fresh pages the thread touches, each a minor fault: a page
// never touched is handed out zeroed, with nothing to read in.
constexpr long TOUCHED_PAGES = 1024;

// Touches TOUCHED_PAGES fresh pages, and gives whether they could be had.
bool touchedFresh() {
    const long page = sysconf(_SC_PAGESIZE);
    const auto length = static_cast<std::size_t>(page * TOUCHED_PAGES);
    void *mapped = mmap(nullptr, length, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS,
                        -1, 0);
    if (mapped == MAP_FAILED)
        return false;
    auto *bytes = static_cast<volatile char *>(mapped);
    for (long at = 0; at < TOUCHED_PAGES; at++)
        bytes[at * page] = 1;
    munmap(mapped, length);
    return true;
}

// The thread's faults are read, are never fewer later, and are the major
// ones alone: a thousand minor faults don't raise them a thousand, which
// a count of every fault would. That they're the calling thread's alone
// rests on getrusage's RUSAGE_THREAD and isn't shown here, since nothing
// in the build can make a page wait on the disk to order.
bool faultsCounted(const KeyTimes &times) {
    const double before = times.faulted();
    const bool touched = touchedFresh();
    const double after = times.faulted();
    const bool holds = before >= 0 && touched && after >= before
        && after - before < TOUCHED_PAGES;
    if (!holds)
        std::fprintf(stderr, "FAIL a thread's major faults are read: %.0f, then %.0f over %ld"
                     " fresh pages%s\n", before, after, TOUCHED_PAGES,
                     touched ? "" : ", which couldn't be had");
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
