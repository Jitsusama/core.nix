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
// (2026-11-19). It never lets the chunks grow past twice the room beyond
// what they held at the first ask since a collection ended, so a heap whose
// sweeps keep a few items in each chunk it grew into doesn't grow by the
// room at every one, and the opening's ask shares the closing's; and it
// reckons from what each sweep left, not from where leaveRoom set it, so
// asking twice leaves the room once. weigh() says where the next starts.
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
// heap_test.cpp holds it to what it says.

#include "heap.h"

#include <QElapsedTimer>
#include <QHash>
#include <QJSEngine>
#include <QRegularExpression>
#include <QStringList>
#include <QVariantList>
#include <algorithm>
#include <cmath>
#include <cstdint>
#include <dlfcn.h>
#include <private/qv4context_p.h>
#include <private/qv4engine_p.h>
#include <private/qv4function_p.h>
#include <private/qv4functionobject_p.h>
#include <private/qv4internalclass_p.h>
#include <private/qv4mm_p.h>
#include <sys/mman.h>
#include <unistd.h>
#include <vector>

namespace {

constexpr double BYTES_PER_MB = 1024.0 * 1024.0;

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

// Gives an allocator's free pages back to the kernel, as the shell's Qt
// gives back a chunk it decommits, and says how many of their bytes were
// resident. The shell's Qt reserves the heap as anonymous memory, which
// this lets go of; a Qt that maps it from a memfd keeps the pages there.
// Spans with none resident are left alone, so the call costs nothing for
// them.
std::size_t givenBack(const QV4::BlockAllocator &allocator) {
    std::size_t bytes = 0;
    eachFreeSpan(allocator, [&bytes](char *at, std::size_t length, std::size_t resident) {
        if (resident > 0 && madvise(at, length, MADV_DONTNEED) == 0)
            bytes += resident;
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
    const std::size_t slots =
        manager->blockAllocator.totalSlots() + manager->icAllocator.totalSlots();
    return {
        {QStringLiteral("live"), megabytes(manager->getRegularItemsMem() + large)},
        {QStringLiteral("held"), megabytes(manager->getAllocatedMem())},
        {QStringLiteral("chunks"), megabytes(slots * QV4::Chunk::SlotSize)},
        {QStringLiteral("collectsAt"), megabytes(manager->heldBeforeCollecting())},
        {QStringLiteral("collectsOutsideAt"), megabytes(manager->unmanagedHeapSizeGCLimit)},
        {QStringLiteral("outside"), megabytes(manager->unmanagedHeapSize)},
        {QStringLiteral("free"),
         megabytes(freePages(manager->blockAllocator) + freePages(manager->icAllocator))},
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
    return megabytes(givenBack(manager->blockAllocator) + givenBack(manager->icAllocator));
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
QString said(const QV4::Value &name) {
    if (name.isUndefined())
        return QStringLiteral("-");
    static const QRegularExpression plain(QStringLiteral("^[A-Za-z_$][A-Za-z0-9_$]*$"));
    const QString text = name.isString() ? name.toQStringNoThrow() : QString();
    return text.size() <= NAME_LENGTH && plain.match(text).hasMatch() ? text
                                                                       : QStringLiteral("~");
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

double Heap::collected() const {
    constexpr double MICROSECONDS_PER_MS = 1000.0;
    qint64 microseconds = 0;
    for (const auto &step : engine->handle()->memoryManager->gcStateMachine->executionTiming)
        microseconds += step.rolling_sum;
    return static_cast<double>(microseconds) / MICROSECONDS_PER_MS;
}
