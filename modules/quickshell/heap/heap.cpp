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
// heap_test.cpp holds it to what it says.

#include "heap.h"

#include <QJSEngine>
#include <private/qv4engine_p.h>
#include <private/qv4mm_p.h>
#include <sys/mman.h>
#include <unistd.h>
#include <vector>

namespace {

constexpr double BYTES_PER_MB = 1024.0 * 1024.0;

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

double Heap::collected() const {
    constexpr double MICROSECONDS_PER_MS = 1000.0;
    qint64 microseconds = 0;
    for (const auto &step : engine->handle()->memoryManager->gcStateMachine->executionTiming)
        microseconds += step.rolling_sum;
    return static_cast<double>(microseconds) / MICROSECONDS_PER_MS;
}
