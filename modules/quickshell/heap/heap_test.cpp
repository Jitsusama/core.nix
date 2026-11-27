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
// each (qtdeclarative-room-after-every-sweep.patch), and a burst kept past
// twice the room collects about once for each room it grows by rather
// than at every miss, even with garbage made before the ask. Giving the
// free pages back leaves next to none resident, says about as much as was
// counted, and what's kept, and what's made after in their place, reads as
// it was made;
// and lending them back says as much, the kernel holds that much lent while
// they stay resident, and once it takes them, as under pressure, they're
// gone and what's kept and made after reads as it was made too.
// And a heap whose free lists lead with runs too small for what's made next,
// held off collecting, makes it in the free runs that fit rather than in new
// chunks, as the shell's Qt keeps its large runs by size
// (qtdeclarative-large-runs-by-size.patch); and small stores made next
// split the small free runs and leave the large runs whole for the large
// stores after them, reading back as they were made
// (qtdeclarative-small-runs-split-first.patch); and what's made after a
// sweep over chunks half and a quarter full fills the half-full ones, so the
// others empty and are freed once what they kept is let go
// (qtdeclarative-chunks-fullest-filled-first.patch). Its census counts all that's
// live and names a shape kept by its property names, but no key taken from
// data. Its holders count names the property that keeps the most first,
// each by the file declaring it and its folder, counts what the engine
// keeps and what it had no time to name apart, and leaves no mark behind:
// asked twice it says the same, and what was counted and then let go is
// collected; and it counts nothing during a collection or over a mark it
// didn't set. And what it says of jemalloc, which
// the test is linked with as Quickshell is: what's in use grows by a block
// kept, and what's freed and kept by one let go, and neither is more than
// what's resident. And where it stands, read without a
// walk, is where weigh() says the collector starts, and says a collection
// under way while one is left between its slices, and none once it's
// ended. It exits 0 once all of it holds, and otherwise 1, saying what
// didn't. Long because each case reads one
// engine's heap through the same helpers, in the one program linked with
// Qt's private engine and jemalloc as the shell is: split, each part would
// build and link that again.
#include "heap.h"

#include <QCoreApplication>
#include <QDir>
#include <QFile>
#include <QJSEngine>
#include <QQmlComponent>
#include <QQmlEngine>
#include <QRegularExpression>
#include <QStringList>
#include <QTemporaryDir>
#include <algorithm>
#include <chrono>
#include <cmath>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <limits>
#include <memory>
#include <private/qv4engine_p.h>
#include <private/qv4mm_p.h>
#include <sys/mman.h>
#include <unistd.h>
#include <utility>

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
    double resident;
};

Weight weighed(QJSEngine &engine, const Heap &heap) {
    engine.collectGarbage();
    const QVariantMap said = heap.weigh();
    return {said.value(QStringLiteral("live")).toDouble(),
            said.value(QStringLiteral("held")).toDouble(),
            said.value(QStringLiteral("free")).toDouble(),
            said.value(QStringLiteral("resident")).toDouble()};
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

// What the chunks held in use as the last sweep left them, in megabytes,
// twice which is where Qt itself starts a collection at a miss. Large
// items, which weigh() counts as live, have chunks of their own outside
// it.
double sweptOf(const QJSEngine &engine) {
    const QV4::MemoryManager *manager = engine.handle()->memoryManager;
    const std::size_t slotsLeft = manager->blockAllocator.usedSlotsAfterLastSweep +
                                  manager->icAllocator.usedSlotsAfterLastSweep;
    return static_cast<double>(slotsLeft * QV4::Chunk::SlotSize) / (1024.0 * 1024.0);
}

// What the last sweep had left in use when the room was last asked for
// after a collection, in megabytes, which the ceiling counts what's kept
// beyond.
double sweptWhenAskedOf(const QJSEngine &engine) {
    const QV4::MemoryManager *manager = engine.handle()->memoryManager;
    return static_cast<double>(manager->slotsSweptWhenRoomLeft * QV4::Chunk::SlotSize) /
           (1024.0 * 1024.0);
}

// What the chunks hold in use now, garbage made since the last sweep with
// it, in megabytes.
double usedNowOf(const QJSEngine &engine) {
    const QV4::MemoryManager *manager = engine.handle()->memoryManager;
    std::size_t used = 0;
    for (const QV4::BlockAllocator *allocator : {&manager->blockAllocator, &manager->icAllocator})
        for (const QV4::Chunk *chunk : allocator->chunks)
            used += chunk->nUsedSlots();
    return static_cast<double>(used * QV4::Chunk::SlotSize) / (1024.0 * 1024.0);
}

// Close enough for two readings of the same slots, in megabytes.
constexpr double SAME_MB = 0.01;

// Twice what was kept beyond what the last sweep kept at the ask, in
// megabytes, and none for less, as the ceiling moves.
double keptPast(double swept, double sweptWhenAsked) {
    return 2 * std::max(0.0, swept - sweptWhenAsked);
}

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
// at twice the room past what it held at that first ask, and twice what
// its sweeps kept beyond what the last kept then, which is never more than
// twice what grew live, so a churn holds no more, its sweeps on misses
// leaving the room as a forced one does; an ask after a collection
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
    const double sweptAsked = sweptWhenAskedOf(engine);
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
    const double cap = twice.chunks + 2 * SMALL_ROOM_MB;
    const double ceiling = cap + keptPast(sweptOf(engine), sweptAsked);
    const bool held = expectRoom(
        "a heap whose sweeps keep what it grew into collects at twice the room past what it held "
        "at the first ask, and twice what it kept since, which is never more than twice what "
        "grew live",
        grewFar && collectedFar && grown.chunks > cap - SMALL_ROOM_MB / 4 &&
            grown.chunks < ceiling + SMALL_ROOM_MB && 2 * grown.live < ceiling &&
            grown.collectsAt - cap <= 2 * (grown.live - twice.live) + SAME_MB &&
            std::abs(grown.collectsAt - ceiling) < SAME_MB,
        twice, grown);
    heap.leaveRoom(SMALL_ROOM_MB);
    const double sweptRearmed = sweptWhenAskedOf(engine);
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
    const double otherCap = rearmed.chunks + 2 * SMALLEST_ROOM_MB;
    const double otherCeiling = otherCap + keptPast(sweptOf(engine), sweptRearmed);
    const bool another = expectRoom(
        "asking for another room before a collection counts it from the same place",
        grewLittle && uncollected && little.chunks >= rearmed.chunks + SMALLEST_ROOM_MB &&
            grewOn && collectedOn && other.chunks > otherCap - SMALLEST_ROOM_MB / 4 &&
            other.chunks < otherCeiling + SMALLEST_ROOM_MB && 2 * other.live < otherCeiling &&
            other.collectsAt - otherCap <= 2 * (other.live - rearmed.live) + SAME_MB &&
            std::abs(other.collectsAt - otherCeiling) < SAME_MB,
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

// A heap made to hold far more than is live with its free runs too small
// for what's made next: objects of two values, one in HOLED_KEPT_ONE_IN
// kept, so each run a sweep leaves between them is a few slots.
constexpr int HOLED = 600000;
constexpr int HOLED_KEPT_ONE_IN = 5;

// What a burst makes and keeps, as an opening keeps its daemons' lines:
// objects of this many values, each one item of about 17 slots, more than
// any run left between the holed heap's objects, so each misses the free
// lists. It's made in batches until the chunks have grown by this many
// rooms, and never past the most.
constexpr int BURST_VALUES = 64;
constexpr int BURST_BATCH = 1000;
constexpr int BURST_ROOMS = 4;
constexpr int BURST_MOST = 200000;

// Objects of the holed heap's own two values made and dropped between
// its sweep and the ask, filling the runs that sweep left without a miss,
// as the shell makes and drops lines while the launcher is shut: more
// than the burst keeps, so a ceiling counting them as in use at the ask
// would let nothing the burst keeps move it.
constexpr int LITTER = 400000;

// How many collections the engine's collector has finished.
qint64 sweepsOf(const QJSEngine &engine) {
    return engine.handle()
        ->memoryManager->gcStateMachine->executionTiming[QV4::GCStateMachine::DoSweep]
        .count;
}

// Whether a burst kept past twice the room, growing the chunks by
// BURST_ROOMS rooms, collects at least once and no more than once for
// each room it grows by, with `litter` objects made and dropped after a
// collection and before the first ask since it, on a heap holding over
// twice what's live in runs too small for it, as Joel's shell held 201
// to 209 MB for 63 to 68 live. With none, its ask stands for the
// closing's, straight after the closing's collection. With some, it
// stands for an ask taking the baseline with garbage made since the last
// sweep: an opening's after a collection run while the launcher was shut
// or still parting, or a fresh shell's first.
// Past twice the room beyond where it was left, a limit below what's
// held would start a collection at every miss, one for each chunk the
// burst takes: two openings of Joel's shell each spent
// about 27 collections' worth inside them, 1955 and 1949 ms of their 3269
// and 3320 (2026-11-25).
bool pacesABurstPastTheRoom(int litter) {
    QJSEngine engine;
    Heap heap(&engine);
    const QJSValue holed = engine.evaluate(
        QStringLiteral("(() => { const made = []; for (let i = 0; i < %1; i++) made.push({ n: i, "
                       "m: i }); return made.filter((_, i) => i % %2 === 0); })()")
            .arg(HOLED)
            .arg(HOLED_KEPT_ONE_IN));
    // An earlier ask, then a collection with its room: the closing's, or
    // with litter to follow, one run while the launcher is shut. The ask
    // after it runs on a heap whose full sweep's count keepRoom has raised
    // past what the sweep kept, as every ask after a cycle with a room does.
    engine.collectGarbage();
    heap.leaveRoom(SMALL_ROOM_MB);
    engine.collectGarbage();
    const qint64 swept = sweepsOf(engine);
    const double sweptAtAsk = sweptOf(engine);
    const QJSValue dropped = engine.evaluate(
        QStringLiteral("(() => { let last; for (let i = 0; i < %1; i++) last = { n: i, m: i }; "
                       "return 0; })()")
            .arg(litter));
    // The litter is still counted in use at the ask, by more than the
    // burst keeps, and no collection has swept it, or the case can't tell
    // a ceiling that counts it.
    const double littered = usedNowOf(engine) - sweptAtAsk;
    const bool unswept = sweepsOf(engine) == swept &&
                         !engine.handle()->memoryManager->gcStateMachine->inProgress() &&
                         !dropped.isError() &&
                         (litter == 0 || littered > BURST_ROOMS * SMALL_ROOM_MB);
    heap.leaveRoom(SMALL_ROOM_MB);
    const Room closed = roomOf(heap);
    const double sweptClosed = sweptOf(engine);
    QStringList values;
    for (int each = 0; each < BURST_VALUES; each++)
        values << QStringLiteral("v%1: i").arg(each);
    const QJSValue kept = engine.newArray();
    const QJSValue burst = engine.evaluate(
        QStringLiteral("(kept) => { for (let i = 0; i < %1; i++) kept.push({ %2 }); }")
            .arg(BURST_BATCH)
            .arg(values.join(QStringLiteral(", "))));
    const double upTo = closed.chunks + BURST_ROOMS * SMALL_ROOM_MB;
    const qint64 before = sweepsOf(engine);
    bool made = !holed.isError() && !burst.isError();
    for (int count = 0; made && count < BURST_MOST && roomOf(heap).chunks < upTo;
         count += BURST_BATCH)
        made = !burst.call({kept}).isError();
    const qint64 ran = sweepsOf(engine) - before;
    const Room grown = roomOf(heap);
    const double sweptGrown = sweptOf(engine);
    // Held past twice what the sweeps left, and the room besides, before
    // the burst and after it, so Qt's own limit would start a collection at
    // every miss throughout, and only the room paces it.
    const bool holedThrough = closed.chunks > 2 * sweptClosed + SMALL_ROOM_MB &&
                              grown.chunks > 2 * sweptGrown;
    const bool paced = ran >= 1 && ran <= BURST_ROOMS + 1;
    if (!made || !unswept || !holedThrough || grown.chunks < upTo || !paced)
        std::fprintf(stderr,
                     "FAIL a burst kept past twice the room collects at least once and no more "
                     "than once for each room it grows by, with %d dropped before the ask: "
                     "%lld collections over %.2f to %.2f MB of chunks, up to %.2f, swept %.2f "
                     "then %.2f MB, collecting at %.2f then %.2f MB, %.2f MB unswept at the "
                     "ask; made %s, unswept %s, holed throughout %s\n",
                     litter, static_cast<long long>(ran), closed.chunks, grown.chunks, upTo,
                     sweptClosed, sweptGrown, closed.collectsAt, grown.collectsAt, littered,
                     made ? "yes" : "no", unswept ? "yes" : "no", holedThrough ? "yes" : "no");
    return made && unswept && holedThrough && grown.chunks >= upTo && paced;
}

// Whether the ceiling is twice the room past what the chunks held at the
// ask, and twice what the sweeps keep beyond what the last sweep kept
// then, which the ask takes from that sweep, read from the counters a
// sweep leaves, set as each sweep would have left them on a heap grown
// past both rooms: sweeps that keep as much cap it there, as it always
// did; sweeps that keep less cap it there too rather than wrapping past
// it; and each slot kept beyond moves it by two, as Qt lets what's used
// grow.
bool roomCapsWhatsKept() {
    QJSEngine engine;
    Heap heap(&engine);
    const QJSValue few = keepScattered(engine);
    // Garbage made after the sweep, in the runs it left, so the chunks
    // count more in use at the ask than the sweep kept, and a baseline
    // taken from them rather than from the sweep is told apart.
    const qint64 swept = sweepsOf(engine);
    const QJSValue dropped = engine.evaluate(
        QStringLiteral("(() => { let last; for (let i = 0; i < %1; i++) last = { n: i, m: i }; "
                       "return 0; })()")
            .arg(KEPT / 10));
    const double usedAtAsk = usedNowOf(engine);
    heap.leaveRoom(SMALL_ROOM_MB);
    const Room asked = roomOf(heap);
    QV4::MemoryManager *manager = engine.handle()->memoryManager;
    const std::size_t sweptAsked = manager->slotsSweptWhenRoomLeft;
    const double sweptAskedMb = static_cast<double>(sweptAsked * QV4::Chunk::SlotSize) /
                                (1024.0 * 1024.0);
    const bool fromTheSweep = !dropped.isError() && sweepsOf(engine) == swept &&
                              std::abs(sweptAskedMb - sweptOf(engine)) < SAME_MB &&
                              sweptAskedMb < usedAtAsk - SAME_MB;
    const double mb = 1024.0 * 1024.0;
    const double slotMb = static_cast<double>(QV4::Chunk::SlotSize) / mb;
    const double cap =
        static_cast<double>(manager->slotsHeldWhenRoomLeft) * slotMb + 2 * SMALL_ROOM_MB;
    QStringList values;
    for (int each = 0; each < BURST_VALUES; each++)
        values << QStringLiteral("v%1: i").arg(each);
    const QJSValue kept = engine.newArray();
    const QJSValue burst = engine.evaluate(
        QStringLiteral("(kept) => { for (let i = 0; i < %1; i++) kept.push({ %2 }); }")
            .arg(BURST_BATCH)
            .arg(values.join(QStringLiteral(", "))));
    const double upTo = asked.chunks + 3 * SMALL_ROOM_MB;
    bool made = !few.isError() && !burst.isError();
    for (int count = 0; made && count < BURST_MOST && roomOf(heap).chunks < upTo;
         count += BURST_BATCH)
        made = !burst.call({kept}).isError();
    const double grown = roomOf(heap).chunks;
    // What the sweeps left, as if they kept this many slots in all.
    const std::size_t icSwept = manager->icAllocator.usedSlotsAfterLastSweep;
    const auto collectsAtWith = [&](std::size_t swept) {
        const std::size_t ic = std::min(icSwept, swept);
        manager->icAllocator.usedSlotsAfterLastSweep = ic;
        manager->blockAllocator.usedSlotsAfterLastSweep = swept - ic;
        manager->keepRoom();
        return static_cast<double>(manager->heldBeforeCollecting()) / mb;
    };
    const std::size_t beyond = static_cast<std::size_t>(SMALL_ROOM_MB / 4 / slotMb);
    const double same = collectsAtWith(sweptAsked);
    const double less = collectsAtWith(sweptAsked / 2);
    const double more = collectsAtWith(sweptAsked + beyond);
    engine.collectGarbage();
    const double moved = 2 * static_cast<double>(beyond) * slotMb;
    const bool holds = made && fromTheSweep && grown >= upTo && std::abs(same - cap) < SAME_MB &&
                       std::abs(less - cap) < SAME_MB && std::abs(more - cap - moved) < SAME_MB;
    if (!holds)
        std::fprintf(stderr,
                     "FAIL the ceiling is twice the room past what was held at the ask, and "
                     "twice what the sweeps keep beyond what the last sweep kept then: held "
                     "%.2f MB at the ask, %.2f kept by the last sweep of %.2f in use, grown "
                     "to %.2f; "
                     "collecting at %.2f, %.2f and %.2f MB for sweeps keeping as much, half "
                     "and %.2f MB more, against %.2f, %.2f and %.2f; made %s, from the sweep "
                     "%s\n",
                     asked.chunks, sweptAskedMb, usedAtAsk, grown, same, less, more, moved / 2, cap,
                     cap,
                     cap + moved, made ? "yes" : "no", fromTheSweep ? "yes" : "no");
    return holds;
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

// A heap kept thinned to a scattered few: every SCATTERED-th of KEPT
// objects, the rest let go, so most of its pages hold nothing.
QJSValue thinnedToAFew(QJSEngine &engine) {
    return engine.evaluate(
        QStringLiteral("(() => { const made = []; for (let i = 0; i < %1; i++) made.push({ n: i, "
                       "s: 'row ' + i }); return made.filter((_, i) => i % %2 === 0); })()")
            .arg(KEPT)
            .arg(SCATTERED));
}

// What the few kept read as, and what as many made after them read as,
// each beside what it reads as when made as it was made.
struct ReadBack {
    double kept;
    double keptMade;
    double after;
    double afterMade;

    [[nodiscard]] bool asMade() const { return kept == keptMade && after == afterMade; }
};

ReadBack readBack(QJSEngine &engine, const QJSValue &few) {
    // The few are every SCATTERED-th of 0 to KEPT - 1, so their numbers sum
    // to SCATTERED times the sum of 0 to KEPT / SCATTERED - 1.
    const double keptMade = SCATTERED * (KEPT / SCATTERED) * (KEPT / SCATTERED - 1) / 2.0;
    const QJSValue kept =
        engine
            .evaluate(QStringLiteral("(few) => few.reduce((sum, each, i) => each.s === 'row ' + "
                                     "i * %1 ? sum + each.n : NaN, 0)")
                          .arg(SCATTERED))
            .call({few});
    const QJSValue after = engine.evaluate(
        QStringLiteral("(() => { const made = []; for (let i = 0; i < %1; i++) made.push({ n: "
                       "i, s: 'row ' + i }); return made.reduce((sum, each, i) => each.s === "
                       "'row ' + i ? sum + each.n : NaN, 0); })()")
            .arg(KEPT));
    return {kept.toNumber(), keptMade, after.toNumber(),
            static_cast<double>(KEPT) * (KEPT - 1) / 2.0};
}

// Whether giving a thinned heap's free pages back takes them, says so, and
// leaves what's kept, and what's made after, as it was made; and whether
// what's held resident covered what was free before and falls by more than
// half of what was given, so a count that ignored which pages are in memory
// fails.
bool givesBack() {
    QJSEngine engine;
    Heap heap(&engine);
    const QJSValue few = thinnedToAFew(engine);
    const Weight thinned = weighed(engine, heap);
    const double given = heap.giveBack();
    const Weight after = weighed(engine, heap);
    const ReadBack read = readBack(engine, few);
    const bool taken = after.free < GIVEN_LEFT_MB;
    const bool said = given > thinned.free / 2 && given <= thinned.free + GIVEN_LEFT_MB;
    const bool resident = thinned.resident >= thinned.free &&
                          thinned.resident - after.resident > given / 2;
    if (!taken || !said || !resident || !read.asMade())
        std::fprintf(stderr,
                     "FAIL giving %.1f MB of free pages back takes them and leaves what's "
                     "kept: said %.1f MB, %.1f MB free after, %.1f then %.1f MB of %.1f held "
                     "resident; the few summed %.0f of %.0f, and what was made after %.0f of "
                     "%.0f\n",
                     thinned.free, given, after.free, thinned.resident, after.resident,
                     thinned.held, read.kept, read.keptMade, read.after, read.afterMade);
    return taken && said && resident && read.asMade();
}

// Has the kernel take, now, what a heap's chunks hold lent, as it would
// when memory ran short, and says whether it would: MADV_PAGEOUT discards
// a clean lent page at once, and leaves a page Qt wrote where it is, or
// swaps it out and back unchanged.
bool takenAsUnderPressure(QJSEngine &engine) {
    const QV4::MemoryManager *manager = engine.handle()->memoryManager;
    bool took = true;
    for (const QV4::BlockAllocator *allocator :
         {&manager->blockAllocator, &manager->icAllocator})
        for (QV4::Chunk *chunk : allocator->chunks)
            took = madvise(chunk, QV4::Chunk::ChunkSize, MADV_PAGEOUT) == 0 && took;
    return took;
}

// How many pages the kernel has counted in one of /proc/vmstat's events since
// it started, across the whole machine, or -1 when it doesn't say.
double pagesCounted(const char *event) {
    QFile vmstat(QStringLiteral("/proc/vmstat"));
    if (!vmstat.open(QIODevice::ReadOnly))
        return -1;
    const QRegularExpression counted(
        QStringLiteral("^%1 (\\d+)$").arg(QString::fromLatin1(event)),
        QRegularExpression::MultilineOption);
    const QRegularExpressionMatch match = counted.match(QString::fromLatin1(vmstat.readAll()));
    return match.hasMatch() ? match.captured(1).toDouble() : -1;
}

// The pages the kernel has marked lent (pglazyfree), and the lent pages it
// has taken (pglazyfreed).
double pagesMarkedLent() { return pagesCounted("pglazyfree"); }
double lentPagesTaken() { return pagesCounted("pglazyfreed"); }

// What one lending of a thinned heap's free pages read. It's calm when the
// kernel took no lent page anywhere while the lending was being read, so
// what it holds lent and what stays resident can be judged; a kernel that
// doesn't count what it takes is taken as calm, so the case judges rather
// than skips. And it says how many MB the kernel marked lent as the free
// pages were lent, which a take afterwards can't undo, or -1 when it
// doesn't count them.
struct Lending {
    double free = 0;
    double lent = 0;
    double lentBefore = 0;
    double lentAfter = 0;
    double freeLent = 0;
    double freeTaken = 0;
    double residentLent = 0;
    double residentTaken = 0;
    double marked = -1;
    bool took = false;
    bool counted = false;
    bool calm = false;
    ReadBack read{};

    // Whether it said about as much as was counted free.
    [[nodiscard]] bool saidAsCounted() const {
        return lent > free / 2 && lent <= free + GIVEN_LEFT_MB;
    }

    // Whether the kernel marked at least half as many pages lent, which
    // giving them back never does, where it counts them.
    [[nodiscard]] bool markedAsLent() const { return marked < 0 || marked > free / 2; }

    // Whether the kernel's take left next to none of the free pages resident.
    [[nodiscard]] bool goneOnceTaken() const { return took && freeTaken < GIVEN_LEFT_MB; }

    // Whether it said what was counted, was marked lent, was gone once
    // taken, and left what's kept and made after reading as made: what
    // holds whether or not the kernel took lent pages early, so both tries
    // are judged by it.
    [[nodiscard]] bool asLent() const {
        return saidAsCounted() && markedAsLent() && goneOnceTaken() && read.asMade();
    }
};

// Thins a heap, lends its free pages back and reads what the kernel then
// holds lent and what stays resident; has the kernel take them, as under
// pressure, and reads what's kept and what's made after. Its free pages are
// read without a collection, which would write the free runs' headers
// again.
Lending lentOnce() {
    QJSEngine engine;
    Heap heap(&engine);
    const QJSValue few = thinnedToAFew(engine);
    Lending lending;
    lending.free = weighed(engine, heap).free;
    const double takenBefore = lentPagesTaken();
    lending.lentBefore = heap.lent();
    const double markedBefore = pagesMarkedLent();
    lending.lent = heap.lendBack();
    const double markedAfter = pagesMarkedLent();
    lending.lentAfter = heap.lent();
    const QVariantMap lent = heap.weigh();
    lending.freeLent = lent.value(QStringLiteral("free")).toDouble();
    lending.residentLent = lent.value(QStringLiteral("resident")).toDouble();
    const double takenAfter = lentPagesTaken();
    lending.counted = takenBefore >= 0 && takenAfter >= 0;
    lending.calm = !lending.counted || takenAfter == takenBefore;
    if (markedBefore >= 0 && markedAfter >= 0)
        lending.marked = (markedAfter - markedBefore) * static_cast<double>(sysconf(_SC_PAGESIZE)) /
                         (1024.0 * 1024.0);
    lending.took = takenAsUnderPressure(engine);
    const QVariantMap taken = heap.weigh();
    lending.freeTaken = taken.value(QStringLiteral("free")).toDouble();
    lending.residentTaken = taken.value(QStringLiteral("resident")).toDouble();
    lending.read = readBack(engine, few);
    return lending;
}

// Whether lending a thinned heap's free pages back says about as much as
// was counted, and the kernel holds that much lent while the pages stay
// resident, which giving them back would not; and whether, once the kernel
// takes them as it would under pressure, they're no longer resident and
// what's kept, and what's made after, still reads as it was made, so no
// page lent held a live item. Where it counts them, the kernel must also
// have marked at least half as many pages lent, which giving them back
// never does, and which a take afterwards can't undo. A builder short of
// memory may take lent pages while they're read, as lending allows, so a
// lending the kernel took from is tried once more, and if it takes again,
// what it holds lent and what stays resident are left unjudged, with a
// note. What it said, marked and left once taken, and what reads back, are
// judged through asLent() on a first try taken from and on the try judged.
bool lendsBack() {
    const Lending first = lentOnce();
    const Lending lending = first.calm ? first : lentOnce();
    const bool firstAsLent = first.calm || first.asLent();
    const bool asLent = lending.asLent();
    const bool held = !lending.calm || (lending.lentBefore >= 0 &&
                                        lending.lentAfter - lending.lentBefore > lending.free / 2);
    const bool kept = !lending.calm || lending.freeLent > lending.free / 2;
    // What's held counted resident falls by what the kernel took, so a count
    // that ignored which pages are in memory fails; a take the kernel made
    // early has already lowered it, so only a calm lending is judged.
    const bool fell =
        !lending.calm || lending.residentLent - lending.residentTaken > lending.lent / 2;
    const ReadBack &read = lending.read;
    if (!lending.counted)
        std::fprintf(stderr,
                     "note: the kernel doesn't say how many lent pages it took "
                     "(/proc/vmstat's pglazyfreed), so the lending is judged as if no page "
                     "were taken early\n");
    if (lending.marked < 0)
        std::fprintf(stderr, "note: the kernel doesn't say how many pages it marked lent "
                             "(/proc/vmstat's pglazyfree), so that goes unjudged\n");
    if (!lending.calm)
        std::fprintf(stderr,
                     "note: the kernel took lent pages twice while lending was read, so what "
                     "it held lent (%.1f, then %.1f MB), what stayed resident (%.1f MB) and "
                     "how far what's held resident fell once taken (%.1f to %.1f MB) go "
                     "unjudged; it marked %.1f MB lent, or -1 if it doesn't say\n",
                     lending.lentBefore, lending.lentAfter, lending.freeLent,
                     lending.residentLent, lending.residentTaken, lending.marked);
    if (!firstAsLent)
        std::fprintf(stderr,
                     "FAIL the first lending, taken from early, said %.1f MB of %.1f free, "
                     "marked %.1f MB lent (-1 if uncounted), %.1f MB free once taken (%s); the "
                     "few summed %.0f of %.0f, and what was made after %.0f of %.0f\n",
                     first.lent, first.free, first.marked, first.freeTaken,
                     first.took ? "taken" : "the kernel refused", first.read.kept,
                     first.read.keptMade, first.read.after, first.read.afterMade);
    if (!asLent || !held || !kept || !fell)
        std::fprintf(stderr,
                     "FAIL lending %.1f MB of free pages back says so, keeps them resident "
                     "until the kernel takes them, and leaves what's kept: said %.1f MB; the "
                     "kernel held %.1f, then %.1f MB lent, and marked %.1f MB (-1 if uncounted); "
                     "%.1f MB free once lent, and %.1f once taken (%s); %.1f MB held "
                     "resident once lent, and %.1f once taken; the few summed %.0f of %.0f, "
                     "and what was made after %.0f of %.0f\n",
                     lending.free, lending.lent, lending.lentBefore, lending.lentAfter,
                     lending.marked, lending.freeLent, lending.freeTaken,
                     lending.took ? "taken" : "the kernel refused", lending.residentLent,
                     lending.residentTaken, read.kept, read.keptMade, read.after, read.afterMade);
    return firstAsLent && asLent && held && kept && fell;
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

// How many objects are made in each of two runs of chunks, the first
// keeping three of every six, half full, and the second, made after it, one
// of every four, a quarter full, so the free runs in both are three objects
// long and fall in the one bin; and how many are made once they're swept,
// as many as the half-full chunks have room for and fewer than the
// quarter-full ones have.
constexpr int HALF_FULL = 120000;
constexpr int QUARTER_FULL = 120000;
constexpr int FILLING = HALF_FULL / 2;

// Whether what's made after a sweep fills the fullest chunks first, so the
// chunks using little are left to empty and be freed once what they keep
// is let go. Each chunk's runs went on the front of the bins in the order
// the chunks lay, so the chunks made last were filled first whatever they
// held, and what lived long scattered over chunks nearly empty, none of
// them ever freed: in five hours Joel's shell held 284 MB around 72 live
// (2026-11-25). It holds only if the quarter-full chunks were made and
// what's made is live, so a Qt that collected while making them, filling
// them in, fails it rather than passing with nothing tried.
bool fillsTheFullestFirst() {
    QJSEngine engine;
    Heap heap(&engine);
    heap.leaveRoom(ROOM_MB);
    const QJSValue kept = engine.evaluate(
        QStringLiteral("(() => { const kept = { half: new Array(%1), quarter: new Array(%2), "
                       "made: new Array(%3) }; let j = 0; for (let i = 0; i < %4; i++) { "
                       "const each = { n: i }; if (i % 6 < 3) kept.half[j++] = each; } "
                       "return kept; })()")
            .arg(HALF_FULL / 2)
            .arg(QUARTER_FULL / 4)
            .arg(FILLING)
            .arg(HALF_FULL));
    const double halfChunks = heap.standing().value(QStringLiteral("chunks")).toDouble();
    const QJSValue quarter = engine.evaluate(
        QStringLiteral("(kept) => { let j = 0; for (let i = 0; i < %1; i++) { const each = { "
                       "n: i }; if (i % 4 === 0) kept.quarter[j++] = each; } }")
            .arg(QUARTER_FULL));
    const QJSValue quartered = quarter.call({kept});
    const double quarterMB =
        heap.standing().value(QStringLiteral("chunks")).toDouble() - halfChunks;
    const Weight swept = weighed(engine, heap);
    heap.leaveRoom(ROOM_MB);
    const QJSValue made =
        engine
            .evaluate(QStringLiteral("(kept) => { for (let i = 0; i < %1; i++) kept.made[i] = { "
                                     "n: i }; }")
                          .arg(FILLING))
            .call({kept});
    // Weighed before the quarter-kept go, which would take from live as much
    // again as half of what's made.
    const Weight filled = weighed(engine, heap);
    const QJSValue dropped =
        engine.evaluate(QStringLiteral("(kept) => { kept.quarter = null; }")).call({kept});
    const Weight after = weighed(engine, heap);
    const QJSValue read =
        engine
            .evaluate(QStringLiteral("(kept) => kept.made.every((each, i) => each.n === i) && "
                                     "kept.half.every((each, i) => each.n === (i / 3 | 0) * "
                                     "6 + i % 3)"))
            .call({kept});
    // Each object takes the same slots, so what's made is to the quarter-full
    // chunks as how many are made is to how many were made in them.
    const double madeMB = quarterMB * FILLING / QUARTER_FULL;
    const bool ran =
        !kept.isError() && !quartered.isError() && !made.isError() && !dropped.isError();
    if (!ran)
        std::fprintf(stderr, "FAIL the chunks half and a quarter full were made and filled: %s\n",
                     qPrintable((kept.isError()        ? kept
                                 : quartered.isError() ? quartered
                                 : made.isError()      ? made
                                                       : dropped)
                                    .toString()));
    const bool tried = ran && quarterMB > 0 && swept.held - swept.live > quarterMB / 2 &&
                       filled.live - swept.live > madeMB / 2;
    const bool intact = read.toBool();
    const double beyond = after.held - after.live;
    const bool freed = beyond < quarterMB / 2;
    // Said whether it holds or not, so a build's log shows how far it's
    // within its bound.
    std::fprintf(stderr,
                 "heap_test: %.1f MB made after a sweep over chunks half and a quarter full "
                 "left %.2f MB held beyond live, within %.2f MB\n",
                 madeMB, beyond, quarterMB / 2);
    if (!tried)
        std::fprintf(stderr,
                     "FAIL the chunks a quarter full were made and what's made after is live: "
                     "%.1f MB of them, then %.1f MB made: live %.1f then %.1f MB, held %.1f "
                     "then %.1f MB\n",
                     quarterMB, madeMB, swept.live, filled.live, swept.held, filled.held);
    else if (!intact)
        std::fprintf(stderr, "FAIL what's made in the fullest chunks reads back as it was made\n");
    else if (!freed)
        std::fprintf(stderr,
                     "FAIL %.1f MB made after a sweep fills the half-full chunks and leaves "
                     "the %.1f MB a quarter full to be freed: live %.1f then %.1f MB, held %.1f "
                     "then %.1f MB, free %.1f then %.1f MB\n",
                     madeMB, quarterMB, swept.live, after.live, swept.held, after.held,
                     swept.free, after.free);
    return tried && intact && freed;
}

// How many stores of 1024 values, 260 slots each, about seven to a chunk,
// are made in each of two runs of chunks, the first keeping three of every
// eight and the second one, so most chunks of both keep one free run of
// 1024 slots or more, in the last bin, beside chunks using more and less.
constexpr int LONG_RUN_STORES = 800;

// Whether a sweep leaves every bin's runs fullest chunk first, the last bin
// among them, read straight after it, before anything is made to take one.
// A small request no smaller run serves takes the head of the first large
// bin holding a run, the last bin among them, and bumps through the rest,
// so with a bin's runs the other way round the emptiest chunks were filled
// first and none drained; no case that weighs what's made reaches the last.
// It holds only if the last bin has runs of chunks using both more and
// less, so a layout that left none there fails it rather than passing with
// nothing tried.
bool sortsEachBinFullestFirst() {
    QJSEngine engine;
    Heap heap(&engine);
    heap.leaveRoom(ROOM_MB);
    const QJSValue kept = engine.evaluate(
        QStringLiteral("(() => { const kept = [new Array(%1), new Array(%2)]; let j = 0; let "
                       "k = 0; for (let i = 0; i < %3; i++) { const each = new Array(1024); if "
                       "(i % 8 < 3) kept[0][j++] = each; } for (let i = 0; i < %3; i++) { const "
                       "each = new Array(1024); if (i % 8 === 0) kept[1][k++] = each; } return "
                       "kept; })()")
            .arg(LONG_RUN_STORES * 3 / 8)
            .arg(LONG_RUN_STORES / 8)
            .arg(LONG_RUN_STORES));
    engine.collectGarbage();
    const QV4::MemoryManager *manager = engine.handle()->memoryManager;
    using Allocator = QV4::BlockAllocator;
    bool ordered = true;
    const char *outOfOrder = "";
    std::size_t badBin = 0;
    uint badBefore = 0;
    uint badAfter = 0;
    int longRuns = 0;
    uint leastUsed = std::numeric_limits<uint>::max();
    uint mostUsed = 0;
    for (const auto &[name, allocator] :
         {std::pair<const char *, const Allocator *>{"objects'", &manager->blockAllocator},
          std::pair<const char *, const Allocator *>{"shapes'", &manager->icAllocator}})
        for (std::size_t bin = 0; bin < Allocator::NumBins; ++bin) {
            uint before = std::numeric_limits<uint>::max();
            for (const QV4::HeapItem *m = allocator->freeBins[bin]; m; m = m->freeData.next) {
                const uint used = m->chunk()->nUsedSlots();
                if (used > before && ordered) {
                    ordered = false;
                    outOfOrder = name;
                    badBin = bin;
                    badBefore = before;
                    badAfter = used;
                }
                before = used;
                if (allocator == &manager->blockAllocator && bin == Allocator::NumBins - 1) {
                    ++longRuns;
                    leastUsed = std::min(leastUsed, used);
                    mostUsed = std::max(mostUsed, used);
                }
            }
        }
    // Read only once the bins are walked, since what it makes would take
    // runs from their heads and push what's left back on.
    const QJSValue read =
        engine
            .evaluate(QStringLiteral("(kept) => kept[0].length === %1 && kept[1].length === %2 "
                                     "&& kept.every((stores) => stores.every((each) => "
                                     "each.length === 1024))")
                          .arg(LONG_RUN_STORES * 3 / 8)
                          .arg(LONG_RUN_STORES / 8))
            .call({kept});
    // A chunk keeping three of its seven or eight stores uses about 760
    // slots, and one keeping one about 260.
    const bool tried =
        !kept.isError() && longRuns >= LONG_RUN_STORES / 80 && leastUsed < 400 && mostUsed > 600;
    const bool intact = read.toBool();
    if (!tried)
        std::fprintf(stderr,
                     "FAIL chunks using more and less were swept with long runs: the last bin "
                     "held %d runs, of chunks using %u to %u slots%s\n",
                     longRuns, longRuns > 0 ? leastUsed : 0, mostUsed,
                     kept.isError() ? qPrintable(QStringLiteral(", ") + kept.toString()) : "");
    else if (!intact)
        std::fprintf(stderr, "FAIL the long stores read back as they were made%s\n",
                     read.isError() ? qPrintable(QStringLiteral(": ") + read.toString()) : "");
    else if (!ordered)
        std::fprintf(stderr,
                     "FAIL each bin's runs come fullest chunk first after a sweep: in bin %zu "
                     "of the %s allocator a chunk using %u slots came after one using %u, and "
                     "the last bin held %d runs, of chunks using %u to %u slots\n",
                     badBin, outOfOrder, badAfter, badBefore, longRuns, leastUsed, mostUsed);
    return tried && intact && ordered;
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

// How many numbers the one huge array kept holds, so its store, past 2 MB,
// is a huge item with a segment of its own outside the heap's chunks.
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

// How many objects of a shape of their own are kept, each needing an
// internal class from the shapes' own allocator; and how many large items
// of LARGE_VALUES numbers, whose stores stay under 2 MB as an array's grows
// by doubling, so the chunk allocator makes them, beside one of
// HUGE_LENGTH, with a segment of its own. On 25 November 150000 numbers
// grew past 2 MB, and every large item had a segment of its own.
constexpr int OWN_SHAPES = 50000;
constexpr int LARGE_ITEMS = 6;
constexpr int LARGE_VALUES = 100000;

double megabytesOf(std::size_t bytes) { return static_cast<double>(bytes) / (1024.0 * 1024.0); }

// The MB of this process's pages the kernel has swapped out, zswap's pool
// included, since each is left a swap entry wherever it's kept, or -1 when
// it doesn't say.
double swappedOut() {
    QFile status(QStringLiteral("/proc/self/status"));
    if (!status.open(QIODevice::ReadOnly))
        return -1;
    const QRegularExpressionMatch match =
        QRegularExpression(QStringLiteral("^VmSwap:\\s+(\\d+) kB$"),
                           QRegularExpression::MultilineOption)
            .match(QString::fromLatin1(status.readAll()));
    return match.hasMatch() ? match.captured(1).toDouble() / 1024.0 : -1;
}

// Whether what's held resident covers what's live, and passes what's held
// by no more than its headers, once a heap keeps items in every allocator
// it has: objects in its chunks, their shapes' internal classes in the
// shapes' own, and large items of both kinds, which it checks it made.
// Each item live was written, so its pages are in memory, and a count that
// left out an allocator falls short of it; one that read a chunk twice
// passes what's held, and one past a large item's own may, where the pages
// after it are resident. What's held counts a chunk less its header, and a
// large item the chunk allocator made at its size, short of its header, the
// page it ends in and the rest of a whole chunk, so those are let over. A
// read a page or less past a large item's chunk isn't caught, and nor is
// one stopping short of its end, since the item's own bytes, which live
// counts, come first. A page of this process the kernel swapped out is no
// longer resident, so what's live is judged less what it has swapped.
bool residentCoversWhatsLive() {
    QJSEngine engine;
    Heap heap(&engine);
    const QJSValue kept = engine.evaluate(
        QStringLiteral("(() => { const kept = []; for (let i = 0; i < %1; i++) kept.push({ "
                       "['own' + i]: i }); for (let i = 0; i < %2; i++) kept.push(new "
                       "Array(%3).fill(i)); kept.push(new Array(%4).fill(0.5)); return kept; "
                       "})()")
            .arg(OWN_SHAPES)
            .arg(LARGE_ITEMS)
            .arg(LARGE_VALUES)
            .arg(HUGE_LENGTH));
    const Weight weight = weighed(engine, heap);
    const double swapped = std::max(swappedOut(), 0.0);
    const QV4::MemoryManager *manager = engine.handle()->memoryManager;
    const double shapes = megabytesOf(manager->icAllocator.usedMem());
    const double large = megabytesOf(manager->getLargeItemsMem());
    const auto page = static_cast<std::size_t>(sysconf(_SC_PAGESIZE));
    // Only the stores of LARGE_VALUES numbers, not `kept`'s own, which the
    // chunk allocator makes too.
    int chunked = 0;
    int segmented = 0;
    std::size_t over = (manager->blockAllocator.chunks.size() +
                        manager->icAllocator.chunks.size()) *
                       QV4::Chunk::HeaderSize;
    for (const auto &huge : manager->hugeItemAllocator.chunks) {
        if (huge.segment != nullptr) {
            ++segmented;
            continue;
        }
        if (huge.size >= static_cast<std::size_t>(LARGE_VALUES) * sizeof(QV4::Value))
            ++chunked;
        over += QV4::Chunk::HeaderSize + page +
                (huge.size < QV4::Chunk::ChunkSize ? QV4::Chunk::ChunkSize - huge.size : 0);
    }
    const bool made = shapes > 1 && chunked >= LARGE_ITEMS && segmented >= 1;
    const bool covers = weight.resident >= 0 && weight.resident + swapped >= weight.live;
    const bool bounded = weight.resident <= weight.held + megabytesOf(over);
    if (kept.isError() || !made || !covers || !bounded)
        std::fprintf(stderr,
                     "FAIL what's held resident covers the %.1f MB live, less %.1f MB swapped, "
                     "%.1f MB of it shapes and %.1f MB large items, %d made by the chunk "
                     "allocator and %d with a segment, and passes the %.1f MB held by at most "
                     "%.2f MB: %.1f MB resident\n",
                     weight.live, swapped, shapes, large, chunked, segmented, weight.held,
                     megabytesOf(over), weight.resident);
    return !kept.isError() && made && covers && bounded;
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

// Whether standing() says what weigh() does of where the collector starts,
// without its walk, on a scattered heap left room, and says no collection
// is under way once one has run to its end.
bool standsAsWeighed() {
    QJSEngine engine;
    Heap heap(&engine);
    const QJSValue few = keepScattered(engine);
    heap.leaveRoom(SMALL_ROOM_MB);
    engine.collectGarbage();
    const QVariantMap weighs = heap.weigh();
    const QVariantMap stands = heap.standing();
    const auto same = [&weighs, &stands](const char *name) {
        const QString key = QString::fromLatin1(name);
        return stands.contains(key) &&
               std::abs(stands.value(key).toDouble() - weighs.value(key).toDouble()) < SAME_MB;
    };
    const bool agrees =
        same("chunks") && same("collectsAt") && same("outside") && same("collectsOutsideAt");
    const QVariant collecting = stands.value(QStringLiteral("collecting"));
    const bool idle = collecting.typeId() == QMetaType::Bool && !collecting.toBool();
    if (!agrees || !idle)
        std::fprintf(stderr,
                     "FAIL standing() says what weigh() does of the collector's limits, and no "
                     "collection under way after one: chunks %.2f and %.2f MB, collecting at "
                     "%.2f and %.2f MB, outside %.2f and %.2f MB collecting at %.2f and %.2f "
                     "MB, collecting %s\n",
                     stands.value(QStringLiteral("chunks")).toDouble(),
                     weighs.value(QStringLiteral("chunks")).toDouble(),
                     stands.value(QStringLiteral("collectsAt")).toDouble(),
                     weighs.value(QStringLiteral("collectsAt")).toDouble(),
                     stands.value(QStringLiteral("outside")).toDouble(),
                     weighs.value(QStringLiteral("outside")).toDouble(),
                     stands.value(QStringLiteral("collectsOutsideAt")).toDouble(),
                     weighs.value(QStringLiteral("collectsOutsideAt")).toDouble(),
                     qPrintable(collecting.toString()));
    return agrees && idle;
}

// So short a slice that a cycle stops after its first step, as one the
// event loop steps on is left between its slices.
constexpr std::chrono::microseconds A_MOMENT{1};

// Whether standing() says a collection is under way while one is left
// between its slices, and none once it's run to its end: read only after
// one had ended, a standing() that never said one under way passed.
bool saysOneUnderWay() {
    QJSEngine engine;
    Heap heap(&engine);
    const QJSValue few = keepScattered(engine);
    QV4::MemoryManager *manager = engine.handle()->memoryManager;
    const std::chrono::microseconds slice = manager->gcStateMachine->timeLimit;
    manager->gcStateMachine->timeLimit = A_MOMENT;
    manager->runGC();
    const bool running = manager->gcStateMachine->inProgress();
    const QVariant during = heap.standing().value(QStringLiteral("collecting"));
    manager->gcStateMachine->timeLimit = slice;
    engine.collectGarbage();
    const QVariant after = heap.standing().value(QStringLiteral("collecting"));
    const bool said = running && during.toBool() && after.typeId() == QMetaType::Bool &&
                      !after.toBool();
    if (!said)
        std::fprintf(stderr,
                     "FAIL standing() says a collection is under way between its slices, and "
                     "none once it's ended: left running %s, said %s during and %s after\n",
                     running ? "yes" : "no", qPrintable(during.toString()),
                     qPrintable(after.toString()));
    return said;
}

// How many items the holders test's largest property keeps, a dictionary
// another object holds, and its smallest.
constexpr int HOLDS_MOST = 40000;
constexpr int KEEPS = 5000;
constexpr int HOLDS_LITTLE = 100;

// Asks holders() for every holder it names, so their parts add up, with no
// time limit to naming them, so the case never reads a busy builder's clock.
constexpr int EVERY = 1000;
constexpr int UNHURRIED_MS = 10000000;

// Whether any mark is set in the engine's heap, read from every chunk's
// own bitmap rather than as holders() reads them.
bool anyMarked(const QV4::MemoryManager *manager) {
    const auto marked = [](const QV4::Chunk *chunk) {
        for (const quintptr word : chunk->blackBitmap) {
            if (word != 0)
                return true;
        }
        return false;
    };
    for (const QV4::Chunk *chunk : manager->blockAllocator.chunks) {
        if (marked(chunk))
            return true;
    }
    for (const QV4::Chunk *chunk : manager->icAllocator.chunks) {
        if (marked(chunk))
            return true;
    }
    for (const auto &huge : manager->hugeItemAllocator.chunks) {
        if (marked(huge.chunk))
            return true;
    }
    return false;
}

// What a holder in the holders' list says: its megabytes and count, or none.
struct Holding {
    double mb = 0;
    double count = 0;
    int rank = -1;
};

Holding holding(const QVariantMap &holders, const QString &key) {
    const QVariantList rows = holders.value(QStringLiteral("holders")).toList();
    for (int at = 0; at < rows.size(); ++at) {
        const QVariantList row = rows.at(at).toList();
        if (row.value(0).toString() == key)
            return {row.value(1).toDouble(), row.value(2).toDouble(), at};
    }
    return {};
}

// How many items the property a file's type declares keeps, where it's used
// with more declared beside it.
constexpr int OWN = 2000;

// How many numbers one property keeps in a single array, whose values, at
// 8 bytes each, are one huge item of about 1.6 MB, past a chunk's 64 KB.
constexpr int HUGE_ITEMS = 200000;
constexpr double HUGE_MB = 1.5;

// Writes one of the holders test's QML files.
bool written(const QDir &into, const QString &name, const QString &text) {
    QFile file(into.filePath(name));
    return file.open(QIODevice::WriteOnly) && file.write(text.toUtf8()) >= 0;
}

// Whether holders() names the property keeping the most first, counting
// each of its objects and their strings; names one held by an object
// declared inside the file by that file, its dictionary's arrays counted,
// and not the property holding that object; names a property by the file
// that declares it, a type's own by its file and one added where it's used
// by the file using it; names a file's functions; says a file named from
// data as `~` and none of its name; counts a small one apart; names no
// property holding a number; counts all that's live, each item once; says
// the same asked twice, so it cleared the marks it set; and counts for the
// largest about what letting it go frees, the next collection taking it.
// The files sit in a folder named as code is, so each is said after it.
bool namesWhatHoldsIt() {
    QTemporaryDir shell;
    QDir files(shell.path());
    const bool wrote =
        files.mkdir(QStringLiteral("pieces")) && files.cd(QStringLiteral("pieces")) &&
        written(files, QStringLiteral("Base.qml"),
                QStringLiteral("import QtQml\nQtObject {\n    property var own: []\n"
                               "    Component.onCompleted: {\n"
                               "        for (let i = 0; i < %1; i++) own.push({ o: i });\n"
                               "    }\n}\n")
                    .arg(OWN)) &&
        written(files, QStringLiteral("Holding.qml"),
                QStringLiteral("import QtQml\nBase {\n    property var holdsMost: []\n"
                               "    property var holdsLittle: []\n    property var holdsHuge: []\n"
                               "    property var notData: 4\n"
                               "    property QtObject inner:\n"
                               "        QtObject { property var keeps: ({}) }\n"
                               "    function letGo() { holdsMost = []; }\n"
                               "    Component.onCompleted: {\n"
                               "        for (let i = 0; i < %1; i++)\n"
                               "            holdsMost.push({ n: i, s: 'w' + i });\n"
                               "        for (let i = 0; i < %2; i++) inner.keeps['k' + i] = [i];\n"
                               "        for (let i = 0; i < %3; i++) holdsLittle.push({ n: i });\n"
                               "        for (let i = 0; i < %4; i++) holdsHuge.push(i);\n"
                               "    }\n}\n")
                    .arg(HOLDS_MOST)
                    .arg(KEEPS)
                    .arg(HOLDS_LITTLE)
                    .arg(HUGE_ITEMS)) &&
        written(files, QStringLiteral("from the clipboard.qml"),
                QStringLiteral("import QtQml\n"
                               "QtObject { property var pasted: [{ a: 1 }, { a: 2 }] }\n"));
    if (!wrote) {
        std::fprintf(stderr, "FAIL the holders' files weren't written in %s\n",
                     qPrintable(shell.path()));
        return false;
    }
    QQmlEngine engine;
    Heap heap(&engine);
    QQmlComponent component(&engine,
                            QUrl::fromLocalFile(files.filePath(QStringLiteral("Holding.qml"))));
    QQmlComponent pasting(
        &engine, QUrl::fromLocalFile(files.filePath(QStringLiteral("from the clipboard.qml"))));
    const std::unique_ptr<QObject> made(component.create());
    const std::unique_ptr<QObject> pasted(pasting.create());
    if (!made || !pasted) {
        std::fprintf(stderr, "FAIL the holders' objects weren't made: %s %s\n",
                     qPrintable(component.errorString()), qPrintable(pasting.errorString()));
        return false;
    }
    // Weighed before it's counted, and let go before any collection after,
    // since a collection's sweep clears every mark: a mark holders() left
    // would keep what it marked through the one that follows.
    const double before = weighed(engine, heap).live;
    const QVariantMap first = heap.holders(EVERY, UNHURRIED_MS);
    const QVariantMap second = heap.holders(EVERY, UNHURRIED_MS);
    // With no time to name any, what the properties hold is said as unnamed,
    // not as held from C++.
    const QVariantMap hurried = heap.holders(EVERY, 0);
    const bool cleared = !anyMarked(engine.handle()->memoryManager);
    const QString line = first.value(QStringLiteral("said")).toString();
    const Holding most = holding(first, QStringLiteral("pieces/Holding.qml holdsMost"));
    const Holding keeps = holding(first, QStringLiteral("pieces/Holding.qml keeps"));
    const Holding little = holding(first, QStringLiteral("pieces/Holding.qml holdsLittle"));
    const Holding huge = holding(first, QStringLiteral("pieces/Holding.qml holdsHuge"));
    const Holding own = holding(first, QStringLiteral("pieces/Base.qml own"));
    const Holding functions = holding(first, QStringLiteral("pieces/Holding.qml (functions)"));
    const Holding fromData = holding(first, QStringLiteral("~ pasted"));
    const Holding mostAgain = holding(second, QStringLiteral("pieces/Holding.qml holdsMost"));
    const bool ranked = most.rank == 0 && most.count >= 2 * HOLDS_MOST && most.mb > keeps.mb;
    // The object `inner` holds is no property's, so what its own property
    // keeps is counted for that one rather than for `inner`.
    const bool inner = keeps.rank > 0 && keeps.count >= 2 * KEEPS &&
                       holding(first, QStringLiteral("pieces/Holding.qml inner")).rank < 0;
    // The first property in name order is counted for its own items and not
    // the engine's builtins, which every plain object reaches.
    const bool byFile = own.count >= OWN && own.count < OWN + OWN / 4 &&
                        holding(first, QStringLiteral("pieces/Holding.qml own")).rank < 0 &&
                        functions.count >= 1 && huge.mb >= HUGE_MB;
    const bool hidden = fromData.count >= 2 && !line.contains(QStringLiteral("clipboard"));
    // Each item counted once: what it counts for each part comes to what's
    // live, with nothing over, and that's what weigh() says is live.
    double parts = 0;
    for (const QVariant &row : first.value(QStringLiteral("holders")).toList())
        parts += row.toList().value(1).toDouble();
    for (const char *part : {"themselves", "engine", "unnamed", "contexts", "objects", "rest"})
        parts += first.value(QString::fromLatin1(part)).toDouble();
    const double total = first.value(QStringLiteral("total")).toDouble();
    const double rest = first.value(QStringLiteral("rest")).toDouble();
    const bool whole =
        std::abs(total - before) < 0.05 && std::abs(parts - total) < 0.05 &&
        first.value(QStringLiteral("named")) == first.value(QStringLiteral("names")) &&
        first.value(QStringLiteral("engine")).toDouble() > 0;
    const bool unnamed = hurried.value(QStringLiteral("named")).toInt() == 0 &&
                         hurried.value(QStringLiteral("unnamed")).toDouble() >= most.mb &&
                         std::abs(hurried.value(QStringLiteral("rest")).toDouble() - rest) < 0.05;
    // The small one's count takes in the shapes and storage its objects
    // share with the largest's, since its name sorts first and the first
    // property reaching an item is the one it's counted for.
    const bool apart = little.rank > keeps.rank && little.count > HOLDS_LITTLE &&
                       little.count < most.count / 50;
    const bool numbers = !line.contains(QStringLiteral("notData"));
    const bool said = line.contains(QStringLiteral("pieces/Holding.qml holdsMost "));
    const bool again = !second.value(QStringLiteral("busy")).toBool() &&
                       mostAgain.count == most.count;
    QMetaObject::invokeMethod(made.get(), "letGo");
    const double after = weighed(engine, heap).live;
    const double freed = before - after;
    const bool collected = std::abs(most.mb - freed) <= freed / 5;
    const bool held = ranked && inner && byFile && hidden && apart && numbers && said && whole &&
                      unnamed && cleared && again && collected;
    if (!held)
        std::fprintf(stderr,
                     "FAIL holders() names what holds the most first, one inside another "
                     "object by its file and not its holder, each level by its file, the "
                     "functions, a file named from data as ~, a small one apart, no number, "
                     "all that's live once, what it had no time for as unnamed, no mark left, "
                     "the same twice, and about what's freed for the largest: most %.0f (rank "
                     "%d, %.2f MB), keeps %.0f (rank %d), own %.0f, functions %.0f, huge %.2f "
                     "MB, ~ %.0f, little %.0f (rank %d), most again %.0f, total %.2f of %.2f "
                     "live in %.2f of parts, unnamed %s, cleared %s, freed %.2f MB: %s; "
                     "hurried: %s\n",
                     most.count, most.rank, most.mb, keeps.count, keeps.rank, own.count,
                     functions.count, huge.mb, fromData.count, little.count, little.rank,
                     mostAgain.count, total, before, parts, unnamed ? "yes" : "no",
                     cleared ? "yes" : "no", freed, qPrintable(line),
                     qPrintable(hurried.value(QStringLiteral("said")).toString()));
    return held;
}

// How many objects the freeing test keeps under one property alone, and
// how many in the one array two properties hold.
constexpr int ALONE = 40000;
constexpr int SHARED = 40000;
// How many objects one property's array holds that a closure connected to
// a signal holds too, and how many objects made in script, with no parent,
// one property alone holds, and how many each of those holds of its own.
constexpr int CLOSED = 40000;
constexpr int SCRIPTED = 2000;
constexpr int EACH = 20;

// Whether freedBy() counts for a property what only it reaches: about what
// letting it go frees, the next collection taking it, objects made in
// script with no parent included, since nothing else keeps them; next to
// nothing for either of two properties holding the same array, though
// holders() counts it all for the one whose name sorts first, nor for one
// whose array a connected closure keeps too, as the collector does through
// its persistent values, nor for one holding an object made in script
// whose parent's tree keeps it, as the collector's weak values do, though
// its own parent is made in script; all of a parentless object made in
// script and its child, which nothing else keeps; none for a name no
// property has, said as held by none without saying the name; and the
// same asked twice, so it cleared the marks it set.
bool freesWhatItAloneHolds() {
    QTemporaryDir shell;
    QDir files(shell.path());
    const bool wrote =
        files.mkdir(QStringLiteral("pieces")) && files.cd(QStringLiteral("pieces")) &&
        written(files, QStringLiteral("Sharing.qml"),
                QStringLiteral("import QtQml\nQtObject {\n    id: root\n"
                               "    property var alone: []\n    property var adopted: null\n"
                               "    property var floating: null\n"
                               "    property var aShares: []\n    property var bShares: null\n"
                               "    property var closed: null\n    property var made: []\n"
                               "    property Component piece: Component {\n"
                               "        QtObject { property var own: [] }\n    }\n"
                               "    signal poked()\n"
                               "    function letAloneGo() { alone = []; }\n"
                               "    function letMadeGo() { made = []; }\n"
                               "    Component.onCompleted: {\n"
                               "        for (let i = 0; i < %1; i++)\n"
                               "            alone.push({ n: i, s: 'a' + i });\n"
                               "        for (let i = 0; i < %2; i++)\n"
                               "            aShares.push({ n: i, s: 'b' + i });\n"
                               "        bShares = aShares;\n"
                               "        const kept = [];\n"
                               "        for (let i = 0; i < %3; i++)\n"
                               "            kept.push({ n: i, s: 'c' + i });\n"
                               "        closed = kept;\n"
                               "        poked.connect(() => kept.length);\n"
                               "        const mid = piece.createObject(root);\n"
                               "        const kid = piece.createObject(mid);\n"
                               "        for (let i = 0; i < %3; i++)\n"
                               "            kid.own.push({ n: i, s: 'e' + i });\n"
                               "        adopted = kid;\n"
                               "        const loose = piece.createObject(null);\n"
                               "        const child = piece.createObject(loose);\n"
                               "        for (let i = 0; i < %3; i++)\n"
                               "            child.own.push({ n: i, s: 'f' + i });\n"
                               "        floating = loose;\n"
                               "        for (let i = 0; i < %4; i++) {\n"
                               "            const one = piece.createObject(null);\n"
                               "            for (let j = 0; j < %5; j++)\n"
                               "                one.own.push({ n: j, s: 'd' + j });\n"
                               "            made.push(one);\n"
                               "        }\n"
                               "    }\n}\n")
                    .arg(ALONE)
                    .arg(SHARED)
                    .arg(CLOSED)
                    .arg(SCRIPTED)
                    .arg(EACH));
    if (!wrote) {
        std::fprintf(stderr, "FAIL the freeing test's file wasn't written in %s\n",
                     qPrintable(shell.path()));
        return false;
    }
    QQmlEngine engine;
    Heap heap(&engine);
    QQmlComponent component(&engine,
                            QUrl::fromLocalFile(files.filePath(QStringLiteral("Sharing.qml"))));
    // A parent with no QML data, as a tree's root made in C++ has. Qt marks
    // the children of a marked wrapper only when its object has no parent,
    // so with this one only the walk to the tree's root keeps the adopted
    // kid. Made first, it's let go last, after the object has left it.
    QObject top;
    const std::unique_ptr<QObject> made(component.create());
    if (!made) {
        std::fprintf(stderr, "FAIL the freeing test's object wasn't made: %s\n",
                     qPrintable(component.errorString()));
        return false;
    }
    made->setParent(&top);
    const double before = weighed(engine, heap).live;
    const QVariantMap alone = heap.freedBy(QStringLiteral("pieces/Sharing.qml alone"));
    const QVariantMap again = heap.freedBy(QStringLiteral("pieces/Sharing.qml alone"));
    const QVariantMap first = heap.freedBy(QStringLiteral("pieces/Sharing.qml aShares"));
    const QVariantMap second = heap.freedBy(QStringLiteral("pieces/Sharing.qml bShares"));
    const QVariantMap none = heap.freedBy(QStringLiteral("pieces/Sharing.qml nothing"));
    const QVariantMap closed = heap.freedBy(QStringLiteral("pieces/Sharing.qml closed"));
    const QVariantMap madeAlone = heap.freedBy(QStringLiteral("pieces/Sharing.qml made"));
    const QVariantMap adopted = heap.freedBy(QStringLiteral("pieces/Sharing.qml adopted"));
    const QVariantMap floating = heap.freedBy(QStringLiteral("pieces/Sharing.qml floating"));
    const Holding sorted =
        holding(heap.holders(EVERY, UNHURRIED_MS), QStringLiteral("pieces/Sharing.qml aShares"));
    const bool cleared = !anyMarked(engine.handle()->memoryManager);
    const QString line = alone.value(QStringLiteral("said")).toString();
    QMetaObject::invokeMethod(made.get(), "letAloneGo");
    const double afterAlone = weighed(engine, heap).live;
    const double freed = before - afterAlone;
    QMetaObject::invokeMethod(made.get(), "letMadeGo");
    const double madeFreed = afterAlone - weighed(engine, heap).live;
    const double madeMB = madeAlone.value(QStringLiteral("freed")).toDouble();
    const double madeCount = madeAlone.value(QStringLiteral("count")).toDouble();
    const double closedMB = closed.value(QStringLiteral("freed")).toDouble();
    const bool scripted = madeAlone.value(QStringLiteral("held")).toBool() &&
                          madeCount >= SCRIPTED * EACH &&
                          std::abs(madeMB - madeFreed) <= madeFreed / 5;
    const double aloneMB = alone.value(QStringLiteral("freed")).toDouble();
    const double adoptedMB = adopted.value(QStringLiteral("freed")).toDouble();
    const bool persisting = closed.value(QStringLiteral("held")).toBool() &&
                            closedMB < aloneMB / 50 &&
                            adopted.value(QStringLiteral("held")).toBool() &&
                            adoptedMB < aloneMB / 50;
    // Its child holds as many items as `alone` holds, alike in shape.
    const double floatingMB = floating.value(QStringLiteral("freed")).toDouble();
    const bool loosed = floating.value(QStringLiteral("held")).toBool() &&
                        floatingMB > aloneMB * 0.8 && floatingMB < aloneMB * 1.2;
    const double aloneCount = alone.value(QStringLiteral("count")).toDouble();
    const double firstMB = first.value(QStringLiteral("freed")).toDouble();
    const double secondMB = second.value(QStringLiteral("freed")).toDouble();
    const bool frees = alone.value(QStringLiteral("held")).toBool() &&
                       aloneCount >= 2 * ALONE && std::abs(aloneMB - freed) <= freed / 5;
    const bool shared = first.value(QStringLiteral("held")).toBool() &&
                        second.value(QStringLiteral("held")).toBool() &&
                        sorted.count >= 2 * SHARED && firstMB < sorted.mb / 50 &&
                        secondMB < sorted.mb / 50;
    const bool unheld = !none.value(QStringLiteral("held")).toBool() &&
                        none.value(QStringLiteral("freed")).toDouble() == 0 &&
                        none.contains(QStringLiteral("said")) &&
                        !none.contains(QStringLiteral("holder")) &&
                        !none.value(QStringLiteral("said")).toString().contains(
                            QStringLiteral("Sharing"));
    const bool same = !again.value(QStringLiteral("busy")).toBool() &&
                      again.value(QStringLiteral("count")).toDouble() == aloneCount;
    const bool said = line.contains(QStringLiteral("pieces/Sharing.qml alone ")) &&
                      alone.value(QStringLiteral("ms")).toDouble() > 0;
    const bool held =
        frees && shared && unheld && same && said && cleared && scripted && persisting && loosed;
    if (!held)
        std::fprintf(stderr,
                     "FAIL freedBy() counts what a property alone reaches, about what letting "
                     "it go frees, objects made in script included, next to nothing for two "
                     "sharing one array, one a connected closure shares or an object its tree "
                     "keeps, all a loose object and its child hold, none for a name no "
                     "property has, the same twice and no mark left: alone %.2f MB (%.0f) "
                     "against %.2f freed, made %.2f MB (%.0f) against %.2f freed, closed "
                     "%.2f MB, adopted %.2f MB, floating %.2f MB, again %.0f, shared %.2f and "
                     "%.2f MB against %.2f "
                     "(%.0f) sorted first, none held %s, cleared %s: %s; none: %s\n",
                     aloneMB, aloneCount, freed, madeMB, madeCount, madeFreed, closedMB, adoptedMB,
                     floatingMB,
                     again.value(QStringLiteral("count")).toDouble(), firstMB, secondMB,
                     sorted.mb, sorted.count,
                     none.value(QStringLiteral("held")).toBool() ? "yes" : "no",
                     cleared ? "yes" : "no", qPrintable(line),
                     qPrintable(none.value(QStringLiteral("said")).toString()));
    return held;
}

// How many objects each object made in script holds in the test of what
// the sweep takes anyway.
constexpr int DROPPED = 20000;

// Whether freedBy() leaves out what an object nothing reaches holds under
// the holder's name, which the next sweep takes whether or not the
// holder's let go: one made in script and dropped since the last
// collection, beside another kept by a property, counts as the kept one
// alone did, though it's still in the heap.
bool leavesTheSweepsAlone() {
    QTemporaryDir shell;
    QDir files(shell.path());
    const bool wrote =
        files.mkdir(QStringLiteral("pieces")) && files.cd(QStringLiteral("pieces")) &&
        written(files, QStringLiteral("Dropping.qml"),
                QStringLiteral("import QtQml\nQtObject {\n    id: root\n"
                               "    property var keep: null\n"
                               "    property Component piece: Component {\n"
                               "        QtObject { property var own: [] }\n    }\n"
                               "    function made(tag) {\n"
                               "        const one = piece.createObject(null);\n"
                               "        for (let i = 0; i < %1; i++)\n"
                               "            one.own.push({ n: i, s: tag + i });\n"
                               "        return one;\n    }\n"
                               "    function keepOne() { keep = made('k'); }\n"
                               "    function dropOne() { made('g'); }\n}\n")
                    .arg(DROPPED));
    if (!wrote) {
        std::fprintf(stderr, "FAIL the sweep's test file wasn't written in %s\n",
                     qPrintable(shell.path()));
        return false;
    }
    QQmlEngine engine;
    Heap heap(&engine);
    QQmlComponent component(&engine,
                            QUrl::fromLocalFile(files.filePath(QStringLiteral("Dropping.qml"))));
    const std::unique_ptr<QObject> made(component.create());
    if (!made) {
        std::fprintf(stderr, "FAIL the sweep's test object wasn't made: %s\n",
                     qPrintable(component.errorString()));
        return false;
    }
    QMetaObject::invokeMethod(made.get(), "keepOne");
    weighed(engine, heap);
    // Room enough that dropping one starts no collection to sweep it.
    heap.leaveRoom(ROOM_MB);
    const QString holder = QStringLiteral("pieces/Dropping.qml own");
    const QVariantMap kept = heap.freedBy(holder);
    const double before = heap.weigh().value(QStringLiteral("live")).toDouble();
    QMetaObject::invokeMethod(made.get(), "dropOne");
    const double grew = heap.weigh().value(QStringLiteral("live")).toDouble() - before;
    const QVariantMap after = heap.freedBy(holder);
    const double keptMB = kept.value(QStringLiteral("freed")).toDouble();
    const double keptCount = kept.value(QStringLiteral("count")).toDouble();
    const double afterCount = after.value(QStringLiteral("count")).toDouble();
    const bool unswept = grew > keptMB * 0.8;
    const bool held = kept.value(QStringLiteral("held")).toBool() && keptCount >= 2 * DROPPED &&
                      after.value(QStringLiteral("held")).toBool() &&
                      !after.value(QStringLiteral("busy")).toBool() && unswept &&
                      afterCount < keptCount * 1.1;
    if (!held)
        std::fprintf(stderr,
                     "FAIL freedBy() leaves out what an object nothing reaches holds: kept one "
                     "%.2f MB (%.0f), with one dropped and unswept %.2f MB (%.0f), the heap "
                     "grew %.2f MB with it, busy %s\n",
                     keptMB, keptCount, after.value(QStringLiteral("freed")).toDouble(),
                     afterCount, grew,
                     after.value(QStringLiteral("busy")).toBool() ? "yes" : "no");
    return held;
}

// How many objects of one file the chain test links, each reached only
// through the one before it, and how many objects each holds under each of
// two properties.
constexpr int LINKED = 12;
constexpr int LINK_LOAD = 2000;

// Whether freedBy() follows a chain of objects of the holder's own file,
// each reached only through the one before it, whatever order it finds
// their storage in: letting go of every node's `load` frees the loads
// alone, each node's `extra` kept by the chain, and letting go of every
// `next` frees every node past the first, with all each holds, each about
// what letting them go frees.
bool followsAChain() {
    QTemporaryDir shell;
    QDir files(shell.path());
    const bool wrote =
        files.mkdir(QStringLiteral("pieces")) && files.cd(QStringLiteral("pieces")) &&
        written(files, QStringLiteral("Node.qml"),
                QStringLiteral("import QtQml\nQtObject {\n    property var next: null\n"
                               "    property var load: []\n    property var extra: []\n}\n")) &&
        written(files, QStringLiteral("Chain.qml"),
                QStringLiteral("import QtQml\nQtObject {\n    property var head: null\n"
                               "    property Component node: Component { Node {} }\n"
                               "    function build() {\n"
                               "        let last = null;\n"
                               "        for (let i = 0; i < %1; i++) {\n"
                               "            const one = node.createObject(null);\n"
                               "            for (let j = 0; j < %2; j++) {\n"
                               "                one.load.push({ n: j, s: 'l' + i + '.' + j });\n"
                               "                one.extra.push({ n: j, s: 'x' + i + '.' + j });\n"
                               "            }\n"
                               "            if (last === null) head = one; else last.next = one;\n"
                               "            last = one;\n"
                               "        }\n    }\n"
                               "    function letLoadsGo() {\n"
                               "        for (let one = head; one !== null; one = one.next)\n"
                               "            one.load = [];\n    }\n"
                               "    function letRestGo() { head.next = null; }\n}\n")
                    .arg(LINKED)
                    .arg(LINK_LOAD));
    if (!wrote) {
        std::fprintf(stderr, "FAIL the chain test's files weren't written in %s\n",
                     qPrintable(shell.path()));
        return false;
    }
    QQmlEngine engine;
    Heap heap(&engine);
    QQmlComponent component(&engine,
                            QUrl::fromLocalFile(files.filePath(QStringLiteral("Chain.qml"))));
    const std::unique_ptr<QObject> made(component.create());
    if (!made) {
        std::fprintf(stderr, "FAIL the chain test's object wasn't made: %s\n",
                     qPrintable(component.errorString()));
        return false;
    }
    QMetaObject::invokeMethod(made.get(), "build");
    const double before = weighed(engine, heap).live;
    const QVariantMap loads = heap.freedBy(QStringLiteral("pieces/Node.qml load"));
    QMetaObject::invokeMethod(made.get(), "letLoadsGo");
    const double loadsFreed = before - weighed(engine, heap).live;
    // A chain made again, its first let go, so the next weighing finds it whole.
    QMetaObject::invokeMethod(made.get(), "build");
    const double rebuilt = weighed(engine, heap).live;
    const QVariantMap rest = heap.freedBy(QStringLiteral("pieces/Node.qml next"));
    QMetaObject::invokeMethod(made.get(), "letRestGo");
    const double restFreed = rebuilt - weighed(engine, heap).live;
    const double loadsMB = loads.value(QStringLiteral("freed")).toDouble();
    const double restMB = rest.value(QStringLiteral("freed")).toDouble();
    const bool loadsHeld = loads.value(QStringLiteral("held")).toBool() &&
                           loads.value(QStringLiteral("count")).toDouble() >=
                               2.0 * LINKED * LINK_LOAD &&
                           std::abs(loadsMB - loadsFreed) <= loadsFreed / 5;
    const bool restHeld = rest.value(QStringLiteral("held")).toBool() &&
                          rest.value(QStringLiteral("count")).toDouble() >=
                              4.0 * (LINKED - 1) * LINK_LOAD &&
                          std::abs(restMB - restFreed) <= restFreed / 5;
    if (!(loadsHeld && restHeld))
        std::fprintf(stderr,
                     "FAIL freedBy() follows a chain of one file's objects: every load %.2f MB "
                     "(%.0f) against %.2f freed, every next %.2f MB (%.0f) against %.2f "
                     "freed\n",
                     loadsMB, loads.value(QStringLiteral("count")).toDouble(), loadsFreed,
                     restMB, rest.value(QStringLiteral("count")).toDouble(), restFreed);
    return loadsHeld && restHeld;
}

// Whether holders() and freedBy() count nothing while a collection is under
// way between its slices, saying only that they're busy, rather than
// clearing the marks the collection has set: what the collection keeps
// still reads back once it's ended.
bool waitsOutACollection() {
    QJSEngine engine;
    Heap heap(&engine);
    const QJSValue few = keepScattered(engine);
    QV4::MemoryManager *manager = engine.handle()->memoryManager;
    const std::chrono::microseconds slice = manager->gcStateMachine->timeLimit;
    manager->gcStateMachine->timeLimit = A_MOMENT;
    manager->runGC();
    const bool running = manager->gcStateMachine->inProgress();
    const QVariantMap during = heap.holders(EVERY, UNHURRIED_MS);
    const QVariantMap freeing = heap.freedBy(QStringLiteral("kept"));
    manager->gcStateMachine->timeLimit = slice;
    engine.collectGarbage();
    constexpr int LAST = KEPT / SCATTERED - 1;
    const bool busy = during.value(QStringLiteral("busy")).toBool() &&
                      !during.value(QStringLiteral("marked")).toBool() &&
                      !during.contains(QStringLiteral("said")) &&
                      freeing.value(QStringLiteral("busy")).toBool() &&
                      !freeing.value(QStringLiteral("marked")).toBool() &&
                      !freeing.contains(QStringLiteral("said"));
    const bool kept = few.property(QStringLiteral("length")).toInt() == LAST + 1 &&
                      few.property(LAST).property(QStringLiteral("n")).toInt() ==
                          LAST * SCATTERED;
    const bool held = running && busy && kept;
    if (!held)
        std::fprintf(stderr,
                     "FAIL holders() and freedBy() say only that they're busy during a "
                     "collection and leave what it keeps: left running %s, busy %s (holders() "
                     "busy %s, freedBy() busy %s, marked %s, said %s), kept %s\n",
                     running ? "yes" : "no", busy ? "yes" : "no",
                     during.value(QStringLiteral("busy")).toBool() ? "yes" : "no",
                     freeing.value(QStringLiteral("busy")).toBool() ? "yes" : "no",
                     freeing.value(QStringLiteral("marked")).toBool() ? "yes" : "no",
                     freeing.contains(QStringLiteral("said")) ? "yes" : "no",
                     kept ? "yes" : "no");
    return held;
}

// Whether holders() and freedBy() count nothing over a mark they find set
// outside a collection, saying so, and leave that mark as it was.
bool leavesAStrayMark() {
    QJSEngine engine;
    Heap heap(&engine);
    const QJSValue kept = engine.evaluate(QStringLiteral("({ kept: true })"));
    QV4::Chunk *chunk = engine.handle()->memoryManager->blockAllocator.chunks.front();
    constexpr auto WORDS = static_cast<std::size_t>(QV4::Chunk::EntriesInBitmap);
    std::size_t word = 0;
    while (word < WORDS && chunk->objectBitmap[word] == 0)
        ++word;
    if (word == WORDS) {
        std::fprintf(stderr, "FAIL the first chunk holds an item to mark\n");
        return false;
    }
    const quintptr bit = chunk->objectBitmap[word] & (~chunk->objectBitmap[word] + 1);
    chunk->blackBitmap[word] |= bit;
    const QVariantMap stray = heap.holders(EVERY, UNHURRIED_MS);
    const QVariantMap strayFreeing = heap.freedBy(QStringLiteral("kept"));
    const bool left = (chunk->blackBitmap[word] & bit) != 0;
    chunk->blackBitmap[word] &= ~bit;
    const bool held = stray.value(QStringLiteral("busy")).toBool() &&
                      stray.value(QStringLiteral("marked")).toBool() &&
                      !stray.contains(QStringLiteral("said")) && left &&
                      strayFreeing.value(QStringLiteral("busy")).toBool() &&
                      strayFreeing.value(QStringLiteral("marked")).toBool() &&
                      !strayFreeing.contains(QStringLiteral("said")) &&
                      kept.property(QStringLiteral("kept")).toBool();
    if (!held)
        std::fprintf(stderr,
                     "FAIL holders() and freedBy() say marks were left set outside a "
                     "collection and leave them: busy %s, marked %s, counted %s, left %s; "
                     "freedBy() busy %s, marked %s, counted %s\n",
                     stray.value(QStringLiteral("busy")).toBool() ? "yes" : "no",
                     stray.value(QStringLiteral("marked")).toBool() ? "yes" : "no",
                     stray.contains(QStringLiteral("said")) ? "yes" : "no", left ? "yes" : "no",
                     strayFreeing.value(QStringLiteral("busy")).toBool() ? "yes" : "no",
                     strayFreeing.value(QStringLiteral("marked")).toBool() ? "yes" : "no",
                     strayFreeing.contains(QStringLiteral("said")) ? "yes" : "no");
    return held;
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
    const bool burstPaced = pacesABurstPastTheRoom(0);
    const bool litterPaced = pacesABurstPastTheRoom(LITTER);
    const bool capped = roomCapsWhatsKept();
    const bool follows = allocatorFollows(heap);
    const bool given = givesBack();
    const bool lent = lendsBack();
    const bool fits = takesRunsThatFit();
    const bool whole = leavesLargeRunsWhole();
    const bool fullest = fillsTheFullestFirst();
    const bool inOrder = sortsEachBinFullestFirst();
    const bool census = namesWhatsKept();
    const bool covered = residentCoversWhatsLive();
    const bool placed = placesWhatMadeIt();
    const bool stands = standsAsWeighed();
    const bool underWay = saysOneUnderWay();
    const bool holdsIt = namesWhatHoldsIt();
    const bool waitsItOut = waitsOutACollection();
    const bool stray = leavesAStrayMark();
    const bool alone = freesWhatItAloneHolds();
    const bool swept = leavesTheSweepsAlone();
    const bool chained = followsAChain();
    return grows && falls && holds && scattered && bounded && timed && collects && roomy &&
                   roomyOutside && lasting && lastingOutside && stillCollectsOutside &&
                   reckoned && ownLimits && burstPaced && litterPaced && capped && follows &&
                   given && lent && fits && whole && fullest && inOrder && census && covered &&
                   placed && stands && underWay && holdsIt && waitsItOut && stray && alone &&
                   swept && chained
               ? 0
               : 1;
}
