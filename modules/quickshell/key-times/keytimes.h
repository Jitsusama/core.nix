// When a key or button was pressed, as the compositor says, and the clock
// it says it on, as a QML singleton, so the launcher can time a key from
// its press to its frame. QML's KeyEvent carries no time, and QML's
// JavaScript has only the realtime clock, so neither is reachable from the
// shell's own code. Beside them, how long the shell's thread has run and
// waited for a CPU, which neither reaches either. keytimes.cpp says how the
// times are read.
#pragma once

#include <QObject>
#include <QtQml/qqmlregistration.h>

class KeyTimes : public QObject {
    Q_OBJECT
    QML_ELEMENT
    QML_SINGLETON

public:
    explicit KeyTimes(QObject *parent = nullptr);

    // When the key or button the shell is handling now was pressed, in
    // milliseconds of the monotonic clock, or 0 when it's handling none, as
    // when the launcher opens through IPC, so a change no press made is
    // never timed from an old one.
    Q_INVOKABLE double pressed() const;

    // When the shell last heard a key, a button or the wheel, in
    // milliseconds of the monotonic clock, as it heard it rather than as
    // the compositor stamped it, or 0 before any: an open launcher
    // collects only once it's been quiet a while (Collector.js's
    // idleCollectionDue), since a collection in the middle of a key, a
    // scroll or a click stalls its frames. The filter sees every one, so
    // no path an input takes to the launcher can miss it.
    Q_INVOKABLE double lastInputHeard() const;

    // The monotonic clock now, in milliseconds.
    Q_INVOKABLE double now() const;

    // How many milliseconds the calling thread has run on a CPU, and how
    // many it has waited for one while it could have run, since it began,
    // so an opening's stall says what of it was the shell's thread at work,
    // what was the machine's other work ahead of it, and what was the
    // thread asleep: Joel's journal called a mean 39 ms of each opening's
    // longest stall unnamed, beside bursts of other programs starting
    // (2026-11-15). `waited` is 0 where the kernel keeps no schedstat.
    Q_INVOKABLE double ran() const;
    Q_INVOKABLE double waited() const;

    // The calling thread's major page faults since it began, each a page
    // read back in from swap or a file, zswap's compressed pool among
    // them, or -1 where they can't be read, so a stall it slept through
    // says how often it waited on its own memory: Joel's "s" slept 448 ms
    // of 566 searching, in synchronous work that never waits on the event
    // loop, with memory short (2026-11-21). Reclaiming pages to allocate,
    // or waiting on a page another thread is reading in, is no major
    // fault of its own, so none doesn't clear memory.
    Q_INVOKABLE double faulted() const;

protected:
    bool eventFilter(QObject *watched, QEvent *event) override;

private:
    double handling = 0;
    double inputHeard = 0;
    // Which press is being handled, so the one after it isn't let go of
    // when the one before is.
    quint64 presses = 0;
};
