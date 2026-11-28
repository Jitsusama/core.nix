// What KeyTimes says a key was pressed at, for a press the compositor
// timed and for a repeat Qt made of a held key, which carries the first
// press's time (keytimes.cpp), and that a key, a button and the wheel are
// each kept as the last input heard. Each event is sent as Qt delivers one, so
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
#include <QMouseEvent>
#include <QWheelEvent>
#include <QTimer>
#include <atomic>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <ctime>
#include <fcntl.h>
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

// Whether an event sent as Qt delivers one is kept as the last input heard:
// read after it's sent, it's no earlier than the clock just before and no
// later than the clock just after, whatever the event said its time was.
bool heardAsSent(const KeyTimes &times, QObject &target, QEvent &event, const char *what) {
    const double before = monotonicNow();
    QCoreApplication::sendEvent(&target, &event);
    const double heard = times.lastInputHeard();
    const double after = monotonicNow();
    const bool holds = heard >= before && heard <= after;
    if (!holds)
        std::fprintf(stderr, "FAIL %s is kept as the last input heard: heard at %.1f, sent"
                     " between %.1f and %.1f\n", what, heard, before, after);
    return holds;
}

// A key, a button's press and the wheel are each the last input heard as
// it comes, a key a program typed with no time among them, and a key's
// release isn't one.
bool inputsHeard(const KeyTimes &times, QObject &target) {
    QKeyEvent key(QEvent::KeyPress, Qt::Key_J, Qt::NoModifier, QStringLiteral("j"));
    const bool keyed = heardAsSent(times, target, key, "a key typed with no time");
    const QPointF at(4, 4);
    QMouseEvent press(QEvent::MouseButtonPress, at, at, Qt::LeftButton, Qt::LeftButton,
                      Qt::NoModifier);
    const bool pressed = heardAsSent(times, target, press, "a button's press");
    QWheelEvent wheel(at, at, QPoint(), QPoint(0, -120), Qt::NoButton, Qt::NoModifier,
                      Qt::NoScrollPhase, false);
    const bool wheeled = heardAsSent(times, target, wheel, "the wheel");
    const double heard = times.lastInputHeard();
    QKeyEvent release(QEvent::KeyRelease, Qt::Key_J, Qt::NoModifier, QStringLiteral("j"));
    QCoreApplication::sendEvent(&target, &release);
    const bool released = times.lastInputHeard() == heard;
    if (!released)
        std::fprintf(stderr, "FAIL a key's release isn't kept as the last input heard\n");
    return keyed && pressed && wheeled && released;
}

// How long the thread spins, by the monotonic clock, for each reading.
constexpr double SPUN_MS = 200;
// How far two reads of one count may differ for being turned into
// milliseconds two ways, far less than a clock in seconds or a wait read
// from the run figure would be off by.
constexpr double ROUNDED_MS = 1;
// How long a thread beside another on its CPU waits at least, an eighth
// of the spin where it waits about half, and the most a spin alone may
// wait for that bound to be told from what other builds made it wait.
constexpr double TURNS_MS = SPUN_MS / 8;

// How long a virtual machine's host has taken from a CPU so far, by the
// kernel's count in /proc/stat (its eighth figure, in clock ticks), or -1
// where it can't be read, so a failed read is never taken for the count.
// A spinning thread is neither running nor waiting while it's taken, so
// the thread's own clocks can't count it.
double stolenFrom(int cpu) {
    std::FILE *stat = std::fopen("/proc/stat", "re");
    if (stat == nullptr)
        return -1;
    char name[16];
    std::snprintf(name, sizeof name, "cpu%d ", cpu);
    const std::size_t named = std::strlen(name);
    char line[512];
    unsigned long long ticks = 0;
    bool found = false;
    while (!found && std::fgets(line, sizeof line, stat) != nullptr) {
        unsigned long long skipped = 0;
        found = std::strncmp(line, name, named) == 0
            && std::sscanf(line + named, "%llu %llu %llu %llu %llu %llu %llu %llu", &skipped,
                           &skipped, &skipped, &skipped, &skipped, &skipped, &skipped, &ticks)
                == 8;
    }
    std::fclose(stat);
    return found ? static_cast<double>(ticks) * 1000.0 / static_cast<double>(sysconf(_SC_CLK_TCK))
                 : -1;
}

// The calling thread's schedstat, read apart from KeyTimes::waited() and
// for both its first figures: how long the thread has run, the count its
// CPU clock keeps too, and how long it has waited on a run queue, each in
// milliseconds, or -1 for both where it can't be read.
struct Sched {
    double run;
    double wait;
};

Sched schedNow() {
    const Sched unread{-1, -1};
    const int file = open("/proc/thread-self/schedstat", O_RDONLY | O_CLOEXEC);
    if (file < 0)
        return unread;
    char text[96];
    const ssize_t got = read(file, text, sizeof text - 1);
    close(file);
    if (got <= 0)
        return unread;
    text[got] = '\0';
    unsigned long long run = 0;
    unsigned long long wait = 0;
    if (std::sscanf(text, "%llu %llu", &run, &wait) != 2)
        return unread;
    return {static_cast<double>(run) / 1e6, static_cast<double>(wait) / 1e6};
}

// The calling thread's voluntary context switches so far, one each time
// it slept, or -1 where they can't be read.
long voluntarySwitches() {
    rusage usage{};
    if (getrusage(RUSAGE_THREAD, &usage) != 0)
        return -1;
    return usage.ru_nvcsw;
}

// What KeyTimes's clocks say at one moment, the thread's schedstat read
// just before and just after them, and its CPU's stolen count.
struct Reading {
    double ran;
    double waited;
    Sched before;
    Sched after;
    double stolen;
};

Reading readAt(const KeyTimes &times, int cpu) {
    const Sched before = schedNow();
    const double ran = times.ran();
    const double waited = times.waited();
    const Sched after = schedNow();
    return {ran, waited, before, after, stolenFrom(cpu)};
}

// How much more the thread's clocks and its CPU's stolen count say after a
// spin than before it, the stolen count 0 unless both its reads worked;
// how long the spin took by the monotonic clock, which is past SPUN_MS
// when the thread is held off as it ends; how many times it slept, or -1
// where that can't be read; and the readings either side.
struct Spin {
    double ran;
    double waited;
    double stolen;
    double took;
    long slept;
    Reading first;
    Reading last;
};

// Keeps the calling thread, held to cpu, busy for SPUN_MS, and gives how
// much more each clock says after than before. The monotonic clock is
// read first and last, so `took` holds every other reading, and a wait
// while they're read counts in it as it does in `waited`. Its voluntary
// switches are read outside even that, so a sleep in any read is seen.
Spin spun(const KeyTimes &times, int cpu) {
    const long switchedBefore = voluntarySwitches();
    const double began = monotonicNow();
    const Reading first = readAt(times, cpu);
    const double until = began + SPUN_MS;
    while (monotonicNow() < until) {
    }
    const Reading last = readAt(times, cpu);
    const double took = monotonicNow() - began;
    const long switchedAfter = voluntarySwitches();
    const bool stolenRead = first.stolen >= 0 && last.stolen >= 0;
    const long slept =
        switchedBefore < 0 || switchedAfter < 0 ? -1 : switchedAfter - switchedBefore;
    return {last.ran - first.ran,
            last.waited - first.waited,
            stolenRead ? last.stolen - first.stolen : 0,
            took,
            slept,
            first,
            last};
}

// Whether a reading's clocks lie between the schedstat read either side of
// them. The CPU clock brings the thread's run figure up to date as it's
// read, and the wait figure only grows, so each lies between its figure
// before and after, to the rounding. Read as totals since the thread
// began, a clock in seconds or a wait read from the run figure is far off
// at any load, where over one spin a wait about as long as the run would
// hide it.
bool betweenSchedstat(const char *when, const Reading &reading) {
    const auto within = [](double value, double low, double high) {
        return low >= 0 && value >= low - ROUNDED_MS && value <= high + ROUNDED_MS;
    };
    const bool holds = within(reading.ran, reading.before.run, reading.after.run)
        && within(reading.waited, reading.before.wait, reading.after.wait);
    if (!holds)
        std::fprintf(stderr, "FAIL a thread's clocks agree with its schedstat %s: ran %.3f"
                     " between %.3f and %.3f, waited %.3f between %.3f and %.3f ms, -1 where"
                     " schedstat couldn't be read\n", when, reading.ran, reading.before.run,
                     reading.after.run, reading.waited, reading.before.wait,
                     reading.after.wait);
    return holds;
}

// Holds a thread to one CPU, so two held to it take turns.
void holdTo(pthread_t thread, int cpu) {
    cpu_set_t one;
    CPU_ZERO(&one);
    CPU_SET(cpu, &one);
    pthread_setaffinity_np(thread, sizeof one, &one);
}

// Says what a spin read when a bound on it doesn't hold: the bound, every
// clock, and the wall time the bounds are judged against.
bool expectSpin(const char *what, const char *bound, bool holds, const Spin &spin) {
    if (!holds)
        std::fprintf(stderr, "FAIL %s, %s: ran %.0f, waited %.0f, stolen %.0f in %.0f ms\n",
                     what, bound, spin.ran, spin.waited, spin.stolen, spin.took);
    return holds;
}

// How far a stolen count may fall short, a clock tick at each end.
double tickMs() {
    return 2 * 1000.0 / static_cast<double>(sysconf(_SC_CLK_TCK));
}

// Whether a spin's clocks cover most of it, and never more than it took. A
// spinning thread is only running, waiting for its CPU, or taken by a
// virtual machine's host, so the three together cover the spin. How they
// share it isn't held: on a builder other builds run on the same CPU,
// and a spin held to it by itself ran 78 ms of 200 there. Only most,
// since the stolen count is kept in whole ticks and the kernel's own work
// is in none of them. Stolen is left out of the most they may say: taken
// while another thread held the CPU, it's in this one's wait too. The run
// has no most of its own: it's in what the two say, and the wait beside
// it, pinned to schedstat's (betweenSchedstat), is never below none.
bool covers(const char *what, const Spin &spin) {
    const double spent = spin.ran + spin.waited;
    const bool ran = expectSpin(what, "too little, it ran none of it", spin.ran > 0, spin);
    const bool most = expectSpin(what, "too little, under half of it with stolen and two ticks",
                                 spent + spin.stolen + tickMs() > spin.took / 2, spin);
    char past[64];
    std::snprintf(past, sizeof past, "too much, ran and waited past it by over %.0f ms",
                  SLACK_MS);
    const bool within = expectSpin(what, past, spent <= spin.took + SLACK_MS, spin);
    return ran && most && within;
}

// What the thread's clocks say while it spins, then beside another thread
// spinning on its CPU, which the scheduler takes turns with. Each reading
// lies between the schedstat read either side of it (betweenSchedstat),
// which a clock in the wrong unit or a wait read from the run figure fails
// at any load. Each spin is covered by its clocks (covers), which a clock
// counted twice fails. Beside the other it waits about half the spin, so
// over an eighth (TURNS_MS) fails a spin with no thread beside it. That's
// told apart only where the spin alone waited less, since other builds
// can make a thread wait as long with none beside it; elsewhere it's
// untried. A bound judged against the wait alone would fail whenever the
// load fell between the two spins, so the thread beside is judged only
// where the load can't stand in for it. A spin that slept, as one whose
// read of /proc blocks under memory pressure does, spent the sleep in
// none of its clocks, so its bounds are untried rather than failed.
bool threadTimed(const KeyTimes &times) {
    const int cpu = sched_getcpu();
    holdTo(pthread_self(), cpu);
    const Spin alone = spun(times, cpu);
    std::atomic<bool> going{true};
    std::thread other([&going] {
        while (going.load(std::memory_order_relaxed)) {
        }
    });
    holdTo(other.native_handle(), cpu);
    const Spin beside = spun(times, cpu);
    going = false;
    other.join();
    const bool setOff = betweenSchedstat("as it sets off alone", alone.first);
    const bool done = betweenSchedstat("once it's spun alone", alone.last);
    const bool turns = betweenSchedstat("as it sets off taking turns", beside.first);
    const bool turned = betweenSchedstat("once it's taken turns", beside.last);
    const bool agree = setOff && done && turns && turned;
    if (alone.slept > 0 || beside.slept > 0) {
        std::fprintf(stderr, "a thread's clocks over its spins: untried, load came first, it"
                     " slept %ld times alone and %ld taking turns\n", alone.slept,
                     beside.slept);
        return agree;
    }
    const bool whole = covers("a thread spinning alone is said to have spent it all", alone);
    const bool shared = covers("a thread taking turns is said to have spent it all", beside);
    const char *turnsWaited = "a thread taking turns with another is said to have waited";
    if (alone.waited >= TURNS_MS) {
        std::fprintf(stderr, "%s: untried, load came first, alone it waited %.0f of %.0f ms\n",
                     turnsWaited, alone.waited, alone.took);
        return agree && whole && shared;
    }
    char least[64];
    std::snprintf(least, sizeof least, "too little, %.0f ms or under", TURNS_MS);
    const bool waited = expectSpin(turnsWaited, least, beside.waited > TURNS_MS, beside);
    return agree && whole && shared && waited;
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
    const bool heard = inputsHeard(times, target);
    // Before the thread's clocks, which hold it to one CPU.
    const bool walled = wallTimed();
    const bool timed = threadTimed(times);
    const bool faults = faultsCounted(times);
    return fromPress && fromRepeat && heard && timed && walled && faults ? 0 : 1;
}
