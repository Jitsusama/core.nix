// A timer that ends on its time, as a QML type, for a wait measured in
// milliseconds. QML's own Timer runs on the animation timer, which Qt
// Quick's threaded render loop steps only once a frame has swapped, so a
// wait shorter than a frame ends at the next swap or later: the launcher's
// 8 ms hold of a key for its answers never ended sooner than 11 ms in
// Joel's journal, a median 47, and 237 behind a held swap (2026-11-12 to
// 2026-11-16). This one is a Qt timer on the monotonic clock, which ends
// as soon as the event loop turns past its time. It goes once per start,
// as the hold needs, with the start and stop QML's Timer has.
#pragma once

#include <QObject>
#include <QTimer>
#include <QtQml/qqmlregistration.h>

class WallTimer : public QObject {
    Q_OBJECT
    QML_ELEMENT
    Q_PROPERTY(int interval READ interval WRITE setInterval NOTIFY intervalChanged)

public:
    explicit WallTimer(QObject *parent = nullptr);

    // How many milliseconds from its start it ends.
    int interval() const;
    void setInterval(int interval);

    // Starts it from now, whether or not it was going.
    Q_INVOKABLE void restart();
    // Stops it, so it doesn't end until it's started again.
    Q_INVOKABLE void stop();

signals:
    void triggered();
    void intervalChanged();

private:
    QTimer timer;
};
