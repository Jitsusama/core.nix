// What Heap says of a JavaScript engine's heap as it keeps a few hundred
// thousand objects and then lets them go, each read after a collection, as
// the launcher reads it: live grows by what's kept and falls once it's let
// go, and the heap never holds less than is live in it. It exits 0 once
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

struct Weight {
    double live;
    double held;
};

Weight weighed(QJSEngine &engine, const Heap &heap) {
    engine.collectGarbage();
    const QVariantMap said = heap.weigh();
    return {said.value(QStringLiteral("live")).toDouble(),
            said.value(QStringLiteral("held")).toDouble()};
}

bool expect(const char *what, bool holds, const Weight &before, const Weight &after) {
    if (!holds)
        std::fprintf(stderr, "FAIL %s: live %.1f then %.1f MB, held %.1f then %.1f MB\n", what,
                     before.live, after.live, before.held, after.held);
    return holds;
}

} // namespace

int main(int argc, char **argv) {
    QCoreApplication app(argc, argv);
    QJSEngine engine;
    Heap heap(&engine);
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
    kept = QJSValue();
    const Weight letGo = weighed(engine, heap);
    const bool grows = expect("what's kept is live after a collection",
                              keeping.live - empty.live > KEPT_MB, empty, keeping);
    const bool falls = expect("what's let go isn't live after a collection",
                              keeping.live - letGo.live > KEPT_MB, keeping, letGo);
    const bool holds = expect("the heap holds at least what's live in it",
                              keeping.held >= keeping.live && letGo.held >= letGo.live, keeping,
                              letGo);
    return grows && falls && holds ? 0 : 1;
}
