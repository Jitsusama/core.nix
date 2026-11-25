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
// run's own header can go back to the kernel and come back zeroed, as Qt
// zeroes each item it makes, so their count says what letting them go
// saves. Only resident ones count, as mincore says, since a page never
// written costs nothing.
//
// giveBack() gives them back, as the shell's Qt gives back a chunk, since
// they grew with use while what's live didn't: one shell's closings left
// 69 to 72 MB live all afternoon while what it held grew from 158 to 207 MB
// and its free pages from 14 to 41 MB (2026-11-15). Qt reads nothing in
// them until it makes an item there, so only a collection under way, which
// may still be sweeping them, rules it out.
//
// lendBack() lends them instead, with MADV_FREE, and the launcher asks it
// after each closing's collection. Giving them back made the next opening
// fault each page in again, 145.5 against 105 ms for the clipboard's
// first (2026-11-16), so the shell never asked; yet a shell four hours on
// kept 131 to 136 MB of them (2026-11-23). Lent, the kernel takes them only
// when memory runs short, and an opening writing one it hasn't taken
// keeps its frame: no fault where the CPU marks pages dirty itself, as
// x86 does, and a fault that zeroes nothing where the kernel tracks it, as
// on an Arm without hardware dirty bits. One it took comes back zeroed,
// which Qt's zeroing of each item it makes already allows for. A page lent
// and not yet taken still counts as resident, to mincore and to the
// shell's weight, so lent() says what the kernel holds lent.
//
// What's held is said resident too, since once free pages are lent what's
// held no longer says what the shell pays for: a page the kernel took
// stays held, and stops being counted free, so a shell 1.4 hours on held
// 252 MB with 65 live and 85 in free pages, and the other 102 could be
// either taken pages, which cost nothing, or the free slots of pages
// still holding an item, which do (2026-11-25). mincore says which of
// every chunk's pages and every large item's are in memory.
//
// Room is left because Qt starts a collection whenever its free lists miss
// while what's held is over twice what the last full sweep left in use, and
// after the closing's collection it always is: three times, in Joel's shell
// and the launcher's lab. So Emoji's openings ran one to six passes each,
// 47 to 204 ms of the collector (the lab, 2026-11-14). Qt reads only the
// count of what the last sweep left in use to decide it, so raising it to
// half of what's held and the room asked for lets the heap grow by that room
// first.
//
// Qt also starts one once the text strings hold outside the heap passes a
// limit it sets at each collection's end, and that limit was what met the
// openings once the heap had room: the lab's openings began 4 MB under it,
// at 17 to 18 MB of 22, and passed it within the opening, 106 to 214 ms of
// the collector each (the lab's cycle-u1, 2026-11-14). So the same room is
// left outside the heap too.
//
// Each collection's end sets both again, so the shell's Qt keeps the room
// and leaves it after every one (qtdeclarative-room-after-every-sweep.patch),
// and leaveRoom only asks it to there: the heap holds over twice what's live
// straight after any sweep, not only the closing's, so a room that lasted
// until the next collection let one opening of Joel's shell run two whole
// collections 16 s apart, each inside a key, 115 ms of its 175 and 193
// (2026-11-19). A miss never starts the collector later than twice the room
// past what the chunks held at the first ask since a collection ended, nor
// sooner than Qt itself would, so a heap whose sweeps keep a few items in
// each chunk it grew into doesn't grow by the room at every one; an ask
// with no collection since the last, the opening's after the closing's or
// one for another room, shares its ceiling; and it reckons from what each
// sweep left, not from where leaveRoom set it, so asking twice leaves the
// room once. weigh() says where the next starts. Every ask after a
// collection moves the ceiling, so such a heap may climb by twice the room
// at each: four rooms between closings when one ran while the launcher was
// shut, six with the census, where main let it climb by the room at each
// ask. The closing's line says what it holds.
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
// What's live is counted by class and by shape, each shape named by its
// property names, because Joel's shell keeps 62 to 73 MB live against the
// lab's 18, and what's live sets both what a collection costs and what the
// heap holds, so the largest kinds say what to cut (2026-11-17). It walks
// every item once, about 17 ms at the lab's 18 MB, and names only the
// largest shapes.
//
// Closures and calls' contexts are counted by place too, the file, line and
// name of the function each is or ran in, because the count before a
// collection showed Joel's shell making 53 MB of strings, arrays and
// closures in the 5 s after a closing (2026-11-17, 15:50) while the lab's
// made almost none, and a class or a shape can't say which code made them.
// It names only the largest places, from the shell's own code.
//
// What's live is counted by the QML property holding it too, because no
// change to Qt's collector cut what the heap holds: in the heap's lab, five
// left it at 192 to 208 MB for 66 live, against 207, while keeping heard
// lines as text rather than parsed cut it to 181 and live to 51
// (2026-11-21). So what to cut is what the shell keeps, and Joel's shell
// keeps 17 MB of strings and 9 MB of shapes whose class, shape and place
// can't say who holds them. It marks from each property with the
// collector's marks, reads what each added, and clears them.
//
// Long because each count reads the one engine's chunks through the same
// walks and private headers, in the one module built against the shell's
// own Qt: split, each part would repeat those walks and that build.
//
// heap_test.cpp holds it to what it says.

#include "heap.h"

#include <QElapsedTimer>
#include <QFile>
#include <QtLogging>
#include <QHash>
#include <QJSEngine>
#include <QMetaProperty>
#include <QRegularExpression>
#include <QSet>
#include <QStringList>
#include <QUrl>
#include <QVariantList>
#include <algorithm>
#include <bit>
#include <cerrno>
#include <cmath>
#include <cstdint>
#include <cstring>
#include <dlfcn.h>
#include <optional>
#include <utility>
#include <private/qqmldata_p.h>
#include <private/qqmlvmemetaobject_p.h>
#include <private/qv4context_p.h>
#include <private/qv4engine_p.h>
#include <private/qv4executablecompilationunit_p.h>
#include <private/qv4function_p.h>
#include <private/qv4functionobject_p.h>
#include <private/qv4internalclass_p.h>
#include <private/qv4memberdata_p.h>
#include <private/qv4mm_p.h>
#include <private/qv4qmlcontext_p.h>
#include <private/qv4qobjectwrapper_p.h>
#include <sys/mman.h>
#include <unistd.h>
#include <vector>

namespace {

constexpr double BYTES_PER_MB = 1024.0 * 1024.0;
// Bytes in a kilobyte, as /proc says sizes in kilobytes.
constexpr std::size_t BYTES_PER_KB = 1024;

// The most room a heap is left, in megabytes, since it's left after every
// collection for the rest of the run: far past the shell's 32, and far
// short of where the slots it counts as could overflow.
constexpr double MOST_ROOM_MB = 1024;

double megabytes(std::size_t bytes) { return static_cast<double>(bytes) / BYTES_PER_MB; }

// Each span of an allocator's pages that hold no item and no free run's
// header, given to `each` as its start, its length and how many of its
// bytes are resident, as mincore says. A chunk's first page holds its
// bitmaps, and a run's header is its first slot, so a page counts when
// every slot in it, and the one just before it, is free.
template <typename Each> void eachFreeSpan(const QV4::BlockAllocator &allocator, Each each) {
    using QV4::Chunk;
    const auto page = static_cast<std::size_t>(sysconf(_SC_PAGESIZE));
    const std::size_t words = page / Chunk::SlotSize / Chunk::Bits;
    if (words == 0 || Chunk::ChunkSize % page != 0)
        return;
    const std::size_t pages = Chunk::ChunkSize / page;
    std::vector<unsigned char> resident(pages);
    for (Chunk *chunk : allocator.chunks) {
        if (mincore(chunk, Chunk::ChunkSize, resident.data()) != 0)
            continue;
        const auto used = [chunk](std::size_t word) {
            return chunk->objectBitmap[word] | chunk->extendsBitmap[word];
        };
        auto *const start = static_cast<char *>(static_cast<void *>(chunk));
        // The first page is never free, so a span beginning at 0 is none.
        std::size_t from = 0;
        std::size_t residentBytes = 0;
        for (std::size_t at = 1; at <= pages; ++at) {
            bool empty = at < pages;
            const std::size_t first = at * words;
            if (empty)
                empty = (used(first - 1) >> (Chunk::Bits - 1)) == 0;
            for (std::size_t word = first; empty && word < first + words; ++word)
                empty = used(word) == 0;
            if (empty) {
                from = from == 0 ? at : from;
                residentBytes += (resident[at] & 1) != 0 ? page : 0;
            } else if (from != 0) {
                each(start + from * page, (at - from) * page, residentBytes);
                from = 0;
                residentBytes = 0;
            }
        }
    }
}

// The resident bytes in an allocator's free pages.
std::size_t freePages(const QV4::BlockAllocator &allocator) {
    std::size_t bytes = 0;
    eachFreeSpan(allocator, [&bytes](char *, std::size_t, std::size_t resident) {
        bytes += resident;
    });
    return bytes;
}

// The resident bytes in `length` bytes from `start`, as mincore says, or
// nothing when it won't say, which is warned of once a process, so a
// refusal never reads as pages the kernel took.
std::optional<std::size_t> residentAt(void *start, std::size_t length) {
    static bool warned = false;
    const auto page = static_cast<std::size_t>(sysconf(_SC_PAGESIZE));
    std::vector<unsigned char> resident((length + page - 1) / page);
    if (resident.empty())
        return 0;
    if (mincore(start, length, resident.data()) != 0) {
        if (!warned) {
            warned = true;
            qWarning("heap: the kernel wouldn't say which of the heap's pages are resident: %s",
                     std::strerror(errno));
        }
        return std::nullopt;
    }
    const auto pages = std::count_if(resident.begin(), resident.end(),
                                     [](unsigned char each) { return (each & 1) != 0; });
    return page * static_cast<std::size_t>(pages);
}

// The bytes a large item's chunk spans. One with a segment of its own spans
// its `size`, already its header and item to a page; the chunk allocator
// makes the others its header and item to a page, and a whole chunk at the
// least, but keeps only the item's size (Qt's requiredChunkSize, qv4mm.cpp).
std::size_t spanOf(const QV4::HugeItemAllocator::HugeChunk &huge) {
    if (huge.segment != nullptr)
        return huge.size;
    const auto page = static_cast<std::size_t>(sysconf(_SC_PAGESIZE));
    const std::size_t span = (huge.size + QV4::Chunk::HeaderSize + page - 1) & ~(page - 1);
    return std::max<std::size_t>(span, QV4::Chunk::ChunkSize);
}

// The resident bytes in every chunk and large item the heap holds, or
// nothing when the kernel won't say for any of them.
std::optional<std::size_t> residentHeld(const QV4::MemoryManager &manager) {
    std::size_t bytes = 0;
    for (const QV4::BlockAllocator *allocator : {&manager.blockAllocator, &manager.icAllocator})
        for (QV4::Chunk *chunk : allocator->chunks) {
            const std::optional<std::size_t> resident = residentAt(chunk, QV4::Chunk::ChunkSize);
            if (!resident)
                return std::nullopt;
            bytes += *resident;
        }
    for (const auto &huge : manager.hugeItemAllocator.chunks) {
        const std::optional<std::size_t> resident = residentAt(huge.chunk, spanOf(huge));
        if (!resident)
            return std::nullopt;
        bytes += *resident;
    }
    return bytes;
}

// Gives an allocator's free pages back to the kernel with `advice`, and
// says how many of their bytes were resident: MADV_DONTNEED lets them go
// now, as the shell's Qt gives back a chunk it decommits, and MADV_FREE
// lends them, for the kernel to take only when memory runs short. The
// shell's Qt reserves the heap as anonymous memory, which both act on; a Qt
// that maps it from a memfd keeps the pages there, and refuses MADV_FREE.
// Spans with none resident are left alone, so the call costs nothing for
// them; a page lent and not yet taken is still resident, so it's lent again.
// The first span the kernel refuses is warned of, once a process for each
// advice, so a refused lending isn't hidden behind a refused giving, since
// a refusal would otherwise read as a heap with nothing free.
std::size_t givenBack(const QV4::BlockAllocator &allocator, int advice) {
    static bool warnedGiving = false;
    static bool warnedLending = false;
    bool &warned = advice == MADV_FREE ? warnedLending : warnedGiving;
    std::size_t bytes = 0;
    eachFreeSpan(allocator,
                 [&bytes, &warned, advice](char *at, std::size_t length, std::size_t resident) {
                     if (resident == 0)
                         return;
                     if (madvise(at, length, advice) == 0) {
                         bytes += resident;
                     } else if (!warned) {
                         warned = true;
                         qWarning("heap: the kernel refused the free pages %s: %s",
                                  advice == MADV_FREE ? "lent" : "given back",
                                  std::strerror(errno));
                     }
                 });
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
    // Not `slots`, which is Qt's keyword macro.
    const std::size_t chunkSlots =
        manager->blockAllocator.totalSlots() + manager->icAllocator.totalSlots();
    const std::optional<std::size_t> resident = residentHeld(*manager);
    return {
        {QStringLiteral("live"), megabytes(manager->getRegularItemsMem() + large)},
        {QStringLiteral("held"), megabytes(manager->getAllocatedMem())},
        {QStringLiteral("resident"), resident ? megabytes(*resident) : -1.0},
        {QStringLiteral("chunks"), megabytes(chunkSlots * QV4::Chunk::SlotSize)},
        {QStringLiteral("collectsAt"), megabytes(manager->heldBeforeCollecting())},
        {QStringLiteral("collectsOutsideAt"), megabytes(manager->unmanagedHeapSizeGCLimit)},
        {QStringLiteral("outside"), megabytes(manager->unmanagedHeapSize)},
        // A chunk the kernel won't say the pages of has none free, so free is
        // unknown whenever resident is, rather than nothing.
        {QStringLiteral("free"),
         resident ? megabytes(freePages(manager->blockAllocator) +
                              freePages(manager->icAllocator))
                  : -1.0},
    };
}

QVariantMap Heap::standing() const {
    const QV4::MemoryManager *manager = engine->handle()->memoryManager;
    const std::size_t chunkSlots =
        manager->blockAllocator.totalSlots() + manager->icAllocator.totalSlots();
    return {
        {QStringLiteral("chunks"), megabytes(chunkSlots * QV4::Chunk::SlotSize)},
        {QStringLiteral("collectsAt"), megabytes(manager->heldBeforeCollecting())},
        {QStringLiteral("outside"), megabytes(manager->unmanagedHeapSize)},
        {QStringLiteral("collectsOutsideAt"), megabytes(manager->unmanagedHeapSizeGCLimit)},
        {QStringLiteral("collecting"), manager->gcStateMachine->inProgress()},
    };
}

void Heap::leaveRoom(double megabytes) {
    // A room that isn't a number is none, and any other is held to the most.
    const double asked = std::isnan(megabytes) ? 0.0 : std::clamp(megabytes, 0.0, MOST_ROOM_MB);
    engine->handle()->memoryManager->leaveRoom(static_cast<std::size_t>(asked * BYTES_PER_MB));
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

double Heap::giveBack() {
    QV4::MemoryManager *manager = engine->handle()->memoryManager;
    if (manager->gcStateMachine->inProgress())
        return 0;
    return megabytes(givenBack(manager->blockAllocator, MADV_DONTNEED) +
                     givenBack(manager->icAllocator, MADV_DONTNEED));
}

double Heap::lendBack() {
    QV4::MemoryManager *manager = engine->handle()->memoryManager;
    if (manager->gcStateMachine->inProgress())
        return 0;
    return megabytes(givenBack(manager->blockAllocator, MADV_FREE) +
                     givenBack(manager->icAllocator, MADV_FREE));
}

double Heap::lent() const {
    QFile rollup(QStringLiteral("/proc/self/smaps_rollup"));
    if (!rollup.open(QIODevice::ReadOnly))
        return -1;
    static const QRegularExpression lazy(QStringLiteral("^LazyFree:\\s+(\\d+) kB$"),
                                         QRegularExpression::MultilineOption);
    const QRegularExpressionMatch match = lazy.match(QString::fromLatin1(rollup.readAll()));
    return match.hasMatch() ? megabytes(match.captured(1).toULongLong() * BYTES_PER_KB) : -1;
}

namespace {

// How many bytes and items of one class or shape are live.
// What an item of a class says of the code it came from: nothing, its own
// function as a closure, or the function it ran in as a context; `Unknown`
// until its shape is first seen.
enum class Made : std::uint8_t { Unknown, Nothing, Closure, Context };

// How many bytes and items of one class, shape or place are live, and for a
// shape, what its items say of their code, worked out once for each shape
// so no other item pays for it.
struct Tally {
    double bytes = 0;
    double count = 0;
    Made made = Made::Unknown;
};

// The `top` largest of the tallies, largest first.
template <typename Key> std::vector<std::pair<Key, Tally>> largest(const QHash<Key, Tally> &tallies,
                                                                   int top) {
    std::vector<std::pair<Key, Tally>> entries;
    for (auto it = tallies.cbegin(); it != tallies.cend(); ++it)
        entries.emplace_back(it.key(), it.value());
    std::sort(entries.begin(), entries.end(),
              [](const auto &a, const auto &b) { return a.second.bytes > b.second.bytes; });
    entries.resize(std::min(entries.size(), static_cast<std::size_t>(std::max(top, 0))));
    return entries;
}

// Calls `each` with every item an allocator holds, and its bytes. An item
// starts at a slot whose object bit is set and runs on through the slots
// whose extends bits are; each chunk's header comes first.
template <typename Each> void walk(const QV4::BlockAllocator &allocator, Each each) {
    using QV4::Chunk;
    const auto set = [](const quintptr *bitmap, std::size_t index) {
        return ((bitmap[index / Chunk::Bits] >> (index % Chunk::Bits)) & 1U) != 0;
    };
    for (Chunk *chunk : allocator.chunks) {
        QV4::HeapItem *base = chunk->realBase();
        for (std::size_t index = Chunk::HeaderSize / Chunk::SlotSize; index < Chunk::NumSlots;
             ++index) {
            if (!set(chunk->objectBitmap, index))
                continue;
            std::size_t span = 1;
            while (index + span < Chunk::NumSlots && set(chunk->extendsBitmap, index + span))
                ++span;
            each(reinterpret_cast<QV4::Heap::Base *>(base + index),
                 static_cast<double>(span * Chunk::SlotSize));
            index += span - 1;
        }
    }
}

// The longest property name said, so a key made of what Joel keeps can't
// carry it into the journal.
constexpr qsizetype NAME_LENGTH = 32;

// A property name as the journal says it: a plain identifier as it is, and
// anything else, a key taken from data, a symbol or a long name, as `~`, so
// no clipboard text, title or path reaches the line, nor breaks its shape
// for what reads it.
QString saidName(const QString &text) {
    static const QRegularExpression plain(QStringLiteral("^[A-Za-z_$][A-Za-z0-9_$]*$"));
    return text.size() <= NAME_LENGTH && plain.match(text).hasMatch() ? text
                                                                       : QStringLiteral("~");
}

QString said(const QV4::Value &name) {
    if (name.isUndefined())
        return QStringLiteral("-");
    return saidName(name.isString() ? name.toQStringNoThrow() : QString());
}

// The code a function was written in, as `Launcher.qml:943 spotsFrom`: its
// file's name, its line and its own name where it has a plain one. Only the
// shell's code is said, never what it holds, so a file named otherwise, as
// an eval's ("eval code") or a text evaluated by name would be, is `~`, and
// its functions' names, written in that text, aren't said at all.
QString placeOf(const QV4::Function *function) {
    static const QRegularExpression plain(QStringLiteral("^[A-Za-z0-9_.-]{1,48}$"));
    const QString file = function->sourceFile().section(QLatin1Char('/'), -1);
    const bool code = plain.match(file).hasMatch();
    QString place = QStringLiteral("%1:%2")
                        .arg(code ? file : QStringLiteral("~"))
                        .arg(function->compiledFunction->location.line());
    if (!code)
        return place;
    const QV4::Heap::String *name = function->name();
    const QString named =
        name == nullptr ? QString() : said(QV4::Value::fromHeapObject(
                                          const_cast<QV4::Heap::String *>(name)));
    if (!named.isEmpty() && named != QStringLiteral("~") && named != QStringLiteral("-"))
        place += QLatin1Char(' ') + named;
    return place;
}

// What a class's items say of their code, by its place among Qt's classes:
// every closure, arrows, methods, generators and bound functions among them,
// is a JavaScriptFunctionObject, and a call's context, or a block's or a
// catch's within one, is a CallContext. Read from the classes themselves
// rather than their names, so a class Qt adds is counted as what it is.
Made madeAs(const QV4::VTable *vtable) {
    if (vtable == QV4::CallContext::staticVTable())
        return Made::Context;
    for (const QV4::VTable *each = vtable; each != nullptr; each = each->parent) {
        if (each == QV4::JavaScriptFunctionObject::staticVTable())
            return Made::Closure;
    }
    return Made::Nothing;
}

// The function an item is, or ran in, as its class says (madeAs), or null.
// A context of a QML binding, a signal handler, an eval or a module holds
// none, so what those make isn't said by place; a bound function of a
// function from C++ holds none either.
const QV4::Function *madeBy(QV4::Heap::Base *item, Made made) {
    if (made == Made::Context) {
        const auto *called = static_cast<QV4::Heap::CallContext *>(item)->function.get();
        return called == nullptr ? nullptr : called->function;
    }
    if (made == Made::Closure)
        return static_cast<QV4::Heap::JavaScriptFunctionObject *>(item)->function;
    return nullptr;
}

// Megabytes as the line says them: whole from ten, a tenth below.
QString sized(double mb) { return QString::number(mb, 'f', mb >= 10 ? 0 : 1); }

} // namespace

QVariantMap Heap::census(int top) const {
    QV4::MemoryManager *manager = engine->handle()->memoryManager;
    // Mid-cycle, what's unmarked is garbage not yet swept, and would be
    // counted as live, so the census waits for the cycle to end.
    if (manager->gcStateMachine->inProgress())
        return {{QStringLiteral("busy"), true}};
    QElapsedTimer clock;
    clock.start();
    // Class names are static strings, so their addresses key them.
    QHash<const char *, Tally> byClass;
    QHash<const QV4::Heap::InternalClass *, Tally> byShape;
    // Closures by the function each is, and contexts by the function each
    // ran in, which says where in the shell's code they came from: what
    // was made since the last sweep, counted before a collection, or
    // what's kept, after one.
    QHash<const QV4::Function *, Tally> byPlace;
    const auto tally = [](Tally &into, double bytes) {
        into.bytes += bytes;
        into.count += 1;
    };
    const auto count = [&](QV4::Heap::Base *item, double bytes) {
        const QV4::Heap::InternalClass *shape = item->internalClass;
        if (shape == nullptr) {
            tally(byClass["unshaped"], bytes);
            return;
        }
        tally(byClass[shape->vtable->className], bytes);
        Tally &shaped = byShape[shape];
        if (shaped.made == Made::Unknown)
            shaped.made = madeAs(shape->vtable);
        tally(shaped, bytes);
        if (const QV4::Function *function = madeBy(item, shaped.made))
            tally(byPlace[function], bytes);
    };
    walk(manager->blockAllocator, count);
    // The shapes themselves live in an allocator of their own, and are
    // counted as one class without reading them as objects.
    walk(manager->icAllocator,
         [&](QV4::Heap::Base *, double bytes) { tally(byClass["InternalClass"], bytes); });
    for (const auto &huge : manager->hugeItemAllocator.chunks)
        count(reinterpret_cast<QV4::Heap::Base *>(huge.chunk->first()),
              static_cast<double>(huge.size));
    double total = 0;
    for (const Tally &each : std::as_const(byClass))
        total += each.bytes;
    QVariantList classes;
    QStringList classesSaid;
    for (const auto &[name, each] : largest(byClass, top)) {
        const QString named = QString::fromLatin1(name);
        classes.append(QVariant(QVariantList{named, megabytes(each.bytes), each.count}));
        classesSaid.append(QStringLiteral("%1 %2 MB (%3)")
                               .arg(named, sized(megabytes(each.bytes)))
                               .arg(each.count, 0, 'f', 0));
    }
    // Names are read only for the shapes said, since each is a string made.
    constexpr uint NAMES = 6;
    QVariantList shapes;
    QStringList shapesSaid;
    for (const auto &[shape, each] : largest(byShape, top)) {
        QStringList names;
        for (uint at = 0; at < shape->size && at < NAMES; ++at)
            names.append(said(QV4::Value::fromReturnedValue(shape->keyAt(at))));
        const QString named = QString::fromLatin1(shape->vtable->className);
        const QString keys = names.join(QLatin1Char(','));
        shapes.append(QVariant(QVariantList{named, keys, static_cast<double>(shape->size),
                                            megabytes(each.bytes), each.count}));
        shapesSaid.append(QStringLiteral("%1 {%2%3} %4 MB (%5)")
                              .arg(named, keys, shape->size > NAMES ? QStringLiteral(",...") : "",
                                   sized(megabytes(each.bytes)))
                              .arg(each.count, 0, 'f', 0));
    }
    // Functions said alike, two on one line or two in files said as `~`,
    // are one place before the largest are chosen, so no place is split
    // across the functions it holds.
    QHash<QString, Tally> bySaid;
    for (auto it = byPlace.cbegin(); it != byPlace.cend(); ++it) {
        Tally &into = bySaid[placeOf(it.key())];
        into.bytes += it.value().bytes;
        into.count += it.value().count;
    }
    QVariantList places;
    QStringList placesSaid;
    for (const auto &[place, each] : largest(bySaid, top)) {
        places.append(QVariant(QVariantList{place, megabytes(each.bytes), each.count}));
        placesSaid.append(QStringLiteral("%1 %2 MB (%3)")
                              .arg(place, sized(megabytes(each.bytes)))
                              .arg(each.count, 0, 'f', 0));
    }
    const double ms = static_cast<double>(clock.nsecsElapsed()) / 1e6;
    return {{QStringLiteral("classes"), classes},
            {QStringLiteral("shapes"), shapes},
            {QStringLiteral("places"), places},
            {QStringLiteral("total"), megabytes(total)},
            {QStringLiteral("ms"), ms},
            {QStringLiteral("said"), QStringLiteral("%1 MB counted in %2 ms: by class %3; by "
                                                    "shape %4; by place %5")
                                         .arg(sized(megabytes(total)))
                                         .arg(ms, 0, 'f', 0)
                                         .arg(classesSaid.join(QStringLiteral(", ")),
                                              shapesSaid.join(QStringLiteral(", ")),
                                              placesSaid.join(QStringLiteral(", ")))}};
}

namespace {

// Whether a class is, or is made from, another, as QObjectWrapper's kinds are.
bool isA(const QV4::VTable *vtable, const QV4::VTable *kind) {
    for (const QV4::VTable *each = vtable; each != nullptr; each = each->parent) {
        if (each == kind)
            return true;
    }
    return false;
}

// A chunk whose marks are read, and for a large item's own chunk, its bytes,
// since its slots run past what the chunk's bitmaps say.
struct Marked {
    QV4::Chunk *chunk;
    std::size_t largeBytes;
};

// The chunks of every allocator, so what marking sets can be read and
// cleared: items, shapes and large items.
std::vector<Marked> markedChunks(const QV4::MemoryManager *manager) {
    std::vector<Marked> chunks;
    for (QV4::Chunk *chunk : manager->blockAllocator.chunks)
        chunks.push_back({chunk, 0});
    for (QV4::Chunk *chunk : manager->icAllocator.chunks)
        chunks.push_back({chunk, 0});
    for (const auto &huge : manager->hugeItemAllocator.chunks)
        chunks.push_back({huge.chunk, huge.size});
    return chunks;
}

// What's newly marked since it last looked, so each holder's share is what
// marking from it alone added: it keeps a copy of every chunk's marks, and
// tallies each item whose mark wasn't in it.
class NewlyMarked {
public:
    explicit NewlyMarked(std::vector<Marked> chunks)
        : chunks(std::move(chunks)), seen(this->chunks.size() * QV4::Chunk::EntriesInBitmap) {}

    bool anyMarked() const {
        for (const Marked &each : chunks) {
            for (const quintptr word : each.chunk->blackBitmap) {
                if (word != 0)
                    return true;
            }
        }
        return false;
    }

    void take(Tally &into) {
        using QV4::Chunk;
        for (std::size_t at = 0; at < chunks.size(); ++at) {
            const Marked &each = chunks[at];
            quintptr *was = &seen[at * Chunk::EntriesInBitmap];
            // Most chunks gain no mark from one holder, so each is compared
            // whole first: it's read once per name in the shell's files.
            if (std::memcmp(was, each.chunk->blackBitmap, sizeof(each.chunk->blackBitmap)) == 0)
                continue;
            for (std::size_t word = 0; word < Chunk::EntriesInBitmap; ++word) {
                const quintptr now = each.chunk->blackBitmap[word];
                quintptr fresh = now & ~was[word];
                was[word] = now;
                while (fresh != 0) {
                    const auto bit = static_cast<std::size_t>(std::countr_zero(fresh));
                    fresh &= fresh - 1;
                    into.bytes += static_cast<double>(
                        each.largeBytes != 0 ? each.largeBytes
                                             : span(each.chunk, word * Chunk::Bits + bit) *
                                                   Chunk::SlotSize);
                    into.count += 1;
                }
            }
        }
    }

    // Leaves every mark as the sweep does, cleared, so the next collection
    // marks from nothing and what's let go meanwhile is collected.
    void clear() {
        for (const Marked &each : chunks)
            std::memset(each.chunk->blackBitmap, 0, sizeof(each.chunk->blackBitmap));
    }

private:
    static std::size_t span(const QV4::Chunk *chunk, std::size_t index) {
        using QV4::Chunk;
        constexpr auto last = static_cast<std::size_t>(Chunk::NumSlots);
        constexpr auto bits = static_cast<std::size_t>(Chunk::Bits);
        std::size_t length = 1;
        while (index + length < last &&
               ((chunk->extendsBitmap[(index + length) / bits] >> ((index + length) % bits)) &
                1U) != 0)
            ++length;
        return length;
    }

    std::vector<Marked> chunks;
    std::vector<quintptr> seen;
};

// The file that declares one level of an object's properties, as the
// holders' line says it: its name, after its folder's where that's named as
// code is, since the shell has a Spend.qml in launcher/ and in pieces/; or
// `~` for one not named as code is. Each level is its own file's, as
// `Base { property var added }` in Launcher.qml declares `added` there and
// Base's own in Base.qml. The filters keep data out of the line only while
// the shell's files are named in its code, as they are: a file named for a
// session or a host, with no space, would pass them.
QString fileSaid(const QQmlVMEMetaObject *level) {
    static const QRegularExpression plain(QStringLiteral("^[A-Za-z0-9_.-]{1,48}$"));
    static const QRegularExpression folder(QStringLiteral("^[A-Za-z_][A-Za-z0-9_]{0,31}$"));
    const auto unit = level->compilationUnit();
    if (!unit)
        return QStringLiteral("~");
    const QStringList path = unit->finalUrl().path().split(QLatin1Char('/'), Qt::SkipEmptyParts);
    const QString file = path.value(path.size() - 1);
    if (!plain.match(file).hasMatch())
        return QStringLiteral("~");
    const QString in = path.value(path.size() - 2);
    return folder.match(in).hasMatch() ? in + QLatin1Char('/') + file : file;
}

// One property's value, and the holder it's counted for.
struct Held {
    QString key;
    QV4::Heap::MemberData *storage;
    uint index;
};

// Why a count of holders can't mark now, or nothing: a collection under way,
// or marks set with no collection under way, which would hide what they mark
// from every holder, and clearing them might undo work it trusts; they're
// said apart, since the sweep clears them all and none should be there.
QVariantMap unmarkable(const QV4::MemoryManager *manager, const NewlyMarked &marks) {
    if (manager->gcStateMachine->inProgress())
        return {{QStringLiteral("busy"), true}};
    if (marks.anyMarked())
        return {{QStringLiteral("busy"), true}, {QStringLiteral("marked"), true}};
    return {};
}

// What eachHolder() finds besides the properties' values.
enum class Found { Context, Object, Storage };

// Calls `onFound` with each QML context and object wrapper in the engine's
// heap and each object's properties' storage, and `onHeld` with every
// managed value a storage holds, under the holder it's counted for: the
// one walk holders() and freedBy() name what they count by.
template <typename OnFound, typename OnHeld>
void eachHolder(QV4::ExecutionEngine *v4, OnFound onFound, OnHeld onHeld) {
    QSet<const QObject *> found;
    walk(v4->memoryManager->blockAllocator, [&](QV4::Heap::Base *item, double) {
        if (item->internalClass == nullptr)
            return;
        const QV4::VTable *vtable = item->internalClass->vtable;
        if (vtable == QV4::QQmlContextWrapper::staticVTable() ||
            vtable == QV4::QmlContext::staticVTable()) {
            onFound(item, Found::Context);
            return;
        }
        if (!isA(vtable, QV4::QObjectWrapper::staticVTable()))
            return;
        onFound(item, Found::Object);
        QObject *object = static_cast<QV4::Heap::QObjectWrapper *>(item)->object();
        if (object == nullptr || found.contains(object))
            return;
        found.insert(object);
        const QMetaObject *meta = object->metaObject();
        for (QQmlVMEMetaObject *vme = QQmlVMEMetaObject::get(object); vme != nullptr;
             vme = vme->parentVMEMetaObject()) {
            // Another engine's storage sits in chunks whose marks it never
            // clears, as Qt's own marking skips it (QQmlVMEMetaObject::mark).
            if (vme->engine() != v4)
                continue;
            QV4::MemberData *storage = vme->propertyAndMethodStorageAsMemberData();
            if (storage == nullptr)
                continue;
            const QString file = fileSaid(vme);
            onFound(storage->d(), Found::Storage);
            const auto count = std::min<uint>(static_cast<uint>(vme->propCount()),
                                              storage->size());
            for (uint at = 0; at < count; ++at) {
                const QV4::Value &value = storage->d()->values[at];
                if (!value.isManaged())
                    continue;
                const QString name = saidName(QString::fromLatin1(
                    meta->property(vme->propOffset() + static_cast<int>(at)).name()));
                onHeld(Held{file + QLatin1Char(' ') + name, storage->d(), at});
            }
            // The file's functions follow its properties in the same
            // storage, each made anew for every object, and count together.
            const QString functions = file + QStringLiteral(" (functions)");
            for (uint at = count; at < storage->size(); ++at) {
                if (storage->d()->values[at].isManaged())
                    onHeld(Held{functions, storage->d(), at});
            }
        }
    });
}

// What every count of holders starts from: what the heap holds in all, and
// every object, its properties' storage and every QML context marked
// without what it reaches, so marking from one property stops at them and
// what another object's properties hold is counted for them; with every
// property's value listed under its holder, in the order of their names.
struct Premarked {
    double total = 0;
    std::vector<QV4::Heap::Base *> objects;
    std::vector<QV4::Heap::Base *> contexts;
    std::vector<Held> held;
};

Premarked premarked(QV4::ExecutionEngine *v4) {
    QV4::MemoryManager *manager = v4->memoryManager;
    Premarked from;
    const auto weigh = [&from](QV4::Heap::Base *, double bytes) { from.total += bytes; };
    walk(manager->blockAllocator, weigh);
    walk(manager->icAllocator, weigh);
    for (const auto &huge : manager->hugeItemAllocator.chunks)
        from.total += static_cast<double>(huge.size);
    eachHolder(
        v4,
        [&from](QV4::Heap::Base *item, Found found) {
            if (item->isMarked())
                return;
            item->setMarkBit();
            if (found == Found::Context)
                from.contexts.push_back(item);
            else if (found == Found::Object)
                from.objects.push_back(item);
        },
        [&from](Held each) { from.held.push_back(std::move(each)); });
    std::stable_sort(from.held.begin(), from.held.end(),
                     [](const Held &a, const Held &b) { return a.key < b.key; });
    return from;
}

// What the engine keeps of its own, as the collector's first steps mark
// it: its builtins and global object, the shapes and names it knows, and
// the code it has compiled with the shapes that code has cached. It's
// marked before any property, since otherwise the first to reach a plain
// object would take Object.prototype and all it leads to, and cutting a
// property frees none of it.
void markEngine(QV4::ExecutionEngine *v4, QV4::MarkStack &stack) {
    v4->markObjects(&stack);
    v4->memoryManager->collectFromJSStack(&stack);
    stack.drain();
}

// What the collector marks as its roots, in the order its mark states do
// (qv4mm.cpp): the engine's own, the JavaScript stack, the values C++ keeps
// (QJSValues, connected closures, the scripts a context imports), and each
// object's wrapper whose object stays alive unreached, being indestructible
// or in a tree whose root is (markWeakValues). Others are kept only if
// something reaches them, as an object made in script with no parent is.
void markRoots(QV4::ExecutionEngine *v4, QV4::MarkStack &stack) {
    markEngine(v4, stack);
    QV4::MemoryManager *manager = v4->memoryManager;
    if (manager->m_persistentValues != nullptr) {
        manager->m_persistentValues->mark(&stack);
        stack.drain();
    }
    for (auto at = manager->m_weakValues->begin(); at.p != nullptr; ++at) {
        QV4::QObjectWrapper *wrapper = (*at).as<QV4::QObjectWrapper>();
        if (wrapper == nullptr)
            continue;
        const QObject *object = wrapper->object();
        if (object == nullptr)
            continue;
        bool alive = QQmlData::keepAliveDuringGarbageCollection(object);
        if (!alive && object->parent() != nullptr) {
            const QObject *root = object->parent();
            while (root->parent() != nullptr)
                root = root->parent();
            alive = QQmlData::keepAliveDuringGarbageCollection(root);
        }
        if (alive) {
            wrapper->mark(&stack);
            stack.drain();
        }
    }
}

// Marks one property's value and all it reaches.
void markHeld(const Held &each, QV4::MarkStack &stack) {
    each.storage->values[each.index].mark(&stack);
    stack.drain();
}

// What contexts and the scripts they import, or objects otherwise, reach
// that nothing marked before them did.
Tally through(const std::vector<QV4::Heap::Base *> &from, QV4::MarkStack &stack,
              NewlyMarked &marks) {
    Tally reached;
    for (QV4::Heap::Base *item : from) {
        item->internalClass->vtable->markObjects(item, &stack);
        stack.drain();
    }
    marks.take(reached);
    return reached;
}

} // namespace

QVariantMap Heap::holders(int top, int namingMs) const {
    QV4::ExecutionEngine *v4 = engine->handle();
    NewlyMarked marks(markedChunks(v4->memoryManager));
    if (const QVariantMap busy = unmarkable(v4->memoryManager, marks); !busy.isEmpty())
        return busy;
    // From here to marks.clear(), nothing may make anything in the engine's
    // heap or run JavaScript: either could start a collection with these
    // marks set, which would take them as its own and free what they hide.
    // What it makes, QStrings, lists and maps, is C++'s own.
    QElapsedTimer clock;
    clock.start();
    const Premarked from = premarked(v4);
    const std::vector<Held> &held = from.held;
    Tally themselves;
    marks.take(themselves);
    QV4::MarkStack stack(v4);
    markEngine(v4, stack);
    Tally engineOwn;
    marks.take(engineOwn);
    QHash<QString, Tally> byHolder;
    std::vector<const Held *> unnamedHeld;
    int names = 0;
    int named = 0;
    QElapsedTimer naming;
    naming.start();
    for (std::size_t at = 0; at < held.size(); ++names) {
        const QString &key = held[at].key;
        if (naming.elapsed() >= namingMs) {
            for (; at < held.size() && held[at].key == key; ++at)
                unnamedHeld.push_back(&held[at]);
            continue;
        }
        ++named;
        for (; at < held.size() && held[at].key == key; ++at)
            markHeld(held[at], stack);
        // One reaching nothing another hadn't, as a property holding
        // another object does, isn't said.
        Tally reached;
        marks.take(reached);
        if (reached.count > 0)
            byHolder.insert(key, reached);
    }
    // Names it had no time for are marked together and said as properties
    // unnamed: their storage is marked already, so nothing after would
    // reach what they hold.
    for (const Held *each : unnamedHeld)
        markHeld(*each, stack);
    Tally unnamed;
    marks.take(unnamed);
    // What no property reaches, through the contexts and the scripts they
    // import, then through the objects otherwise.
    const Tally fromContexts = through(from.contexts, stack, marks);
    const Tally fromObjects = through(from.objects, stack, marks);
    marks.clear();
    double counted = themselves.bytes + engineOwn.bytes + unnamed.bytes + fromContexts.bytes +
                     fromObjects.bytes;
    for (const Tally &each : std::as_const(byHolder))
        counted += each.bytes;
    const double total = from.total;
    const double rest = std::max(0.0, total - counted);
    QVariantList holding;
    QStringList holdingSaid;
    for (const auto &[key, each] : largest(byHolder, top)) {
        holding.append(QVariant(QVariantList{key, megabytes(each.bytes), each.count}));
        holdingSaid.append(QStringLiteral("%1 %2 MB (%3)")
                               .arg(key, sized(megabytes(each.bytes)))
                               .arg(each.count, 0, 'f', 0));
    }
    const QString stopped =
        named < names ? QStringLiteral(" (%1 of %2 names before its %3 ms were up, the rest's %4 "
                                       "MB unnamed)")
                            .arg(named)
                            .arg(names)
                            .arg(namingMs)
                            .arg(sized(megabytes(unnamed.bytes)))
                      : QString();
    const double ms = static_cast<double>(clock.nsecsElapsed()) / 1e6;
    return {{QStringLiteral("holders"), holding},
            {QStringLiteral("names"), names},
            {QStringLiteral("named"), named},
            {QStringLiteral("themselves"), megabytes(themselves.bytes)},
            {QStringLiteral("engine"), megabytes(engineOwn.bytes)},
            {QStringLiteral("unnamed"), megabytes(unnamed.bytes)},
            {QStringLiteral("contexts"), megabytes(fromContexts.bytes)},
            {QStringLiteral("objects"), megabytes(fromObjects.bytes)},
            {QStringLiteral("rest"), megabytes(rest)},
            {QStringLiteral("total"), megabytes(total)},
            {QStringLiteral("ms"), ms},
            {QStringLiteral("said"),
             QStringLiteral("%1 MB counted in %2 ms: by property%3 %4; the objects, their "
                            "storage and contexts themselves %5 MB; the engine's own %6 MB; "
                            "reached through contexts and scripts %7 MB, through objects "
                            "otherwise %8 MB, from none of these %9 MB")
                 .arg(sized(megabytes(total)))
                 .arg(ms, 0, 'f', 0)
                 .arg(stopped, holdingSaid.join(QStringLiteral(", ")),
                      sized(megabytes(themselves.bytes)), sized(megabytes(engineOwn.bytes)),
                      sized(megabytes(fromContexts.bytes)),
                      sized(megabytes(fromObjects.bytes)), sized(megabytes(rest)))}};
}

QVariantMap Heap::freedBy(const QString &holder) const {
    QV4::ExecutionEngine *v4 = engine->handle();
    NewlyMarked marks(markedChunks(v4->memoryManager));
    if (const QVariantMap busy = unmarkable(v4->memoryManager, marks); !busy.isEmpty())
        return busy;
    // As in holders(), nothing from here to marks.clear() may make anything
    // in the engine's heap or run JavaScript.
    QElapsedTimer clock;
    clock.start();
    // Where the holder's values sit, by the storage holding them; the
    // object each storage is for, whose wrapper eachHolder() finds just
    // before the storage; and every wrapper of each object, since Qt can
    // wrap one twice in an engine (a const wrapper beside the usual one),
    // and marking either marks its storage.
    QHash<QV4::Heap::MemberData *, QSet<uint>> own;
    QHash<QV4::Heap::MemberData *, const QObject *> objectOf;
    QMultiHash<const QObject *, QV4::Heap::Base *> wrappersOf;
    const QObject *lastObject = nullptr;
    eachHolder(
        v4,
        [&objectOf, &wrappersOf, &lastObject](QV4::Heap::Base *item, Found found) {
            if (found == Found::Object) {
                lastObject = static_cast<QV4::Heap::QObjectWrapper *>(item)->object();
                if (lastObject != nullptr)
                    wrappersOf.insert(lastObject, item);
            } else if (found == Found::Storage) {
                objectOf.insert(static_cast<QV4::Heap::MemberData *>(item), lastObject);
            }
        },
        [&own, &holder](const Held &each) {
            if (each.key == holder)
                own[each.storage].insert(each.index);
        });
    // A name nothing holds isn't said or given back, since only holders()'
    // own names, filtered for the journal, are known to be safe to say.
    if (own.isEmpty())
        return {{QStringLiteral("held"), false},
                {QStringLiteral("freed"), 0.0},
                {QStringLiteral("count"), 0.0},
                {QStringLiteral("ms"), static_cast<double>(clock.nsecsElapsed()) / 1e6},
                {QStringLiteral("said"), QStringLiteral("no property holds anything so named")}};
    // Marked as the next collection would mark, but for the holder's own
    // values: the storage holding them first, without what it reaches, then
    // the collector's roots, then the other values of each storage whose
    // object those reach. What the holder's values add after them is what
    // nothing else keeps, so what that collection would free were they let
    // go. A storage whose object nothing reaches is the sweep's whatever
    // happens, as an object made in script and dropped since the last
    // collection is, so none of what it holds is counted.
    QV4::MarkStack stack(v4);
    for (auto at = own.cbegin(); at != own.cend(); ++at)
        at.key()->setMarkBit();
    markRoots(v4, stack);
    // Takes in each storage whose object is now reached, marking its
    // values, all of them or all but the holder's, until marking them
    // reaches no other; an object reached only through another's storage
    // is kept or freed with it.
    QSet<QV4::Heap::MemberData *> reached;
    const auto isReached = [&objectOf, &wrappersOf](QV4::Heap::MemberData *storage) {
        const QObject *object = objectOf.value(storage);
        for (auto at = wrappersOf.constFind(object);
             object != nullptr && at != wrappersOf.cend() && at.key() == object; ++at) {
            if (at.value()->isMarked())
                return true;
        }
        return false;
    };
    const auto takeIn = [&](bool holderToo) {
        for (bool more = true; more;) {
            more = false;
            for (auto at = own.cbegin(); at != own.cend(); ++at) {
                if (reached.contains(at.key()) || !isReached(at.key()))
                    continue;
                reached.insert(at.key());
                more = true;
                for (uint index = 0; index < at.key()->values.size; ++index) {
                    if (holderToo || !at.value().contains(index))
                        markHeld(Held{QString(), at.key(), index}, stack);
                }
            }
        }
    };
    takeIn(false);
    Tally kept;
    marks.take(kept);
    for (QV4::Heap::MemberData *storage : std::as_const(reached)) {
        for (const uint index : own.value(storage))
            markHeld(Held{QString(), storage, index}, stack);
    }
    takeIn(true);
    Tally alone;
    marks.take(alone);
    marks.clear();
    const double ms = static_cast<double>(clock.nsecsElapsed()) / 1e6;
    return {{QStringLiteral("holder"), holder},
            {QStringLiteral("held"), true},
            {QStringLiteral("freed"), megabytes(alone.bytes)},
            {QStringLiteral("count"), alone.count},
            {QStringLiteral("ms"), ms},
            {QStringLiteral("said"),
             QStringLiteral("letting go of %1 would free %2 MB of the heap (%3), counted in %4 ms")
                 .arg(holder, sized(megabytes(alone.bytes)))
                 .arg(alone.count, 0, 'f', 0)
                 .arg(ms, 0, 'f', 0)}};
}

double Heap::collected() const {
    constexpr double MICROSECONDS_PER_MS = 1000.0;
    qint64 microseconds = 0;
    for (const auto &step : engine->handle()->memoryManager->gcStateMachine->executionTiming)
        microseconds += step.rolling_sum;
    return static_cast<double>(microseconds) / MICROSECONDS_PER_MS;
}
