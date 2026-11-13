// How much of the shell's JavaScript heap is live, as a QML singleton, so
// the launcher can say it beside the collection it runs on closing. From
// outside the shell only the heap's resident pages can be read, which Qt's
// collector keeps while any item in a chunk is live, so they overstate
// what's live by two to three times; heap.cpp says what's read instead.
#pragma once

#include <QObject>
#include <QVariantMap>
#include <QtQml/qqmlregistration.h>

class QJSEngine;
class QQmlEngine;

class Heap : public QObject {
    Q_OBJECT
    QML_ELEMENT
    QML_SINGLETON

public:
    explicit Heap(QJSEngine *engine, QObject *parent = nullptr);

    static Heap *create(QQmlEngine *qmlEngine, QJSEngine *engine);

    // The heap as it stands, in megabytes: `live`, what its items take;
    // `held`, the chunks and large items it keeps for them; and `outside`,
    // what its strings and objects hold beyond the heap; and `free`, of what
    // it holds, the resident pages with nothing in them, which could go
    // back to the kernel without moving anything. Read after a collection,
    // live is what survived it.
    Q_INVOKABLE QVariantMap weigh() const;

    // How many milliseconds Qt's collector has spent collecting since the
    // module was made, its slices between other work included, so a key
    // timed from one reading to the next says how much of it was the
    // collector's (Joel's slowest keys waited on it, 2026-11-12).
    Q_INVOKABLE double collected() const;

private:
    QJSEngine *engine;
};
