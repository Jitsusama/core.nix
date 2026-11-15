// Qt's memory manager keeps the heap's own count of it, which no public
// API gives, so the module reads it through Qt's private headers, built
// with the very Qt the shell runs (modules/quickshell/nixos.nix), whose
// private symbols are tied to its version. Each count walks the heap's
// chunks once, a few milliseconds at most, so it's read only when asked,
// as the launcher asks once a closing's collection is done.
//
// The collector's time is Qt's own count of each step it runs, kept only
// while its step timings are on, which they're turned on for here: two
// reads of the monotonic clock a step, about 30 steps a collection in the
// launcher's lab (2026-11-12), a few microseconds against the 25 to 90 ms
// a collection takes.
//
// Free pages are counted because Qt gives a chunk back only once nothing in
// it is live, and never moves what is, so Joel's shell held three times what
// was live after a day (2026-11-13). A page that holds no item and no free
// run's own header could go back to the kernel and come back zeroed, as Qt
// zeroes each item it makes, so their count says what letting them go would
// save. Only resident ones count, as mincore says, since a page never
// written costs nothing.
//
// Room is left because Qt starts a collection whenever its free lists miss
// while what's held is over twice what the last full sweep left in use, and
// after the closing's collection it always is: three times, in Joel's shell
// and the launcher's lab. So Emoji's openings ran one to six passes each,
// 47 to 204 ms of the collector (the lab, 2026-11-14). Qt reads only the
// count of what the last sweep left in use to decide it, and the next sweep
// sets it again, so raising it to half of what's held and the room asked
// for lets the heap grow by that room first, and lasts until a collection.
//
// Qt also starts one once the text strings hold outside the heap passes a
// limit it sets at each collection's end, and that limit was what met the
// openings once the heap had room: the lab's openings began 4 MB under it,
// at 17 to 18 MB of 22, and passed it within the opening, 106 to 214 ms of
// the collector each (the lab's cycle-u1, 2026-11-14). So the same room is
// left outside the heap too, until the next collection sets the limit
// again.
//
// What jemalloc holds is read because the shell's memory outside its heap
// and files was 165 to 200 MB on Joel's shell, against 126 in the lab,
// where jemalloc's own counts split it: 80 MB in use, 7 MB of its own
// bookkeeping, and 12 to 55 MB of freed pages it kept for up to 10 s after
// each closing's collection (jm1, 2026-11-15). Only the process can read
// them, through jemalloc's mallctl, found by name so the module needs no
// link to it and says nothing in a process that doesn't allocate through
// it.
//
// heap_test.cpp holds it to what it says.

#include "heap.h"

#include <QJSEngine>
#include <algorithm>
#include <cstdint>
#include <dlfcn.h>
#include <private/qv4engine_p.h>
#include <private/qv4mm_p.h>
#include <sys/mman.h>
#include <unistd.h>
#include <vector>

namespace {

constexpr double BYTES_PER_MB = 1024.0 * 1024.0;

// What's held may be this many times what the last sweep left in use before
// a miss starts the collector: Qt's GCOverallocation, 200%, private to
// qv4mm.cpp.
constexpr std::size_t OVERALLOCATION = 2;

double megabytes(std::size_t bytes) { return static_cast<double>(bytes) / BYTES_PER_MB; }

// The bytes in an allocator's chunks that are resident and lie in pages
// holding no item and no free run's header. A chunk's first page holds its
// bitmaps, and a run's header is its first slot, so a page counts when
// every slot in it, and the one just before it, is free.
std::size_t freePages(const QV4::BlockAllocator &allocator) {
    using QV4::Chunk;
    const auto page = static_cast<std::size_t>(sysconf(_SC_PAGESIZE));
    const std::size_t words = page / Chunk::SlotSize / Chunk::Bits;
    if (words == 0 || Chunk::ChunkSize % page != 0)
        return 0;
    const std::size_t pages = Chunk::ChunkSize / page;
    std::vector<unsigned char> resident(pages);
    std::size_t bytes = 0;
    for (Chunk *chunk : allocator.chunks) {
        if (mincore(chunk, Chunk::ChunkSize, resident.data()) != 0)
            continue;
        const auto used = [chunk](std::size_t word) {
            return chunk->objectBitmap[word] | chunk->extendsBitmap[word];
        };
        for (std::size_t at = 1; at < pages; ++at) {
            const std::size_t first = at * words;
            bool empty = (used(first - 1) >> (Chunk::Bits - 1)) == 0;
            for (std::size_t word = first; empty && word < first + words; ++word)
                empty = used(word) == 0;
            if (empty && (resident[at] & 1) != 0)
                bytes += page;
        }
    }
    return bytes;
}

} // namespace

Heap::Heap(QJSEngine *engine, QObject *parent) : QObject(parent), engine(engine) {
    engine->handle()->memoryManager->gcStateMachine->collectTimings = true;
}

Heap *Heap::create(QQmlEngine * /*qmlEngine*/, QJSEngine *engine) { return new Heap(engine); }

QVariantMap Heap::weigh() const {
    const QV4::MemoryManager *manager = engine->handle()->memoryManager;
    // Large items have chunks of their own, as big as each, so they're
    // live and held alike.
    const std::size_t large = manager->getLargeItemsMem();
    return {
        {QStringLiteral("live"), megabytes(manager->getRegularItemsMem() + large)},
        {QStringLiteral("held"), megabytes(manager->getAllocatedMem())},
        {QStringLiteral("outside"), megabytes(manager->unmanagedHeapSize)},
        {QStringLiteral("free"),
         megabytes(freePages(manager->blockAllocator) + freePages(manager->icAllocator))},
    };
}

void Heap::leaveRoom(double megabytes) {
    QV4::MemoryManager *manager = engine->handle()->memoryManager;
    const std::size_t held =
        manager->blockAllocator.totalSlots() + manager->icAllocator.totalSlots();
    const auto bytes = static_cast<std::size_t>(std::max(megabytes, 0.0) * BYTES_PER_MB);
    const auto room = bytes / QV4::Chunk::SlotSize;
    manager->usedSlotsAfterLastFullSweep =
        std::max(manager->usedSlotsAfterLastFullSweep, (held + room) / OVERALLOCATION);
    manager->unmanagedHeapSizeGCLimit =
        std::max(manager->unmanagedHeapSizeGCLimit, manager->unmanagedHeapSize + bytes);
}

QVariantMap Heap::allocator() const {
    using Mallctl = int (*)(const char *, void *, std::size_t *, void *, std::size_t);
    // NOLINTNEXTLINE(cppcoreguidelines-pro-type-reinterpret-cast)
    auto *mallctl = reinterpret_cast<Mallctl>(dlsym(RTLD_DEFAULT, "mallctl"));
    if (mallctl == nullptr)
        return {};
    // jemalloc's counts are a snapshot it takes when its epoch moves on.
    std::uint64_t epoch = 1;
    std::size_t epochSize = sizeof(epoch);
    if (mallctl("epoch", &epoch, &epochSize, &epoch, epochSize) != 0)
        return {};
    const auto read = [mallctl](const char *name) {
        std::size_t value = 0;
        std::size_t size = sizeof(value);
        return mallctl(name, &value, &size, nullptr, 0) == 0 ? value : 0;
    };
    // Freed pages are counted in pages, every arena's at once (4096 is
    // MALLCTL_ARENAS_ALL).
    const auto page = static_cast<std::size_t>(sysconf(_SC_PAGESIZE));
    return {{QStringLiteral("used"), megabytes(read("stats.allocated"))},
            {QStringLiteral("resident"), megabytes(read("stats.resident"))},
            {QStringLiteral("kept"), megabytes(read("stats.arenas.4096.pdirty") * page)}};
}

double Heap::collected() const {
    constexpr double MICROSECONDS_PER_MS = 1000.0;
    qint64 microseconds = 0;
    for (const auto &step : engine->handle()->memoryManager->gcStateMachine->executionTiming)
        microseconds += step.rolling_sum;
    return static_cast<double>(microseconds) / MICROSECONDS_PER_MS;
}
