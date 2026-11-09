// What For You's :filter narrows to, run in Qt's own JavaScript engine, the
// one the shell runs it in. The `for-you-narrows-by-kind-and-source` check
// copies modules/quickshell/launcher/ForYouFilter.js and Order.js, which it
// imports, beside this file and runs `qml -platform offscreen
// for-you-filter.qml`. Each case is a promise VKCF3Z ruled (2026-11-08):
// a kind and a source, by their words, narrow to both; the head joins
// their names; and Tab offers only what's there, with how many.
import QtQuick
import "ForYouFilter.js" as Filter

QtObject {
    // A test that throws ends as a failure, rather than leaving qml running.
    Component.onCompleted: {
        try {
            run();
        } catch (error) {
            console.warn(`the test threw: ${error}`);
            Qt.exit(1);
        }
    }

    // The vocabulary's names, by id, as words.json names its for-you:kind
    // and for-you:source entries, beside one of another place's.
    readonly property var names: ({
            "for-you:kind:do": "To-Dos",
            "for-you:kind:look": "Looks",
            "for-you:kind:try": "To Try",
            "for-you:source:mail": "Mail",
            "for-you:source:slack": "Slack",
            "for-you:source:calendar": "Calendar",
            "mail": "Mail"
        })
    // The words a filter's head says, as Now.qml reads them.
    readonly property var said: Filter.saidFrom(Object.keys(names), id => names[id])

    // For You's rows, "key:kind:source" each, in the order it lists them,
    // each saying both under its forYou as Now.qml's do; then Later, which
    // is no item of For You's, and says neither. Each row's own kind is the
    // launcher's, never the filter's.
    readonly property var items: [
        "a:do:needs-you", "b:look:mail", "c:look:slack", "d:try:sessions", "e:look:mail",
        "f:notice:slack", "g:notice:mail", "h:notice:desktop", "i:notice:home-network",
        "j:look:looks"
    ].map(one => {
        const [key, kind, source] = one.split(":");
        return { key, kind: "thing", forYou: { kind, source } };
    }).concat([{ key: "later", kind: "thing" }])

    function keys(listed) {
        return listed.map(item => item.key).join(" ");
    }

    function offers(listed) {
        return listed.map(one => `${one.value} ${one.side}`).join(", ");
    }

    function check(failures, what, got, want) {
        if (got !== want)
            failures.push(`${what}: got ${got}, wanted ${want}`);
    }

    function run() {
        const failures = [];
        // [what, what :filter was given, the items kept, what the head says]
        const filters = [
            ["a kind keeps its band", "look", "b c e j", "Looks"],
            ["a source of one kind alone answers as its kind", "needs-you", "a", "To-Dos"],
            ["as the looks do", "looks", "b c e j", "Looks"],
            ["and so narrows beside a source", "looks mail", "b e", "Mail \u00b7 Looks"],
            ["the kind said twice is the one kind", "look looks", "b c e j", "Looks"],
            ["a source keeps both bands, notices too", "slack", "c f", "Slack"],
            ["two words narrow to both", "mail look", "b e", "Mail · Looks"],
            ["in either order, the source said first", "look mail", "b e", "Mail · Looks"],
            ["typed in capitals", "Mail LOOK", "b e", "Mail · Looks"],
            ["a source the words don't know, by its id", "sessions", "d", "Sessions"],
            ["and an id of parts, each begun in capitals", "home-network", "i", "Home Network"],
            ["a source the words know, with nothing now, is an empty step", "calendar", "",
                "Calendar"],
            ["as are two words that keep nothing", "slack do", "", "Slack · To-Dos"],
        ];
        for (const [what, arg, kept, head] of filters) {
            const filter = Filter.asked(arg, items, said);
            check(failures, `${what} (error)`, filter.error ?? "", "");
            check(failures, `${what} (kept)`, keys(Filter.filtered(items, filter)), kept);
            check(failures, `${what} (head)`, Filter.heading(filter, said), head);
        }

        // Inside a filter, :filter narrows it further, so `-` still comes
        // back out to everything in one step.
        const looking = Filter.asked("look", items, said);
        const narrowed = Filter.asked("mail", items, said, looking);
        check(failures, "a filter inside a filter narrows to both",
            `${keys(Filter.filtered(items, narrowed))} / ${Filter.heading(narrowed, said)}`,
            "b e / Mail \u00b7 Looks");
        check(failures, "and its key is the filter's, whichever way it was reached",
            Filter.keyOf(narrowed), Filter.keyOf(Filter.asked("look mail", items, said)));
        check(failures, "a second kind inside a filter is refused",
            Filter.asked("do", items, said, looking).error ?? "", "E475: Invalid argument: do");
        check(failures, "Tab inside a filter offers only what narrows it",
            offers(Filter.offered(items, "", said, looking)), "mail 2, slack 1");

        // [what, what :filter was given, Vim's error]
        const refused = [
            ["a word that's neither", "mial", "E475: Invalid argument: mial"],
            ["a notice is no kind to filter by", "notice", "E475: Invalid argument: notice"],
            ["a second kind", "look do", "E475: Invalid argument: do"],
            ["a second source", "mail slack", "E475: Invalid argument: slack"],
            ["nothing at all, since - goes back to everything", " ", "E471: Argument required"]
        ];
        for (const [what, arg, error] of refused)
            check(failures, what, Filter.asked(arg, items, said).error ?? "", error);

        // Only sources present are offered, so never the calendar here.
        // Nor a source of one kind alone, which its kind already reaches.
        const everything = "do 1, look 4, try 1, desktop 1, home-network 1, mail 3, sessions 1, "
            + "slack 2";
        // [what, what's typed after `:filter`, what Tab offers]
        const tabs = [
            ["kinds in their bands' order, then sources A to Z by name, each with its count",
                "", everything],
            ["a word being typed is offered among the rest", "ma", everything],
            ["after a kind, only the sources it keeps", "look ", "mail 2, slack 1"],
            ["after a source, only the kinds it holds", "mail ", "look 2"],
            ["after both, nothing", "mail look ", ""],
            ["after a word that's neither, nothing", "mial ", ""]
        ];
        for (const [what, arg, offered] of tabs)
            check(failures, what, offers(Filter.offered(items, arg, said)), offered);
        check(failures, "each offer is named as the head names it, with the field it fills",
            Filter.offered(items, "", said).map(one => `${one.value}:${one.field}:${one.note}`)
                .join(", "), "do:kind:To-Dos, look:kind:Looks, try:kind:To Try, "
                + "desktop:source:Desktop, home-network:source:Home Network, "
                + "mail:source:Mail, sessions:source:Sessions, slack:source:Slack");
        // Every offer wears a mark: a kind its own, a source its own or its
        // program's, and one the table doesn't know For You's tray, never the
        // to-do's.
        check(failures, "each offer wears its kind's, its source's or its program's mark",
            Filter.offered(items, "", said).map(one => {
                const mark = Filter.markOf(one);
                return `${one.value} ${mark.glyph ?? `program:${mark.program}`}`;
            }).join(", "), "do waitsOnJoel, look look, try toTry, desktop desktop, "
                + "home-network forYou, mail mail, sessions session, slack program:slack");

        // Two sources alike by name keep the order they came, and A to Z
        // reads numbers as numbers, so a tenth session follows the ninth.
        const sessions = ["s10", "s9"].map(source => ({ key: source,
                forYou: { kind: "do", source } }));
        check(failures, "sources by name, numbers as numbers",
            offers(Filter.offered(sessions, "do ", said)), "s9 1, s10 1");

        for (const failure of failures)
            console.warn(failure);
        console.log(`for-you-filter: ${failures.length} of the cases failed`);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}
