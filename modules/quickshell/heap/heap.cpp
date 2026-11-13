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
// heap_test.cpp holds it to what it says.

#include "heap.h"

#include <QJSEngine>
#include <private/qv4engine_p.h>
#include <private/qv4mm_p.h>

namespace {

constexpr double BYTES_PER_MB = 1024.0 * 1024.0;

double megabytes(std::size_t bytes) { return static_cast<double>(bytes) / BYTES_PER_MB; }

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
    };
}

double Heap::collected() const {
    constexpr double MICROSECONDS_PER_MS = 1000.0;
    qint64 microseconds = 0;
    for (const auto &step : engine->handle()->memoryManager->gcStateMachine->executionTiming)
        microseconds += step.rolling_sum;
    return static_cast<double>(microseconds) / MICROSECONDS_PER_MS;
}
