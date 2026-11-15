// When a key or button was pressed, as the compositor says, and the clock
// it says it on, as a QML singleton, so the launcher can time a key from
// its press to its frame. QML's KeyEvent carries no time, and QML's
// JavaScript has only the realtime clock, so neither is reachable from the
// shell's own code. keytimes.cpp says how the times are read.
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

protected:
    bool eventFilter(QObject *watched, QEvent *event) override;

private:
    double handling = 0;
    // Which press is being handled, so the one after it isn't let go of
    // when the one before is.
    quint64 presses = 0;
};
