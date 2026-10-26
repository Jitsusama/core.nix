// The launcher's maths, as a QML singleton: libqalculate in the shell's own
// process, its definitions read once, asked directly on each key with a
// time limit, so no calculator program is started and no text is scraped
// from one. maths.cpp says what it gives back.
#pragma once

#include <QObject>
#include <QString>
#include <QVariantMap>
#include <QtQml/qqmlregistration.h>

class Maths : public QObject {
    Q_OBJECT
    QML_ELEMENT
    QML_SINGLETON

public:
    explicit Maths(QObject *parent = nullptr);

    // Starts reading the definitions in the background if nothing has yet,
    // so the first sum typed finds them read; the launcher calls it as it's
    // made.
    Q_INVOKABLE void prepare() const;

    // What libqalculate makes of an expression, as maths.cpp lists it.
    Q_INVOKABLE QVariantMap calculate(const QString &expression) const;

private:
    // The last expression asked and what it gave, since the launcher asks
    // the same words several times for one key.
    mutable QString lastAsked;
    mutable QVariantMap lastSaid;
};
