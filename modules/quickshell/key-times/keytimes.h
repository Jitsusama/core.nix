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

protected:
    bool eventFilter(QObject *watched, QEvent *event) override;

private:
    double handling = 0;
    // Which press is being handled, so the one after it isn't let go of
    // when the one before is.
    quint64 presses = 0;
};
