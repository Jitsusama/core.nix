// What KeyTimes says of a press, run offscreen as the package builds: a
// key's own press keeps the compositor's time, and a held key's repeat is
// timed from when the shell was given it, never from the press seconds
// before, which Qt's Wayland plugin stamps every repeat with.
#include "keytimes.h"

#include <QCoreApplication>
#include <QGuiApplication>
#include <QKeyEvent>
#include <cmath>
#include <cstdint>
#include <cstdio>

namespace {

// How far a time read may stray from the one meant, in milliseconds, for
// a builder that's slow to run the lines between.
constexpr double slack = 100.0;

// A held key's first press, this long before its repeat, as an arrow held
// across a list is.
constexpr double heldFor = 5000.0;

int failures = 0;

void expect(const char *what, double said, double meant) {
    if (std::fabs(said - meant) > slack) {
        std::fprintf(stderr, "FAIL %s: said %.0f, meant %.0f\n", what, said, meant);
        ++failures;
    }
}

// What KeyTimes says while the shell handles a press stamped at `at`.
double pressedAt(KeyTimes &times, double at, bool repeat) {
    QObject target;
    QKeyEvent press(QEvent::KeyPress, Qt::Key_Down, Qt::NoModifier, QString(), repeat);
    press.setTimestamp(static_cast<std::uint32_t>(static_cast<std::uint64_t>(at)));
    QCoreApplication::sendEvent(&target, &press);
    return times.pressed();
}

} // namespace

int main(int argc, char **argv) {
    qputenv("QT_QPA_PLATFORM", "offscreen");
    QGuiApplication app(argc, argv);
    KeyTimes times;
    const double first = times.now() - heldFor;
    expect("a key's press keeps its own time", pressedAt(times, first, false), first);
    expect("a held key's repeat is timed from when it's given", pressedAt(times, first, true),
           times.now());
    return failures == 0 ? 0 : 1;
}
