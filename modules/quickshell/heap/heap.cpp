// Qt's memory manager keeps the heap's own count of it, which no public
// API gives, so the module reads it through Qt's private headers, built
// with the very Qt the shell runs (modules/quickshell/nixos.nix), whose
// private symbols are tied to its version. Each count walks the heap's
// chunks once, a few milliseconds at most, so it's read only when asked,
// as the launcher asks once a closing's collection is done.
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

Heap::Heap(QJSEngine *engine, QObject *parent) : QObject(parent), engine(engine) {}

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
