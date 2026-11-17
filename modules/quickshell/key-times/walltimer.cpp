// A precise timer is kept to the millisecond, where a coarse one may end
// up to 5% late, and goes once, as walltimer.h says.

#include "walltimer.h"

WallTimer::WallTimer(QObject *parent) : QObject(parent) {
    timer.setSingleShot(true);
    timer.setTimerType(Qt::PreciseTimer);
    connect(&timer, &QTimer::timeout, this, &WallTimer::triggered);
}

int WallTimer::interval() const { return timer.interval(); }

void WallTimer::setInterval(int interval) {
    if (interval == timer.interval())
        return;
    timer.setInterval(interval);
    emit intervalChanged();
}

void WallTimer::restart() { timer.start(); }

void WallTimer::stop() { timer.stop(); }
