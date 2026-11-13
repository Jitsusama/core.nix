// What Heap says of a JavaScript engine's heap as it keeps a few hundred
// thousand objects and then lets them go, each read after a collection, as
// the launcher reads it: live grows by what's kept and falls once it's let
// go, and the heap never holds less than is live in it; keeping a scattered
// few leaves most of what's held beyond them in free pages, and free pages
// are never more than what's held beyond live; and the time the collector
// has spent grows with each collection. It exits 0 once
// all of it holds, and otherwise 1, saying what didn't.
#include "heap.h"

#include <QCoreApplication>
#include <QJSEngine>
#include <cstdio>

namespace {

// How many objects are kept, each a number and a short string, about 20 MB
// of the heap in all.
constexpr int KEPT = 200000;
// How much of that, in megabytes, the live count must move by at least.
constexpr double KEPT_MB = 10;

// One object in this many is kept once the rest are let go, so nearly
// every page that held them holds nothing.
constexpr int SCATTERED = 1000;

struct Weight {
    double live;
    double held;
    double free;
};

Weight weighed(QJSEngine &engine, const Heap &heap) {
    engine.collectGarbage();
    const QVariantMap said = heap.weigh();
    return {said.value(QStringLiteral("live")).toDouble(),
            said.value(QStringLiteral("held")).toDouble(),
            said.value(QStringLiteral("free")).toDouble()};
}

bool expect(const char *what, bool holds, const Weight &before, const Weight &after) {
    if (!holds)
        std::fprintf(stderr,
                     "FAIL %s: live %.1f then %.1f MB, held %.1f then %.1f MB, free %.1f then "
                     "%.1f MB\n",
                     what, before.live, after.live, before.held, after.held, before.free,
                     after.free);
    return holds;
}

} // namespace

int main(int argc, char **argv) {
    QCoreApplication app(argc, argv);
    QJSEngine engine;
    Heap heap(&engine);
    const double collectedFirst = heap.collected();
    const Weight empty = weighed(engine, heap);
    // Held from here, since the engine's global object takes no new names.
    QJSValue kept = engine.evaluate(
        QStringLiteral("(() => { const kept = []; for (let i = 0; i < %1; i++) kept.push({ n: i, "
                       "s: 'row ' + i }); return kept; })()")
            .arg(KEPT));
    if (kept.isError()) {
        std::fprintf(stderr, "FAIL the objects weren't made: %s\n",
                     qPrintable(kept.toString()));
        return 1;
    }
    const Weight keeping = weighed(engine, heap);
    QJSValue few = engine
                       .evaluate(QStringLiteral("(kept) => kept.filter((_, i) => i % %1 === 0)")
                                     .arg(SCATTERED))
                       .call({kept});
    kept = QJSValue();
    const Weight thinned = weighed(engine, heap);
    few = QJSValue();
    const double collectedBefore = heap.collected();
    const Weight letGo = weighed(engine, heap);
    const double collectedAfter = heap.collected();
    const bool grows = expect("what's kept is live after a collection",
                              keeping.live - empty.live > KEPT_MB, empty, keeping);
    const bool falls = expect("what's let go isn't live after a collection",
                              keeping.live - letGo.live > KEPT_MB, keeping, letGo);
    const bool holds = expect("the heap holds at least what's live in it",
                              keeping.held >= keeping.live && letGo.held >= letGo.live, keeping,
                              letGo);
    const bool scattered =
        expect("a scattered few kept leave most of what's held beyond them in free pages",
               thinned.free > (thinned.held - thinned.live) / 2, keeping, thinned);
    const auto withinHeld = [](const Weight &weight) {
        return weight.free >= 0 && weight.free <= weight.held - weight.live;
    };
    const bool bounded =
        expect("free pages are never more than what's held beyond live",
               withinHeld(empty) && withinHeld(keeping) && withinHeld(thinned) &&
                   withinHeld(letGo),
               thinned, letGo);
    // Letting go of 20 MB takes the collector well over a microsecond,
    // and each collection adds to what it's said to have spent.
    const bool timed = collectedAfter > collectedBefore && collectedBefore > collectedFirst;
    if (!timed)
        std::fprintf(stderr, "FAIL the collector's time grows with each collection: %.3f, "
                     "then %.3f, then %.3f ms\n", collectedFirst, collectedBefore, collectedAfter);
    return grows && falls && holds && scattered && bounded && timed ? 0 : 1;
}
