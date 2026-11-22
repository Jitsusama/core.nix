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
    // back to the kernel without moving anything; `chunks`, what its
    // chunks hold, large items aside; `collectsAt`, what they may hold
    // before a miss in the free lists starts the collector; and
    // `collectsOutsideAt`, what may be held outside before the collector
    // starts. Read after a collection, live is what survived it.
    Q_INVOKABLE QVariantMap weigh() const;

    // Where the heap stands against the collector, as weigh() says it but
    // without walking a chunk, so it's cheap enough to read as an opening
    // is asked: `chunks`, `collectsAt`, `outside` and `collectsOutsideAt`,
    // in megabytes, and `collecting`, whether a collection is under way
    // between its slices. Openings whose longest stall the collector ran
    // in say it, read once their room is left: the room each limit had
    // as they began, which nothing else says.
    Q_INVOKABLE QVariantMap standing() const;

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
    // later, though a miss starts it no later than twice the room beyond
    // what its chunks held at the first ask since a collection ended,
    // whatever room it asked for, and never sooner than Qt would: up to
    // 1024, and none for 0 or what isn't a number, in the heap from the
    // next collection on and outside it as Qt lowers its limit. Asked after the
    // closing's collection and again as an opening is asked, so the
    // opening's allocations don't start one, whatever was made while it was
    // shut or collected since, and the closing's collects what they left
    // (heap.cpp).
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

    // What's live, by the QML property that holds it: properties of one
    // name declared in one file counted together, the file named after its
    // folder, as `launcher/Spend.qml rows`, and each file's functions as
    // `File.qml (functions)`. Each item is counted once, for the first
    // holder reaching it in the order of their names, so what several share,
    // as the shapes of objects made alike, goes to the one sorting first;
    // what the engine keeps of its own is counted before any. `holders`, the
    // `top` that hold the most, each its name, its megabytes and its count;
    // `names` and `named`, how many there were and how many it named before
    // `namingMs` had passed, what the rest hold being `unnamed`;
    // `themselves`, the megabytes of the objects, their properties' storage
    // and the QML contexts, which no property is said to hold; `engine`, its
    // builtins, global object, shapes, names and compiled code; `contexts`,
    // what's reached from the contexts and the scripts they import and no
    // property; `objects`, what's reached from the objects otherwise, their
    // children and the like; `rest`, what none of them reaches: what's held
    // from C++, and what was made since the last collection and is no longer
    // reached; `total`, `ms` and `said`, as census() says them. It
    // marks what each reaches with the collector's own marks and clears
    // them before it returns, so while a collection is under way it counts
    // nothing and answers only `busy`, and with marks already set outside
    // one, which the sweep should leave none of, `busy` and `marked`.
    Q_INVOKABLE QVariantMap holders(int top, int namingMs = 100) const;

    // What letting go of one holder, named as holders() names it, would
    // free: what its values reach that the next collection's own marking
    // wouldn't reach without them, from the engine, the JavaScript stack,
    // the values C++ keeps and the objects that stay alive unreached,
    // through every other property of the objects those reach. Every other
    // holder stays as it is, so memory it shares with another is kept by
    // that other. What an object nothing reaches holds, the sweep's
    // anyway, isn't counted. It counts the heap's own items: the text a
    // string keeps outside the heap, and what C++ keeps for an object the
    // collector would delete, aren't counted, nor is the storage of an
    // object of the holder's own file that only the holder keeps, a few
    // bytes each, though all its values reach is. `held`, whether any property
    // so named holds a value the heap keeps; `freed`, its megabytes;
    // `count`; `ms`; `said`, one line naming it, or saying no property
    // holds anything so named; and, only when one holds something,
    // `holder`. It marks with the collector's own marks and clears them, so
    // it answers `busy`, and `marked`, as holders() does.
    Q_INVOKABLE QVariantMap freedBy(const QString &holder) const;

private:
    QJSEngine *engine;
};
