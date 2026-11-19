// What Heap says of a JavaScript engine's heap as it keeps a few hundred
// thousand objects and then lets them go, each read after a collection, as
// the launcher reads it: live grows by what's kept and falls once it's let
// go, and the heap never holds less than is live in it; keeping a scattered
// few leaves most of what's held beyond them in free pages, and free pages
// are never more than what's held beyond live; and the time the collector
// has spent grows with each collection; and a heap holding far more than is
// live, as one does after the launcher's closing, starts the collector as
// it makes more unless it's left room, and doesn't once it is, nor as it
// makes strings whose text is held outside the heap, nor after the
// collections that run once it is, as the shell's Qt leaves the room after
// each (qtdeclarative-room-after-every-sweep.patch). Giving the free pages
// back leaves next to none resident, says about as much as was counted, and
// what's kept, and what's made after in their place, reads as it was made.
// And a heap whose free lists lead with runs too small for what's made next,
// held off collecting, makes it in the free runs that fit rather than in new
// chunks, as the shell's Qt keeps its large runs by size
// (qtdeclarative-large-runs-by-size.patch); and small stores made next
// split the small free runs and leave the large runs whole for the large
// stores after them, reading back as they were made
// (qtdeclarative-small-runs-split-first.patch). Its census counts all that's
// live and names a shape kept by its property names, but no key taken from
// data. And what it says of jemalloc, which the test is linked with as Quickshell is: what's in use
// grows by a block kept, and what's freed and kept by one let go, and
// neither is more than what's resident. It exits 0 once all of it holds,
// and otherwise 1, saying what didn't. Long because each case reads one
// engine's heap through the same helpers, in the one program linked with
// Qt's private engine and jemalloc as the shell is: split, each part would
// build and link that again.
#include "heap.h"

#include <QCoreApplication>
#include <QJSEngine>
#include <QRegularExpression>
#include <cmath>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <limits>

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

// A room well short of the strings' 8 MB outside the heap, so making them
// must start the collector however long the room lasts.
constexpr double SMALLEST_ROOM_MB = 2;

// A room short of what a scattered heap holds, so its chunks hold more
// than twice what's live and the room besides, as Joel's shell did at 69
// MB live of 207 held (2026-11-15): there, a room counted from what's
// live rather than from what's held leaves none.
constexpr double SMALL_ROOM_MB = 4;

// How many collections run after the room is left in the cases that ask
// whether it lasts: Qt halves its limit outside the heap at each one that
// ends with little held there, so after four the 64 MB of ROOM_MB is down
// to 4, under the strings' 8 MB.
constexpr int COLLECTIONS_SINCE = 4;

// Objects made, in batches, while a few of each batch are kept, so every
// chunk the heap grows into keeps some and none goes back: ten times the
// objects the scattered heap was made from, so a heap that grew by the
// room at each collection would grow by it several times.
constexpr int GROWN = 2000000;
constexpr int GROWN_BATCH = 10000;
constexpr int GROWN_KEPT_ONE_IN = 50;

// Leaves a scattered few in the engine's heap: nearly every page that held
// what was made holds nothing once it's collected.
QJSValue keepScattered(QJSEngine &engine) {
    QJSValue few = engine.evaluate(
        QStringLiteral("(() => { const made = []; for (let i = 0; i < %1; i++) made.push({ n: i, "
                       "m: i }); return made.filter((_, i) => i % %2 === 0); })()")
            .arg(KEPT)
            .arg(SCATTERED));
    engine.collectGarbage();
    return few;
}

// Whether making them starts the collector on a heap left holding a
// scattered few, given that much room first and then `collectedSince`
// collections, as the shell runs others between its closings.
bool collectsWhileMaking(double room, const Making &made, int collectedSince = 0) {
    QJSEngine engine;
    Heap heap(&engine);
    const QJSValue few = keepScattered(engine);
    heap.leaveRoom(room);
    for (int each = 0; each < collectedSince; each++)
        engine.collectGarbage();
    const double before = heap.collected();
    QJSValue making = engine.evaluate(
        QStringLiteral("(() => { const made = []; for (let i = 0; i < %1; i++) made.push(%2); "
                       "return made; })()")
            .arg(made.count)
            .arg(QLatin1StringView(made.each)));
    return heap.collected() > before;
}

// What the heap holds, live and in its chunks, what its chunks, and what's
// held outside it, may come to before the collector starts, and what's held
// outside it, in megabytes.
struct Room {
    double live;
    double chunks;
    double collectsAt;
    double collectsOutsideAt;
    double outside;
};

Room roomOf(const Heap &heap) {
    const QVariantMap said = heap.weigh();
    return {said.value(QStringLiteral("live")).toDouble(),
            said.value(QStringLiteral("chunks")).toDouble(),
            said.value(QStringLiteral("collectsAt")).toDouble(),
            said.value(QStringLiteral("collectsOutsideAt")).toDouble(),
            said.value(QStringLiteral("outside")).toDouble()};
}

// Close enough for two readings of the same slots, in megabytes.
constexpr double SAME_MB = 0.01;

// The most room a heap is left, in megabytes, as heap.cpp holds it.
constexpr double MOST_ROOM_MB = 1024;

bool expectRoom(const char *what, bool holds, const Room &before, const Room &after) {
    if (!holds)
        std::fprintf(stderr,
                     "FAIL %s: live %.2f then %.2f MB, chunks %.2f then %.2f MB, collecting at "
                     "%.2f then %.2f MB, outside %.2f then %.2f MB collecting at %.2f then "
                     "%.2f MB\n",
                     what, before.live, after.live, before.chunks, after.chunks,
                     before.collectsAt, after.collectsAt, before.outside, after.outside,
                     before.collectsOutsideAt, after.collectsOutsideAt);
    return holds;
}

// Makes objects in batches through `grow`, which keeps a few of each in
// `kept`, until the heap's chunks hold `upTo` megabytes or GROWN are
// made, and says whether every batch ran.
bool growKeeping(const Heap &heap, const QJSValue &grow, const QJSValue &kept, double upTo) {
    for (int made = 0; made < GROWN && roomOf(heap).chunks < upTo; made += GROWN_BATCH) {
        const QJSValue batch = grow.call({kept});
        if (batch.isError()) {
            std::fprintf(stderr, "FAIL the kept objects weren't made: %s\n",
                         qPrintable(batch.toString()));
            return false;
        }
    }
    return true;
}

// Whether the room is reckoned from what the chunks hold, as the shell
// asks for it: a heap holding more than twice what's live and the room
// besides is still left the whole room after a collection; asking twice
// leaves it once; a heap grown by half the room since it asked collects
// nothing as it grows and keeps the whole room after a collection, and
// asking again as it grew, as an opening asks after the closing, doesn't
// move the ceiling; a heap whose sweeps keep what it grew into collects
// at twice the room past what it held at that first ask, its sweeps on
// misses leaving the room as a forced one does; an ask after a collection
// leaves the whole room again; asking for another room before a
// collection counts it from the same place; and the room is held to the
// most, and none for what's under nothing or isn't a number, from the
// next collection on.
bool roomFollowsWhatsHeld() {
    QJSEngine engine;
    Heap heap(&engine);
    const QJSValue few = keepScattered(engine);
    heap.leaveRoom(SMALL_ROOM_MB);
    const Room asked = roomOf(heap);
    engine.collectGarbage();
    const Room swept = roomOf(heap);
    const bool whole = expectRoom(
        "a heap holding more than twice what's live and the room besides keeps the whole room "
        "after a collection",
        swept.chunks > 2 * swept.live + SMALL_ROOM_MB &&
            swept.collectsAt >= swept.chunks + SMALL_ROOM_MB - SAME_MB,
        asked, swept);
    heap.leaveRoom(SMALL_ROOM_MB);
    const Room once = roomOf(heap);
    heap.leaveRoom(SMALL_ROOM_MB);
    const Room twice = roomOf(heap);
    const bool asOne =
        expectRoom("asking for the same room twice leaves it once, in the heap and outside it",
                   std::abs(twice.collectsAt - once.collectsAt) < SAME_MB &&
                       std::abs(twice.collectsOutsideAt - once.collectsOutsideAt) < SAME_MB,
                   once, twice);
    const QJSValue kept = engine.newArray();
    const QJSValue grow = engine.evaluate(
        QStringLiteral("(kept) => { for (let i = 0; i < %1; i++) { const made = { n: i, m: i }; "
                       "if (i % %2 === 0) kept.push(made); } }")
            .arg(GROWN_BATCH)
            .arg(GROWN_KEPT_ONE_IN));
    const double quiet = heap.collected();
    const bool grewHalf = growKeeping(heap, grow, kept, twice.chunks + SMALL_ROOM_MB / 2);
    const bool unprompted = heap.collected() == quiet;
    heap.leaveRoom(SMALL_ROOM_MB);
    engine.collectGarbage();
    const Room regrown = roomOf(heap);
    const bool grownHalf = expectRoom(
        "a heap grown by half the room since it asked collects nothing as it grows and keeps the "
        "whole room after a collection",
        grewHalf && unprompted && regrown.chunks > twice.chunks + SMALL_ROOM_MB / 4 &&
            regrown.collectsAt >= regrown.chunks + SMALL_ROOM_MB - SAME_MB,
        twice, regrown);
    const double before = heap.collected();
    const bool grewFar = growKeeping(heap, grow, kept, std::numeric_limits<double>::infinity());
    const bool collectedFar = heap.collected() > before;
    engine.collectGarbage();
    const Room grown = roomOf(heap);
    const double ceiling = twice.chunks + 2 * SMALL_ROOM_MB;
    const bool held = expectRoom("a heap whose sweeps keep what it grew into collects at twice "
                                 "the room past what it held at the first ask",
                                 grewFar && collectedFar &&
                                     grown.chunks > ceiling - SMALL_ROOM_MB / 4 &&
                                     grown.chunks < ceiling + SMALL_ROOM_MB &&
                                     2 * grown.live < ceiling &&
                                     std::abs(grown.collectsAt - ceiling) < SAME_MB,
                                 twice, grown);
    heap.leaveRoom(SMALL_ROOM_MB);
    const Room rearmed = roomOf(heap);
    const bool again = expectRoom(
        "an ask after a collection leaves the whole room again, past the ceiling before it",
        rearmed.collectsAt >= rearmed.chunks + SMALL_ROOM_MB - SAME_MB, grown, rearmed);
    const double calm = heap.collected();
    const bool grewLittle = growKeeping(heap, grow, kept, rearmed.chunks + SMALLEST_ROOM_MB);
    const Room little = roomOf(heap);
    const bool uncollected = heap.collected() == calm;
    heap.leaveRoom(SMALLEST_ROOM_MB);
    const double asking = heap.collected();
    const bool grewOn = growKeeping(heap, grow, kept, std::numeric_limits<double>::infinity());
    const bool collectedOn = heap.collected() > asking;
    engine.collectGarbage();
    const Room other = roomOf(heap);
    const double otherCeiling = rearmed.chunks + 2 * SMALLEST_ROOM_MB;
    const bool another = expectRoom(
        "asking for another room before a collection counts it from the same place",
        grewLittle && uncollected && little.chunks >= rearmed.chunks + SMALLEST_ROOM_MB &&
            grewOn && collectedOn && other.chunks > otherCeiling - SMALLEST_ROOM_MB / 4 &&
            other.chunks < otherCeiling + SMALLEST_ROOM_MB &&
            2 * other.live < otherCeiling && std::abs(other.collectsAt - otherCeiling) < SAME_MB,
        rearmed, other);
    heap.leaveRoom(std::numeric_limits<double>::infinity());
    const Room endless = roomOf(heap);
    heap.leaveRoom(2 * MOST_ROOM_MB);
    const Room past = roomOf(heap);
    const bool most = expectRoom(
        "an endless room, and one past the most, are the most",
        std::abs(endless.collectsAt - endless.chunks - MOST_ROOM_MB) < SAME_MB &&
            std::abs(past.collectsAt - endless.collectsAt) < SAME_MB,
        endless, past);
    heap.leaveRoom(-SMALL_ROOM_MB);
    engine.collectGarbage();
    const Room negative = roomOf(heap);
    heap.leaveRoom(SMALL_ROOM_MB);
    heap.leaveRoom(std::numeric_limits<double>::quiet_NaN());
    engine.collectGarbage();
    const Room notNumber = roomOf(heap);
    const bool none = expectRoom(
        "a room under nothing, and one that isn't a number, are none after a collection",
        negative.collectsAt < negative.chunks && notNumber.collectsAt < notNumber.chunks,
        negative, notNumber);
    return whole && asOne && grownHalf && held && again && another && most && none;
}

// Whether the room never starts the collector sooner than Qt itself would:
// in a heap whose chunks are nearly all live, Qt's own limit, twice what's
// live, is past anything the room gives, and so is the limit it raised for
// the strings held outside it, and both stay where Qt put them.
bool roomKeepsQtsOwnLimits() {
    QJSEngine engine;
    Heap heap(&engine);
    const QJSValue dense = engine.evaluate(
        QStringLiteral("(() => { const made = []; for (let i = 0; i < %1; i++) made.push({ n: i, "
                       "m: i }); return made; })()")
            .arg(KEPT));
    const QJSValue strings = engine.evaluate(
        QStringLiteral("(() => { const made = []; for (let i = 0; i < %1; i++) made.push(%2); "
                       "return made; })()")
            .arg(STRINGS.count)
            .arg(QLatin1StringView(STRINGS.each)));
    for (int each = 0; each < COLLECTIONS_SINCE; each++)
        engine.collectGarbage();
    const Room own = roomOf(heap);
    heap.leaveRoom(SMALLEST_ROOM_MB);
    engine.collectGarbage();
    const Room left = roomOf(heap);
    return expectRoom(
        "a room never starts the collector sooner than Qt's own limits, in the heap and outside it",
        !dense.isError() && !strings.isError() &&
            own.collectsAt > own.chunks + 2 * SMALLEST_ROOM_MB &&
            own.collectsOutsideAt > own.outside + SMALLEST_ROOM_MB &&
            left.collectsAt >= own.collectsAt - SAME_MB &&
            left.collectsOutsideAt >= own.collectsOutsideAt - SAME_MB,
        own, left);
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

// How many runs too small for what's made after are freed, each a store
// of 256 values, about 65 of the heap's 32-byte slots; and how many large
// enough, each of 1024, about 257 slots, freed before them, so the small
// ones' chunks are swept last and their runs lie first in the free lists.
// Then how many stores of 512 values, about 129 slots each, are made in
// their place. Each array is followed by an object kept, so no two of its
// runs join and no chunk is emptied. The stores made fit no run of their
// own bin, so this proves a request looks past a bin with nothing that fits
// it, not that it passes 32 runs too small in its own.
constexpr int SMALL_RUNS = 2000;
constexpr int LARGE_RUNS = 600;
constexpr int FITTING = 500;

// Whether stores made with the collector held off take the free runs large
// enough for them rather than new chunks, as the launcher makes them after
// its closing leaves room. In one list for every run of 63 slots or more, a
// request passed 32 of the small ones and took a new chunk whenever its
// tries ran out. It holds only if the runs were freed and what was made is
// live, so a Qt that sized its stores otherwise fails it rather than passing
// with nothing tried.
bool takesRunsThatFit() {
    QJSEngine engine;
    Heap heap(&engine);
    const QJSValue pins = engine.evaluate(
        QStringLiteral("(() => { const pins = [new Array(%1), new Array(%2)]; const freed = []; "
                       "for (let i = 0; i < %1; i++) { freed.push(new Array(1024)); pins[0][i] = "
                       "{}; } for (let i = 0; i < %2; i++) { freed.push(new Array(256)); "
                       "pins[1][i] = {}; } return pins; })()")
            .arg(LARGE_RUNS)
            .arg(SMALL_RUNS));
    const Weight freed = weighed(engine, heap);
    heap.leaveRoom(ROOM_MB);
    const QJSValue made = engine.evaluate(
        QStringLiteral("(() => { const made = new Array(%1); for (let i = 0; i < %1; i++) "
                       "made[i] = new Array(512); return made; })()")
            .arg(FITTING));
    const Weight after = weighed(engine, heap);
    // Each store of 512 values is about 4 KB, and each freed run about 2 or
    // 8 KB, 9 MB in all; half of that held but not live shows they're free.
    const double madeMB = FITTING * 4.0 / 1024;
    const double freedMB = (SMALL_RUNS * 2.0 + LARGE_RUNS * 8.0) / 1024;
    const bool tried = freed.held - freed.live > freedMB / 2 &&
                       after.live - freed.live > madeMB / 2;
    const bool fits = !pins.isError() && !made.isError() && tried &&
                      after.held - freed.held < madeMB / 4;
    if (!fits)
        std::fprintf(stderr,
                     "FAIL %.1f MB made with the collector held off takes the free runs that "
                     "fit it: live %.1f then %.1f MB, held %.1f then %.1f MB, free %.1f then "
                     "%.1f MB\n",
                     madeMB, freed.live, after.live, freed.held, after.held, freed.free,
                     after.free);
    return fits;
}

// How many small runs are freed, each a store of 76 values with room for
// 100, 36 slots with its array on Qt 6.12, under the 63 of the large bins;
// how many large, each of 1024 values, 260 slots; and how many small
// stores of 24 values, 12 slots, are made after, a third of a small run
// each, filling two thirds of the small runs, before as many large stores
// as large runs were freed. Each freed array is followed by an object
// kept, so no two runs join.
constexpr int SPLIT_RUNS = 3000;
constexpr int WHOLE_RUNS = 600;
constexpr int THIRDS = 6000;
constexpr int SLOT_BYTES = 32;
constexpr int LARGE_SLOTS = 63;

// How many of the heap's slots a store of so many values takes with its
// array, weighed over a few thousand of them in a heap of their own; each
// also takes a value of 8 bytes in the array that keeps them.
int slotsOf(int values) {
    constexpr int STORES = 4000;
    QJSEngine engine;
    Heap heap(&engine);
    const Weight before = weighed(engine, heap);
    const QJSValue made = engine.evaluate(
        QStringLiteral("(() => { const made = new Array(%1); for (let i = 0; i < %1; i++) "
                       "made[i] = new Array(%2); return made; })()")
            .arg(STORES)
            .arg(values));
    const Weight after = weighed(engine, heap);
    if (made.isError())
        return 0;
    const double each = (after.live - before.live) * 1024 * 1024 / STORES - 8;
    return static_cast<int>(each / SLOT_BYTES + 0.5);
}

// Whether small stores made with the collector held off split the small
// free runs larger than them, and leave the large runs whole for the large
// stores made next. A small store that found no run of its own size carved
// a large run before splitting a small one, so the large stores took new
// chunks while the small runs sat free. It holds only if the stores are
// the sizes it's built on, the runs were freed and what was made is live,
// so a Qt that sized them otherwise fails it rather than passing with
// nothing tried.
bool leavesLargeRunsWhole() {
    const int small = slotsOf(76);
    const int third = slotsOf(24);
    const int large = slotsOf(1024);
    const bool sized = small < LARGE_SLOTS && third > 0 && third * 3 <= small &&
                       large >= LARGE_SLOTS;
    if (!sized) {
        std::fprintf(stderr,
                     "FAIL the stores leaving large runs whole are sized as it's built on: a "
                     "small run of %d slots under %d, a third of %d in it, a large run of %d\n",
                     small, LARGE_SLOTS, third, large);
        return false;
    }
    QJSEngine engine;
    Heap heap(&engine);
    const QJSValue pins = engine.evaluate(
        QStringLiteral("(() => { const pins = [new Array(%1), new Array(%2)]; const freed = []; "
                       "for (let i = 0; i < %1; i++) { freed.push(new Array(1024)); pins[0][i] = "
                       "{}; } for (let i = 0; i < %2; i++) { freed.push(new Array(76)); "
                       "pins[1][i] = {}; } return pins; })()")
            .arg(WHOLE_RUNS)
            .arg(SPLIT_RUNS));
    const Weight freed = weighed(engine, heap);
    heap.leaveRoom(ROOM_MB);
    const QJSValue made = engine.evaluate(
        QStringLiteral("(() => { const made = [new Array(%1), new Array(%2)]; for (let i = 0; "
                       "i < %1; i++) { const each = new Array(24); each[0] = i; made[0][i] = "
                       "each; } for (let i = 0; i < %2; i++) { const each = new Array(1024); "
                       "each[0] = i; made[1][i] = each; } return made; })()")
            .arg(THIRDS)
            .arg(WHOLE_RUNS));
    const Weight after = weighed(engine, heap);
    const QJSValue read =
        engine
            .evaluate(QStringLiteral("(made) => made.every((stores) => stores.every((each, i) "
                                     "=> each[0] === i))"))
            .call({made});
    const double mb = SLOT_BYTES / (1024.0 * 1024.0);
    const double thirdsMB = THIRDS * third * mb;
    const double madeMB = thirdsMB + WHOLE_RUNS * large * mb;
    const double freedMB = (SPLIT_RUNS * small + WHOLE_RUNS * large) * mb;
    const bool tried = !pins.isError() && !made.isError() &&
                       freed.held - freed.live > freedMB / 2 &&
                       after.live - freed.live > madeMB / 2;
    const bool intact = read.toBool();
    const double grew = after.held - freed.held;
    const bool whole = grew < thirdsMB / 4;
    // Said whether it holds or not, so a build's log shows how far it's
    // within its bound.
    std::fprintf(stderr,
                 "heap_test: %.1f MB of small stores and %.1f MB of large ones made in freed "
                 "runs took %.2f MB of new chunks, within %.2f MB\n",
                 thirdsMB, madeMB - thirdsMB, grew, thirdsMB / 4);
    if (!tried)
        std::fprintf(stderr,
                     "FAIL the runs leaving large runs whole were freed and what's made is "
                     "live: live %.1f then %.1f MB, held %.1f then %.1f MB, %.1f MB freed\n",
                     freed.live, after.live, freed.held, after.held, freedMB);
    else if (!intact)
        std::fprintf(stderr, "FAIL the stores made in split runs read back as they were made\n");
    else if (!whole)
        std::fprintf(stderr,
                     "FAIL %.1f MB of small stores split the small free runs and leave the "
                     "large whole for %.1f MB more: live %.1f then %.1f MB, held %.1f then "
                     "%.1f MB, free %.1f then %.1f MB\n",
                     thirdsMB, madeMB - thirdsMB, freed.live, after.live, freed.held, after.held,
                     freed.free, after.free);
    return tried && intact && whole;
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

// How many objects of one shape are kept for the census to find, each two
// names no other object in the engine has; and how many of a shape whose
// second key is taken from data, as a map keyed by clipboard text is.
constexpr int SHAPED = 50000;
constexpr int KEYED = 20000;

// How many numbers the one huge array kept holds, so its store, about 2.4
// MB, is a huge item of its own outside the heap's chunks.
constexpr int HUGE_LENGTH = 300000;

// Whether the census counts what weigh() calls live, to a rounding, the
// huge item and the shapes' own allocator included; names the shape kept
// by its property names and counts every one of it; says a key from data
// as `~`; and says it all in its line.
bool namesWhatsKept() {
    QJSEngine engine;
    Heap heap(&engine);
    const QJSValue kept = engine.evaluate(
        QStringLiteral("(() => { const kept = []; for (let i = 0; i < %1; i++) kept.push({ "
                       "censusFirst: i, censusSecond: true }); for (let i = 0; i < %2; i++) "
                       "kept.push({ censusThird: i, 'from the clipboard': true }); "
                       "kept.push(new Array(%3).fill(0.5)); return kept; })()")
            .arg(SHAPED)
            .arg(KEYED)
            .arg(HUGE_LENGTH));
    const Weight weight = weighed(engine, heap);
    const QVariantMap census = heap.census(10);
    const double total = census.value(QStringLiteral("total")).toDouble();
    const QString line = census.value(QStringLiteral("said")).toString();
    const auto counts = [&](const QString &keys, int many) {
        for (const QVariant &row : census.value(QStringLiteral("shapes")).toList()) {
            const QVariantList shape = row.toList();
            if (shape.value(0).toString() == QStringLiteral("Object") &&
                shape.value(1).toString() == keys && shape.value(2).toDouble() == 2 &&
                shape.value(3).toDouble() > 0 && shape.value(4).toDouble() == many)
                return line.contains(QStringLiteral("Object {%1} ").arg(keys));
        }
        return false;
    };
    const bool named = counts(QStringLiteral("censusFirst,censusSecond"), SHAPED);
    const bool hidden = counts(QStringLiteral("censusThird,~"), KEYED) &&
                        !line.contains(QStringLiteral("clipboard"));
    const bool whole = std::abs(total - weight.live) <= 0.01;
    const bool timed = census.value(QStringLiteral("ms")).toDouble() > 0;
    if (kept.isError() || !named || !hidden || !whole || !timed)
        std::fprintf(stderr,
                     "FAIL the census counts the %.1f MB live, names %d objects kept by their "
                     "names and hides %d keys from data, and says how long it took: %.2f MB, "
                     "%s, %s, %.2f ms: %s\n",
                     weight.live, SHAPED, KEYED, total, named ? "named" : "not named",
                     hidden ? "hidden" : "not hidden",
                     census.value(QStringLiteral("ms")).toDouble(), qPrintable(line));
    return !kept.isError() && named && hidden && whole && timed;
}

// How many closures the place test keeps, each from a call of its own.
constexpr int MADE = 30000;

// Whether the census says where in the code its closures and their calls'
// contexts came from: each call's context, and the function itself, by the
// named function called, each closure by the line it's written on, every one
// counted once, in the list and in the line; and whether a file whose name
// isn't a source file's, as one named by text from data could be, is said as
// `~`, its functions' names unsaid, and its two functions on one line merged.
bool placesWhatMadeIt() {
    QJSEngine engine;
    Heap heap(&engine);
    const QJSValue made = engine.evaluate(
        QStringLiteral("function maker(i) {\n    return () => i;\n}\nconst made = [];\n"
                       "for (let i = 0; i < %1; i++) made.push(maker(i));\nmade;")
            .arg(MADE),
        QStringLiteral("/shell/made.js"));
    // Its function and each call's context, and each closure, all on line 1.
    const QJSValue odd = engine.evaluate(
        QStringLiteral("function secretName(j) { return () => j; } const odd = []; "
                       "for (let i = 0; i < %1; i++) odd.push(secretName(i)); odd;")
            .arg(MADE),
        QStringLiteral("/shell/from the clipboard"));
    const QVariantMap census = heap.census(10);
    const QString line = census.value(QStringLiteral("said")).toString();
    const auto counted = [&](const QString &place, int many) {
        for (const QVariant &row : census.value(QStringLiteral("places")).toList()) {
            const QVariantList each = row.toList();
            if (each.value(0).toString() == place && each.value(1).toDouble() > 0 &&
                each.value(2).toDouble() == many)
                return line.contains(
                    QRegularExpression(QStringLiteral("(^|, |by place )%1 [0-9.]+ MB \\(%2\\)")
                                           .arg(QRegularExpression::escape(place))
                                           .arg(many)));
        }
        return false;
    };
    const bool called = counted(QStringLiteral("made.js:1 maker"), MADE + 1);
    const bool closed = counted(QStringLiteral("made.js:2"), MADE);
    const bool hidden = counted(QStringLiteral("~:1"), 2 * MADE + 1) &&
                        !line.contains(QStringLiteral("clipboard")) &&
                        !line.contains(QStringLiteral("secretName"));
    if (made.isError() || odd.isError() || !called || !closed || !hidden)
        std::fprintf(stderr,
                     "FAIL the census says %d calls' contexts by the function called, %d "
                     "closures by their line, and a file not named as code as ~: %s, %s, %s: "
                     "%s\n",
                     MADE, MADE, called ? "called" : "not called", closed ? "closed" : "not closed",
                     hidden ? "hidden" : "not hidden", qPrintable(line));
    return !made.isError() && !odd.isError() && called && closed && hidden;
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
    const bool lasting = !collectsWhileMaking(ROOM_MB, NUMBERS, COLLECTIONS_SINCE);
    if (!lasting)
        std::fprintf(stderr, "FAIL a heap left %.0f MB of room keeps it through %d "
                     "collections as it makes more\n", ROOM_MB, COLLECTIONS_SINCE);
    const bool lastingOutside = !collectsWhileMaking(ROOM_MB, STRINGS, COLLECTIONS_SINCE);
    if (!lastingOutside)
        std::fprintf(stderr, "FAIL a heap left %.0f MB of room keeps it through %d "
                     "collections as it makes strings held outside it\n", ROOM_MB,
                     COLLECTIONS_SINCE);
    // The room outside the heap lasts but stays a room: well past it, the
    // collector still starts.
    const bool stillCollectsOutside =
        collectsWhileMaking(SMALLEST_ROOM_MB, STRINGS, COLLECTIONS_SINCE);
    if (!stillCollectsOutside)
        std::fprintf(stderr, "FAIL a heap left %.0f MB of room, %d collections ago, starts "
                     "the collector as it makes far more in strings held outside it\n",
                     SMALLEST_ROOM_MB, COLLECTIONS_SINCE);
    const bool reckoned = roomFollowsWhatsHeld();
    const bool ownLimits = roomKeepsQtsOwnLimits();
    const bool follows = allocatorFollows(heap);
    const bool given = givesBack();
    const bool fits = takesRunsThatFit();
    const bool whole = leavesLargeRunsWhole();
    const bool census = namesWhatsKept();
    const bool placed = placesWhatMadeIt();
    return grows && falls && holds && scattered && bounded && timed && collects && roomy &&
                   roomyOutside && lasting && lastingOutside && stillCollectsOutside &&
                   reckoned && ownLimits && follows && given && fits && whole && census && placed
               ? 0
               : 1;
}
