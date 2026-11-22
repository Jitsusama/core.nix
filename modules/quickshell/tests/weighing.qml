// When the launcher weighs what holds its heap, and what, run in Qt's own
// JavaScript engine, the one the shell runs it in. The
// `the-launcher-weighs-what-it-owes` check copies
// modules/quickshell/launcher/Weighing.js and tests/lib/Expect.js beside
// this file and runs `qml -platform offscreen weighing.qml`. The holders
// are counted once a shell, after the first census; a busy count or
// weighing, or one an opening put off, stays owed for the next closing;
// folded names aren't weighed; and nothing is counted or weighed again.
import QtQuick
import "Expect.js" as Expect
import "Weighing.js" as Weighing

QtObject {
    Component.onCompleted: Expect.run(cases)

    readonly property var busy: ({ busy: true })
    readonly property var done: ({ held: true })

    function rows(names) {
        return { holders: names.map(name => [name, 1]) };
    }

    // A shell's first closings, before the census is due, start nothing.
    function startsNothingBeforeTheCensus() {
        const state = Weighing.begun();
        Expect.same("a closing before the census", Weighing.afterClosing(state, false), "none");
        Expect.same("the census when due", Weighing.afterClosing(state, true), "census");
    }

    // The first census owes a count, and the count queues the largest
    // cuttable names; then each weighing takes one off, and once all are
    // weighed nothing more is counted or weighed again, census or not.
    function countsOnceAndWeighsEach() {
        let state = Weighing.afterCensus(Weighing.begun());
        Expect.same("a census owes a count", state.owed, true);
        Expect.same("the count's turn counts", Weighing.countsTurn(state), "count");
        state = Weighing.counted(state, rows(["a.qml held", "b.qml all"]));
        Expect.same("the count queues both", state.queue, ["a.qml held", "b.qml all"]);
        Expect.same("counted, nothing owed", [state.counted, state.owed], [true, false]);
        Expect.same("the turn then weighs", Weighing.countsTurn(state), "weigh");
        state = Weighing.weighed(state, done);
        Expect.same("a weighing takes its name off", state.queue, ["b.qml all"]);
        state = Weighing.weighed(state, { held: false });
        Expect.same("a name holding nothing now leaves too", state.queue, []);
        Expect.same("a closing then starts nothing", Weighing.afterClosing(state, false), "none");
        state = Weighing.afterCensus(state);
        Expect.same("a later census owes no count", state.owed, false);
        Expect.same("and its turn does nothing", Weighing.countsTurn(state), "none");
    }

    // A busy count stays owed, so the next closing resumes it.
    function aBusyCountStaysOwed() {
        const owing = Weighing.afterCensus(Weighing.begun());
        const state = Weighing.counted(owing, busy);
        Expect.same("a busy count changes nothing", state, owing);
        Expect.same("the next closing resumes", Weighing.afterClosing(state, false), "resume");
        Expect.same("and counts", Weighing.countsTurn(state), "count");
    }

    // A busy weighing, or one an opening put off, which changes nothing,
    // keeps its name first, so the next closing resumes with it.
    function aBusyWeighingKeepsTheQueue() {
        const queued = Weighing.counted(Weighing.begun(), rows(["a.qml held", "b.qml all"]));
        const state = Weighing.weighed(queued, busy);
        Expect.same("a busy weighing keeps the queue", state.queue, ["a.qml held", "b.qml all"]);
        Expect.same("the next closing resumes", Weighing.afterClosing(state, false), "resume");
        Expect.same("and weighs", Weighing.countsTurn(state), "weigh");
        Expect.same("a census due takes it up too",
            Weighing.countsTurn(Weighing.afterCensus(state)), "weigh");
    }

    // Folded names, wherever `~` stands in them, aren't weighed; a file's
    // functions are; at most six are queued, and fewer when fewer are held.
    function weighsWhatCouldBeCut() {
        const state = Weighing.counted(Weighing.begun(), rows([
            "launcher/ReadyRows.qml held", "~ held", "Now.qml ~", "Spend.qml (functions)",
            "b all", "c all", "d all", "e all", "f all"
        ]));
        Expect.same("the six largest cuttable", state.queue, [
            "launcher/ReadyRows.qml held", "Spend.qml (functions)", "b all", "c all", "d all",
            "e all"
        ]);
        Expect.same("fewer when fewer are held",
            Weighing.counted(Weighing.begun(), rows(["a all", "~ held"])).queue, ["a all"]);
        const none = Weighing.counted(Weighing.begun(), rows([]));
        Expect.same("none held, nothing to weigh", Weighing.countsTurn(none), "none");
    }

    function cases() {
        startsNothingBeforeTheCensus();
        countsOnceAndWeighsEach();
        aBusyCountStaysOwed();
        aBusyWeighingKeepsTheQueue();
        weighsWhatCouldBeCut();
    }
}
