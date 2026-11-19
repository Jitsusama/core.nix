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

    // What jemalloc, which Quickshell allocates through, holds outside the
    // heap, in megabytes: `used`, what's allocated and not yet freed;
    // `resident`, its pages in memory; and `kept`, the pages it was handed
    // back and keeps for a while before giving them to the kernel. Empty
    // where the process doesn't allocate through jemalloc.
    Q_INVOKABLE QVariantMap allocator() const;

    // Lets the heap take this many megabytes of new chunks before a miss
    // in its free lists starts the collector, and its strings as many
    // more outside it, and as much again after every collection that ends
    // later. Asked after the closing's collection and again as an opening
    // is asked, so the opening's allocations don't start one, whatever was
    // made while it was shut or collected since, and the closing's collects
    // what they left (heap.cpp).
    Q_INVOKABLE void leaveRoom(double megabytes);

    // Gives the heap's free pages, the ones weigh() counts as `free`, back
    // to the kernel, and says how many megabytes of them were resident.
    // Asked once a collection is done, since a collection under way may
    // still be reading them; it gives nothing back while one is.
    Q_INVOKABLE double giveBack();

    // What's live, by what each item is: `classes`, the `top` largest of
    // Qt's classes, each a list of its name, megabytes and count; and
    // `shapes`, the `top` largest shapes, each its class, its first six
    // property names joined by commas, how many names it has, its
    // megabytes and its count, a name taken from data said as `~`; `places`,
    // the `top` largest places in the code closures and calls' contexts
    // came from, each its file, line and name, as `Launcher.qml:943
    // spotsFrom`, its megabytes and its count; `total`,
    // the megabytes of every class; `ms`, how long counting took; and
    // `said`, all of it as one line. While a collection is under way it
    // counts nothing and answers only `busy`.
    Q_INVOKABLE QVariantMap census(int top) const;

private:
    QJSEngine *engine;
};
