.pragma library
.import "Order.js" as Order

// The names that counted most, as the launcher's journal lines say which
// sources made rows and which changed: the most first, then A to Z as
// Order.js reads names, each with its count, at most `few` of them, and
// none that counted nothing. modules/quickshell/tests/most.qml holds it in
// Qt's engine.

// The most first, then by name, Order.js's last tie.
const mostFirst = Order.then(
    Order.larger(each => each.count),
    Order.byName(each => each.name)
);

// "name count" for each of the most, joined by commas, or "" for none.
function said(counts, few) {
    const counted = Object.keys(counts)
        .filter(name => counts[name] > 0)
        .map(name => ({ name: name, count: counts[name] }));
    return Order.sorted(counted, mostFirst)
        .slice(0, few)
        .map(each => `${each.name} ${each.count}`)
        .join(", ");
}
