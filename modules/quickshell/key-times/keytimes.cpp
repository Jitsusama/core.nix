// Every event the shell is given passes the application's own filters
// first, so one filter sees each key's and button's press before QML does,
// and keeps its timestamp. On Wayland that's the time the compositor gave
// the press, which niri takes from libinput, which takes it from the
// kernel's input event: the moment the device said it went down, in
// milliseconds of the monotonic clock. The filter only looks at an event's
// type, so it costs nothing to the rest.
//
// A press is handled in full before the event loop turns again, so the
// time is let go of on the loop's next turn, and a launcher that changes
// for some other reason afterwards finds none.
//
// The compositor's time is 32 bits of milliseconds, which wrap every 49.7
// days, so it's widened against the clock now: a press is always handled
// within 49 days of its making.
//
// A held key is repeated by Qt's Wayland plugin itself, which stamps each
// repeat with the first press's time, so an arrow held across a list read
// every step as made seconds before: Joel's 187 keys, a median 375 ms, had
// steps of 9 to 23 ms to their frames (FEH1HE, 2026-11-11). A repeat is
// timed from when the shell is given it instead. When it was due is Qt's
// alone, as the compositor's repeat delay and rate are, so a repeat late
// for a busy shell is counted from when it came, not when it was due.
#include "keytimes.h"

#include <QCoreApplication>
#include <QInputEvent>
#include <QKeyEvent>
#include <QMetaObject>
#include <cstdint>
#include <ctime>

namespace {

double monotonicNow() {
    timespec now{};
    clock_gettime(CLOCK_MONOTONIC, &now);
    return static_cast<double>(now.tv_sec) * 1000.0 + static_cast<double>(now.tv_nsec) / 1e6;
}

} // namespace

KeyTimes::KeyTimes(QObject *parent) : QObject(parent) {
    QCoreApplication::instance()->installEventFilter(this);
}

double KeyTimes::pressed() const { return handling; }

double KeyTimes::now() const { return monotonicNow(); }

bool KeyTimes::eventFilter(QObject *watched, QEvent *event) {
    const auto type = event->type();
    if (type == QEvent::KeyPress || type == QEvent::MouseButtonPress) {
        const double now = monotonicNow();
        const bool repeat =
            type == QEvent::KeyPress && static_cast<QKeyEvent *>(event)->isAutoRepeat();
        const auto stamp = repeat ? static_cast<std::uint32_t>(static_cast<std::uint64_t>(now))
                                  : static_cast<std::uint32_t>(
                                        static_cast<QInputEvent *>(event)->timestamp());
        const auto since = static_cast<std::uint32_t>(static_cast<std::uint64_t>(now)) - stamp;
        // A key a program typed can come with no time, as wtype's do in the
        // test machine, which widened would read as pressed a moment after
        // the clock began; it's a press of unknown time instead.
        handling = stamp == 0 ? 0 : now - static_cast<double>(since);
        const quint64 press = ++presses;
        QMetaObject::invokeMethod(
            this,
            [this, press] {
                if (presses == press)
                    handling = 0;
            },
            Qt::QueuedConnection);
    }
    return QObject::eventFilter(watched, event);
}
