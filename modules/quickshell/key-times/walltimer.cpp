// A Qt timer goes once per start, as walltimer.h says. It's set precise,
// kept to the millisecond, though Qt makes one this short precise anyway,
// so a longer interval given it later isn't let run 5% late.

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
