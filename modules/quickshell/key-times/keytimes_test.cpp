// What KeyTimes says a key was pressed at, for a press the compositor
// timed and for a repeat Qt made of a held key, which carries the first
// press's time (keytimes.cpp). Each event is sent as Qt delivers one, so
// it passes the application's filters, and read while it's handled, as
// the launcher reads it. And how long the thread ran and waited for a
// CPU, read while it spins alone and then beside another thread held to
// the same CPU. It exits 0 once all hold, and otherwise 1, saying which
// didn't.
#include "keytimes.h"

#include <QGuiApplication>
#include <QKeyEvent>
#include <atomic>
#include <cstdint>
#include <cstdio>
#include <ctime>
#include <pthread.h>
#include <sched.h>
#include <thread>

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
    const bool timed = threadTimed(times);
    return fromPress && fromRepeat && timed ? 0 : 1;
}
