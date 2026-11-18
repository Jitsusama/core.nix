// What a search does about the answers it waits for, run in Qt's own
// JavaScript engine, the one the shell runs it in. The
// `the-launcher-holds-a-key-for-its-answers` check copies
// modules/quickshell/launcher/Awaited.js, with what it imports, into
// launcher/ and tests/lib/Expect.js beside this file, and runs it
// offscreen. A key typed and Return pressed inside the few milliseconds a
// key is held can't be timed on niri, so each case here is the state
// Launcher.qml's searchNow finds as a key or a setter comes.
import QtQuick
import "Expect.js" as Expect
import "launcher/Awaited.js" as Awaited

QtObject {
    // A case that doesn't hold throws, and ends the test as a failure.
    Component.onCompleted: Expect.run(cases)

    // A key as Launcher.qml's keysDue holds it, named by what it typed.
    function key(text) {
        return { text: text, began: 0 };
    }

    // The sources a search waits on, counted as they're asked.
    property int asked: 0
    function waitingOn(sources) {
        return () => {
            asked += 1;
            return sources;
        };
    }
    readonly property var notices: ["notices"]

    // The launcher as a search comes, with none of its keys searched yet
    // (`due`), the key held for answers or null, whether typing asked, and
    // whether a question is open.
    function at(due, held, typing, asking) {
        return { due: due, held: held, typing: typing, asking: asking };
    }

    function cases() {
        const fo = { due: [key("f"), key("o")], hold: notices };

        Expect.same("a key whose search awaits an answer is held",
            Awaited.next(at([key("f")], null, true, false), waitingOn(notices)),
            { hold: notices, due: [key("f")] });
        Expect.same("a key none waits on is searched at once",
            Awaited.next(at([key("f")], null, true, false), waitingOn([])),
            { due: [key("f")], waited: null });

        // Typed over a held key, the held one's search is dropped, and its
        // key timed by the search of the one typed over it.
        Expect.same("a key typed while one is held holds both, searching neither",
            Awaited.next(at([key("r")], fo, true, false), waitingOn(notices)),
            { hold: notices, due: [key("f"), key("o"), key("r")] });
        Expect.same("a key typed over a held one with nothing to wait on searches now",
            Awaited.next(at([key("r")], fo, true, false), waitingOn([])),
            { due: [key("f"), key("o"), key("r")], waited: null });

        // Return, or anything else that reads what was found, searches the
        // held key at once, what has answered and no more.
        Expect.same("Return while a key is held searches it now, ending its wait",
            Awaited.next(at([], fo, false, false), waitingOn(notices)),
            { due: [key("f"), key("o")], waited: fo });
        Expect.same("Return with keys unsearched searches them without holding",
            Awaited.next(at([key("f")], null, false, false), waitingOn(notices)),
            { due: [key("f")], waited: null });

        // Nothing due and nothing held, nothing is searched; a turn's end
        // with nothing new typed leaves the held key waiting.
        const before = asked;
        Expect.same("nothing typed and nothing held searches nothing",
            Awaited.next(at([], null, false, false), waitingOn(notices)), null);
        Expect.same("a turn ending with nothing new leaves the held key held",
            Awaited.next(at([], fo, true, false), waitingOn(notices)), null);
        Expect.same("a question open waits on no answer",
            Awaited.next(at([key("y")], null, true, true), waitingOn(notices)),
            { due: [key("y")], waited: null });
        Expect.expect("the sources are asked only when a key could be held",
            asked === before, `asked ${asked - before} times`);

        // A key is counted from what the field read before it, a held key's
        // text too, though it isn't searched yet: from what was last
        // searched, the held key's letters were counted again, eight
        // letters for kitty's five (80ECO2, 2026-11-12).
        Expect.same("a key comes after the held one",
            Awaited.before(at([], { due: [key("ki")] }, true, false), "k"), "ki");
        Expect.same("or after the last one due",
            Awaited.before(at([key("kit")], { due: [key("ki")] }, true, false), "k"), "kit");
        Expect.same("or, with none, after what was searched",
            Awaited.before(at([], null, true, false), "k"), "k");

        // A wait its bound ended is said apart from the time past that
        // bound, when the shell's thread was busy with other work and its
        // timer couldn't fire: "ca" said 98 ms on answers against a bound
        // of 8, behind two drawers drawing (Y2YBTS, 2026-11-12, 15:46).
        const asked98 = { began: 1000 };
        Expect.same("a wait its bound ended late says how far past it",
            Awaited.waitedOf(asked98, 1098, 8, true), { waited: 98, behind: 90 });
        Expect.same("a wait its bound ended on time is behind nothing",
            Awaited.waitedOf(asked98, 1008, 8, true), { waited: 8, behind: 0 });
        Expect.same("a wait an answer ended is behind nothing, however long",
            Awaited.waitedOf(asked98, 1030, 8, false), { waited: 30, behind: 0 });

        // What held the shell's thread past the bound is named by the work
        // noted while the key was held, by the shell's own clock: Joel's
        // "mut" waited 77 ms on answers, 69 of it behind the shell's other
        // work, unnamed (RTOIMQ, 2026-11-17). Where two run at once, the
        // one begun last has the time, so none counts twice; the collector
        // is said of the whole wait, since it may run inside any of them.
        const held = { from: 1000, collected: 50 };
        const noted = [
            { name: "agents changing", from: 1000, to: 1012 },
            { name: "syncing", from: 1010, to: 1050 },
            { name: "laying out", from: 1050, to: 1080 }
        ];
        Expect.same("a wait past its bound names what ran past it, most first",
            Awaited.behindOn(held, { at: 1098, spans: noted, collected: 60 }, 8),
            "syncing 40, laying out 30, agents changing 2, 18 unnamed;"
                + " collecting 10 over the whole wait");
        Expect.same("work done before the bound isn't what the key waited behind,"
            + " nor a collector that took none",
            Awaited.behindOn(held, { at: 1098, collected: 50,
                spans: [{ name: "showing", from: 1000, to: 1008 }] }, 8),
            "90 unnamed");
        Expect.same("a wait ended on its bound names nothing",
            Awaited.behindOn(held, { at: 1008, spans: noted, collected: 60 }, 8), "");

        // A bound can end on the timer queued behind a frame's sync, ahead
        // of the call that notes it, as Pacing.feltFramed's frame then holds
        // it: begun, not yet heard to end.
        Expect.same("a sync not yet heard to end runs to the time asked, beside rows laid out",
            Awaited.frameOpen({ rows: { from: 30, to: 34 }, syncFrom: 20 }, 50), [
                { name: "syncing", from: 20, to: 50 }, { name: "laying out", from: 30, to: 34 }]);
        Expect.same("a frame all said has nothing open",
            Awaited.frameOpen({ rows: null, syncFrom: -1 }, 50), []);
        const syncing = { rows: null, syncFrom: 1010 };
        Expect.same("a sync still open when the bound fired is what the key waited behind",
            Awaited.heldFor({ began: 0, from: 1000, collected: 50, timedOut: true },
                { at: 1060, collected: 50, spans: Awaited.frameOpen(syncing, 1050) }, 8),
            { waited: 60, behind: 52, behindOn: "syncing 40, 12 unnamed" });
        Expect.same("a held key's open sync ends where its bound's timer fired",
            Awaited.heldOpen(syncing, { firedAt: 1050 }, 1060),
            [{ name: "syncing", from: 1010, to: 1050 }]);
        Expect.same("a held key all answered has its open sync run to the listing",
            Awaited.heldOpen(syncing, {}, 1060), [{ name: "syncing", from: 1010, to: 1060 }]);

        // A key typed over a held one, or an opening felt while a key is
        // held, goes on noting the frame in flight; one held with nothing
        // listening notes a fresh one.
        Expect.same("a hold begun while another listened keeps the frame in flight",
            Awaited.frameHeld(syncing, true), syncing);
        Expect.same("a hold begun with nothing listening notes a fresh frame",
            Awaited.frameHeld(syncing, false), { rows: null, syncFrom: -1 });

        // A held key's line is timed from when it was held by the shell's
        // clock, never the wall clock's, which reads another time: timed
        // apart, one said 2 ms behind and named 1 (the keys lab,
        // 2026-11-18).
        const late = { began: 1712000000000, from: 1000, collected: 50, timedOut: true };
        Expect.same("a key its bound ended says its wait and what held it, on one clock",
            Awaited.heldFor(late, { at: 1098, spans: noted, collected: 50 }, 8),
            { waited: 98, behind: 90, behindOn: "syncing 40, laying out 30, agents changing 2,"
                + " 18 unnamed" });
        // The shell's thread is said syncing from its frame's animating,
        // after its polish, to the sync letting it go, which holds its wait
        // for a render thread still drawing the frame before. The render
        // thread's own times say which, as "sta" said syncing 68 where its
        // frame's sync took 1 (Joel, 2026-11-18).
        const sta = [{ name: "syncing", from: 1338, to: 1406 }];
        const swapHeld = [{ frame: 7, synchronizing: 1328, synchronized: 1330, rendering: 1330,
            rendered: 1336, swapped: 1395 }, { frame: 8, synchronizing: 1398, synchronized: 1400,
            rendering: 1400, rendered: 1404, swapped: 1405 }];
        Expect.same("a sync behind the last frame's swap says it waited on the swap",
            Awaited.syncsSplit(sta, swapHeld), [
                { name: "waiting on the last swap", from: 1338, to: 1395 },
                { name: "before the sync", from: 1395, to: 1398 },
                { name: "the sync itself", from: 1398, to: 1400 },
                { name: "after the sync", from: 1400, to: 1406 }]);
        const renderHeld = [{ name: "syncing", from: 1338, to: 1380 }];
        Expect.same("a sync behind the last frame's render says it waited on the render",
            Awaited.syncsSplit(renderHeld, [
                { frame: 7, synchronizing: 1330, synchronized: 1335, rendering: 1335,
                    rendered: 1360, swapped: 1362 },
                { frame: 8, synchronizing: 1370, synchronized: 1372, rendering: 1372,
                    rendered: 1376, swapped: 1378 }]), [
                { name: "waiting on the last render", from: 1338, to: 1360 },
                { name: "waiting on the last swap", from: 1360, to: 1362 },
                { name: "before the sync", from: 1362, to: 1370 },
                { name: "the sync itself", from: 1370, to: 1372 },
                { name: "after the sync", from: 1372, to: 1380 }]);
        // On a loop rendering on the shell's thread, as in a sandbox, the
        // frame's own render and swap run inside the span, after its sync,
        // so they fall past the wait and are said with what came after.
        Expect.same("a frame's own render on the shell's thread, after its sync, is after it",
            Awaited.syncsSplit(renderHeld, [{ frame: 8, synchronizing: 1340,
                synchronized: 1342, rendering: 1342, rendered: 1360, swapped: 1375 }]), [
                { name: "before the sync", from: 1338, to: 1340 },
                { name: "the sync itself", from: 1340, to: 1342 },
                { name: "after the sync", from: 1342, to: 1380 }]);
        // The patch sets both sync times at once, so a sync with no end
        // is only a read gone wrong; it still doesn't take the rest.
        Expect.same("a sync with no end, which the patch never gives, takes none",
            Awaited.syncsSplit(renderHeld, [{ frame: 8, synchronizing: 1370,
                rendering: 1372, rendered: 1376, swapped: 1378 }]), [
                { name: "before the sync", from: 1338, to: 1370 },
                { name: "after the sync", from: 1370, to: 1380 }]);
        // A held key's span is cut at its bound, which can come before the
        // render thread says the sync ended.
        const pastSpan = [{ frame: 8, synchronizing: 1370, synchronized: 1390 }];
        Expect.same("a sync ending past its span is said only to the span's end",
            Awaited.syncsSplit(renderHeld, pastSpan), [
                { name: "before the sync", from: 1338, to: 1370 },
                { name: "the sync itself", from: 1370, to: 1380 }]);
        const atOpen = [{ frame: 8, synchronizing: 1338, synchronized: 1340 }];
        Expect.same("a sync taken as its span opens waited on nothing",
            Awaited.syncsSplit(renderHeld, atOpen), [
                { name: "the sync itself", from: 1338, to: 1340 },
                { name: "after the sync", from: 1340, to: 1380 }]);
        // A render wholly inside another's is said once, with the longer.
        const inside = [{ frame: 6, rendering: 1340, rendered: 1390, swapped: 1392 },
            { frame: 7, rendering: 1350, rendered: 1355, swapped: 1360 }, swapHeld[1]];
        Expect.same("a step wholly inside one already said isn't said again",
            Awaited.syncsSplit(sta, inside), [
                { name: "before the sync", from: 1338, to: 1340 },
                { name: "waiting on the last render", from: 1340, to: 1390 },
                { name: "waiting on the last swap", from: 1390, to: 1392 },
                { name: "before the sync", from: 1392, to: 1398 },
                { name: "the sync itself", from: 1398, to: 1400 },
                { name: "after the sync", from: 1400, to: 1406 }]);
        // A frame its window wasn't ready to render, or whose slot a newer
        // frame took, comes back with its number alone, so a sync whose own
        // frame has no times says what's left untimed.
        Expect.same("a sync whose own frame isn't timed says what nothing told apart",
            Awaited.syncsSplit(sta, [swapHeld[0], { frame: 8 }]), [
                { name: "waiting on the last swap", from: 1338, to: 1395 },
                { name: "untimed syncing", from: 1395, to: 1406 }]);
        // One render thread draws a frame at a time, so steps never cross;
        // given newest first, with a frame gone, one begun after the span
        // and steps that cross the span's start and each other anyway, each
        // millisecond is still said once, in order.
        const jumbled = [swapHeld[1], { frame: 10, synchronizing: 1410, synchronized: 1411,
            rendering: 1412, rendered: 1414, swapped: 1415 }, { frame: 9 },
            { frame: 6, synchronizing: 1290, synchronized: 1300, rendering: 1300,
                rendered: 1340, swapped: 1342 },
            { frame: 7, synchronizing: 1320, synchronized: 1322, rendering: 1341,
                rendered: 1390, swapped: 1395 }];
        Expect.same("frames out of order, gone or crossing are each said once, in order",
            Awaited.syncsSplit(sta, jumbled), [
                { name: "waiting on the last render", from: 1338, to: 1340 },
                { name: "waiting on the last swap", from: 1340, to: 1342 },
                { name: "waiting on the last render", from: 1342, to: 1390 },
                { name: "waiting on the last swap", from: 1390, to: 1395 },
                { name: "before the sync", from: 1395, to: 1398 },
                { name: "the sync itself", from: 1398, to: 1400 },
                { name: "after the sync", from: 1400, to: 1406 }]);
        const tiles = (span, parts) => parts.length > 0 && parts[0].from === span.from
            && parts[parts.length - 1].to === span.to
            && parts.every((part, at) => part.to > part.from
                && (at === 0 || part.from === parts[at - 1].to));
        Expect.same("a sync's parts cover it, end to end, each once",
            [Awaited.syncsSplit(sta, swapHeld), Awaited.syncsSplit(sta, jumbled),
                Awaited.syncsSplit(sta, [swapHeld[0]]), Awaited.syncsSplit(sta, [{ frame: 8 }]),
                Awaited.syncsSplit(sta, inside)]
                .map(parts => tiles(sta[0], parts)), [true, true, true, true, true]);
        Expect.same("a span cut before its sync ends is still covered once",
            [Awaited.syncsSplit(renderHeld, pastSpan), Awaited.syncsSplit(renderHeld, atOpen)]
                .map(parts => tiles(renderHeld[0], parts)), [true, true]);
        const others = [{ name: "laying out", from: 1300, to: 1320 }];
        Expect.same("work other than a sync is said as it was",
            Awaited.syncsSplit(others, swapHeld), others);
        Expect.same("a held key's line names the render thread's steps it waited behind",
            Awaited.behindSplit({ held: { from: 1330, timedOut: true, collected: 50 },
                listed: { at: 1407, spans: sta, collected: 50 } }, swapHeld, 8),
            "waiting on the last swap 57, after the sync 6, before the sync 3,"
                + " the sync itself 2, 1 unnamed");

        // FramePresentation keeps the last sixteen frames' times, none for
        // the newest while it's begun and not yet rendered, and none for a
        // frame whose slot a newer one took, whatever older frame answers.
        const kept = frame => frame === 2 || frame >= 6 && frame <= 19
            ? { rendered: frame * 10 } : {};
        Expect.same("the frames read are those still timed, oldest first, to the first gone",
            Awaited.framesTimed(20, kept).map(frame => frame.frame),
            [6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19]);
        Expect.same("only the newest untimed frame is passed over; the next ends the read",
            Awaited.framesTimed(21, kept), []);
        Expect.same("a read whose every frame answers ends at the ring's sixteen",
            Awaited.framesTimed(1000, frame => ({ rendered: frame })).length, 16);

        // A key typed before an earlier held key's frame is swapped keeps
        // the earlier one's wait for it.
        const first = { lists: 1 };
        const second = { lists: 2 };
        const both = Awaited.heldKept(Awaited.heldKept([], first, { held: 1 }), second,
            { held: 2 });
        Expect.same("a held key's wait is found by its own parts, and let go",
            [Awaited.heldDrawn(both, first).behind.held, Awaited.heldDrawn(both, first).kept
                .map(one => one.held)], [1, [2]]);
        Expect.same("a held key's wait is found behind an older one still kept",
            [Awaited.heldDrawn(both, second).behind.held, Awaited.heldDrawn(both, second).kept
                .map(one => one.held)], [2, [1]]);
        Expect.same("a key with no wait kept finds none, and keeps the rest",
            [Awaited.heldDrawn(both, { lists: 1 }).behind,
                Awaited.heldDrawn(both, { lists: 1 }).kept.length], [null, 2]);
        let many = [];
        for (let key = 0; key < 10; key++)
            many = Awaited.heldKept(many, { key }, { held: key });
        Expect.same("only the newest eight held keys' waits are kept",
            many.map(one => one.held), [2, 3, 4, 5, 6, 7, 8, 9]);

        Expect.same("a key its answers ended was behind nothing, whatever ran",
            Awaited.heldFor({ began: 1712000000000, from: 1000, collected: 50 },
                { at: 1030, spans: noted, collected: 60 }, 8), { waited: 30, behind: 0,
                behindOn: "" });
    }
}
