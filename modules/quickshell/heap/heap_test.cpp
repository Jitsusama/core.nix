// What Heap says of a JavaScript engine's heap as it keeps a few hundred
// thousand objects and then lets them go, each read after a collection, as
// the launcher reads it: live grows by what's kept and falls once it's let
// go, and the heap never holds less than is live in it; keeping a scattered
// few leaves most of what's held beyond them in free pages, and free pages
// are never more than what's held beyond live; and the time the collector
// has spent grows with each collection; and a heap holding far more than is
// live, as one does after the launcher's closing, starts the collector as
// it makes more unless it's left room, and doesn't once it is, nor as it
// makes strings whose text is held outside the heap. Giving the free pages
// back leaves next to none resident, says about as much as was counted, and
// what's kept, and what's made after in their place, reads as it was made.
// And what it says of
// jemalloc, which the test is linked with as Quickshell is: what's in use
// grows by a block kept, and what's freed and kept by one let go, and
// neither is more than what's resident. It exits 0 once all of it holds,
// and otherwise 1, saying what didn't.
#include "heap.h"

#include <QCoreApplication>
#include <QJSEngine>
#include <cstdio>
#include <cstdlib>
#include <cstring>

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

// How much room a heap is left, in megabytes, well beyond what's made in it.
constexpr double ROOM_MB = 64;

// What's made, and how many: objects holding only numbers, twice as many as
// are let go, so nothing outside the heap comes into it; or strings of 200
// characters, whose text Qt holds outside the heap, about 8 MB of it against
// 2 MB inside, so only what Qt allows outside can start the collector.
struct Making {
    const char *each;
    int count;
};
constexpr Making NUMBERS{"{ n: i, m: i }", 2 * KEPT};
constexpr Making STRINGS{"String(i).padEnd(200, ' of the clipboard history')", 20000};

// Whether making them starts the collector on a heap left holding a
// scattered few, given that much room first.
bool collectsWhileMaking(double room, const Making &made) {
    QJSEngine engine;
    Heap heap(&engine);
    QJSValue few = engine.evaluate(
        QStringLiteral("(() => { const made = []; for (let i = 0; i < %1; i++) made.push({ n: i, "
                       "m: i }); return made.filter((_, i) => i % %2 === 0); })()")
            .arg(KEPT)
            .arg(SCATTERED));
    engine.collectGarbage();
    heap.leaveRoom(room);
    const double before = heap.collected();
    QJSValue making = engine.evaluate(
        QStringLiteral("(() => { const made = []; for (let i = 0; i < %1; i++) made.push(%2); "
                       "return made; })()")
            .arg(made.count)
            .arg(QLatin1StringView(made.each)));
    return heap.collected() > before;
}

// Blocks as large as the shell's answers' pieces, 32 MB in all, and how
// much of it the allocator's counts must move by at least. jemalloc hands a
// block past 8 MB back to the kernel as it's freed, and keeps the pages of
// smaller ones for a while, which is what the shell frees.
constexpr std::size_t BLOCKS = 512;
constexpr std::size_t BLOCK_BYTES = 64 * 1024;
constexpr double BLOCKS_MB = 32;
constexpr double BLOCKS_MOVE_MB = 16;

// Held where the compiler can't see them go unused, so the blocks are
// really made and written.
char *volatile blocks[BLOCKS] = {};

struct Allocated {
    double used;
    double resident;
    double kept;
};

Allocated allocated(const Heap &heap) {
    const QVariantMap said = heap.allocator();
    return {said.value(QStringLiteral("used"), -1).toDouble(),
            said.value(QStringLiteral("resident"), -1).toDouble(),
            said.value(QStringLiteral("kept"), -1).toDouble()};
}

// Whether the allocator's counts follow a block kept and then let go.
bool allocatorFollows(const Heap &heap) {
    const Allocated before = allocated(heap);
    for (auto &block : blocks) {
        block = static_cast<char *>(std::malloc(BLOCK_BYTES));
        std::memset(block, 1, BLOCK_BYTES);
    }
    const Allocated keeping = allocated(heap);
    for (auto &block : blocks) {
        std::free(block);
        block = nullptr;
    }
    const Allocated letGo = allocated(heap);
    const auto withinResident = [](const Allocated &each) {
        return each.used >= 0 && each.kept >= 0 && each.used <= each.resident &&
               each.kept <= each.resident;
    };
    const bool said = withinResident(before) && withinResident(keeping) &&
                      withinResident(letGo);
    const bool grows = keeping.used - before.used > BLOCKS_MOVE_MB;
    const bool kept = letGo.kept - keeping.kept > BLOCKS_MOVE_MB;
    if (!said || !grows || !kept)
        std::fprintf(stderr,
                     "FAIL jemalloc's counts follow %.0f MB of blocks kept and let go: in use "
                     "%.1f, %.1f, %.1f MB; resident %.1f, %.1f, %.1f MB; freed and kept %.1f, "
                     "%.1f, %.1f MB\n",
                     BLOCKS_MB, before.used, keeping.used, letGo.used, before.resident,
                     keeping.resident, letGo.resident, before.kept, keeping.kept, letGo.kept);
    return said && grows && kept;
}

// What may stay counted free once they're given back, in megabytes: the
// collection the weighing runs rewrites its free runs' headers, and a page
// may hold one again.
constexpr double GIVEN_LEFT_MB = 1;

// Whether giving a thinned heap's free pages back takes them, says so, and
// leaves what's kept, and what's made after, as it was made.
bool givesBack() {
    QJSEngine engine;
    Heap heap(&engine);
    const QJSValue few = engine.evaluate(
        QStringLiteral("(() => { const made = []; for (let i = 0; i < %1; i++) made.push({ n: i, "
                       "s: 'row ' + i }); return made.filter((_, i) => i % %2 === 0); })()")
            .arg(KEPT)
            .arg(SCATTERED));
    const Weight thinned = weighed(engine, heap);
    const double given = heap.giveBack();
    const Weight after = weighed(engine, heap);
    // The few are every SCATTERED-th of 0 to KEPT - 1, so their numbers sum
    // to SCATTERED times the sum of 0 to KEPT / SCATTERED - 1.
    const double sum = SCATTERED * (KEPT / SCATTERED) * (KEPT / SCATTERED - 1) / 2.0;
    const QJSValue kept =
        engine
            .evaluate(QStringLiteral("(few) => few.reduce((sum, each, i) => each.s === 'row ' + "
                                     "i * %1 ? sum + each.n : NaN, 0)")
                          .arg(SCATTERED))
            .call({few});
    const QJSValue made = engine.evaluate(
        QStringLiteral("(() => { const made = []; for (let i = 0; i < %1; i++) made.push({ n: "
                       "i, s: 'row ' + i }); return made.reduce((sum, each, i) => each.s === "
                       "'row ' + i ? sum + each.n : NaN, 0); })()")
            .arg(KEPT));
    const double madeSum = static_cast<double>(KEPT) * (KEPT - 1) / 2.0;
    const bool taken = after.free < GIVEN_LEFT_MB;
    const bool said = given > thinned.free / 2 && given <= thinned.free + GIVEN_LEFT_MB;
    const bool intact = kept.toNumber() == sum && made.toNumber() == madeSum;
    if (!taken || !said || !intact)
        std::fprintf(stderr,
                     "FAIL giving %.1f MB of free pages back takes them and leaves what's "
                     "kept: said %.1f MB, %.1f MB free after; the few summed %.0f of %.0f, "
                     "and what was made after %.0f of %.0f\n",
                     thinned.free, given, after.free, kept.toNumber(), sum, made.toNumber(),
                     madeSum);
    return taken && said && intact;
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
    const bool collects = collectsWhileMaking(0, NUMBERS);
    if (!collects)
        std::fprintf(stderr, "FAIL a heap holding far more than is live starts the collector "
                     "as it makes more\n");
    const bool roomy = !collectsWhileMaking(ROOM_MB, NUMBERS);
    if (!roomy)
        std::fprintf(stderr, "FAIL a heap left %.0f MB of room doesn't start the collector as "
                     "it makes more\n", ROOM_MB);
    const bool roomyOutside = !collectsWhileMaking(ROOM_MB, STRINGS);
    if (!roomyOutside)
        std::fprintf(stderr, "FAIL a heap left %.0f MB of room doesn't start the collector as "
                     "it makes strings held outside it\n", ROOM_MB);
    const bool follows = allocatorFollows(heap);
    const bool given = givesBack();
    return grows && falls && holds && scattered && bounded && timed && collects && roomy &&
                   roomyOutside && follows && given
               ? 0
               : 1;
}
