.pragma library

// Whether a source's rows are wanted now, from where the launcher stands, so
// a source whose rows cost much builds them only then and a change nobody
// sees builds none (Provider.qml's wanted). One opening on Joel's shell made
// 6,933 rows, most of them sessions' that home only counts (Joel,
// 2026-11-08: "weird lag can happen").
//
// `stands` is what the launcher says of itself: whether its catalogue is
// walked from outside (`cataloguing`), whether it's `open`, the steps
// `held`, each with the row it stepped into, the acts `becoming`, each with
// the source whose rows it waits on, and what's `typed`.
function wanted(stands, source) {
    // Walked from outside, every row is read.
    if (stands.cataloguing) {
        return true;
    }
    if (!stands.open) {
        return false;
    }
    // Held, or with an act of its own waiting to be agreed, its rows are
    // read for what's shown and for what the act becomes.
    if (stands.held.some(step => step.row?.from === source)
        || stands.becoming.some(one => one.from === source)) {
        return true;
    }
    // In another place, only that place is shown and searched.
    if (stands.held.length > 0) {
        return false;
    }
    // At home, what's typed searches every place, and home names what each
    // holds, unless it says what it holds itself, as Agents does.
    return stands.typed !== "" || !source.saysAtHome;
}
