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

    // The words, as words.json's for-you:kinds and for-you:sources say them.
    readonly property var said: ({
            kinds: { do: "To Do", look: "To Look At", try: "To Try" },
            sources: {
                mail: "Mail",
                slack: "Slack",
                "needs-you": "Needs You",
                calendar: "Calendar"
            }
        })

    // For You's items, "key:kind:source" each, in the order it lists them.
    readonly property var items: [
        "a:do:needs-you", "b:look:mail", "c:look:slack", "d:try:sessions", "e:look:mail",
        "f:notice:slack", "g:notice:mail", "h:notice:desktop", "i:notice:home-network"
    ].map(one => {
        const [key, kind, source] = one.split(":");
        return { key, kind, source, title: key, at: 0, weight: 0 };
    })

    function keys(listed) {
        return listed.map(item => item.key).join(" ");
    }

    function offers(listed) {
        return listed.map(one => `${one.value} ${one.note}`).join(", ");
    }

    function check(failures, what, got, want) {
        if (got !== want)
            failures.push(`${what}: got ${got}, wanted ${want}`);
    }

    function run() {
        const failures = [];
        // [what, what :filter was given, the items kept, what the head says]
        const filters = [
            ["a kind keeps its band", "look", "b c e", "To Look At"],
            ["a source keeps both bands, notices too", "slack", "c f", "Slack"],
            ["two words narrow to both", "mail look", "b e", "Mail · To Look At"],
            ["in either order, the source said first", "look mail", "b e", "Mail · To Look At"],
            ["typed in capitals", "Mail LOOK", "b e", "Mail · To Look At"],
            ["a source the words don't know, by its id", "sessions", "d", "Sessions"],
            ["and an id of parts, each begun in capitals", "home-network", "i", "Home Network"],
            ["a source the words know, with nothing now, is an empty step", "calendar", "",
                "Calendar"],
            ["as are two words that keep nothing", "slack do", "", "Slack · To Do"],
            ["nothing given keeps everything", "", "a b c d e f g h i", ""]
        ];
        for (const [what, arg, kept, head] of filters) {
            const filter = Filter.read(arg, items, said);
            check(failures, `${what} (error)`, filter.error ?? "", "");
            check(failures, `${what} (kept)`, keys(Filter.filtered(items, filter)), kept);
            check(failures, `${what} (head)`, Filter.heading(filter, said), head);
        }

        // [what, what :filter was given, Vim's error]
        const refused = [
            ["a word that's neither", "mial", "E475: Invalid argument: mial"],
            ["a notice is no kind to filter by", "notice", "E475: Invalid argument: notice"],
            ["a second kind", "look do", "E475: Invalid argument: do"],
            ["a second source", "mail slack", "E475: Invalid argument: slack"]
        ];
        for (const [what, arg, error] of refused)
            check(failures, what, Filter.read(arg, items, said).error ?? "", error);

        // Only sources present are offered, so never the calendar here.
        const everything = "do 1, look 3, try 1, desktop 1, home-network 1, mail 3, needs-you 1, "
            + "sessions 1, slack 2";
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

        // Two sources alike by name keep the order they came, and A to Z
        // reads numbers as numbers, so a tenth session follows the ninth.
        const sessions = ["s10", "s9"].map(source => ({ key: source, kind: "do", source }));
        check(failures, "sources by name, numbers as numbers",
            offers(Filter.offered(sessions, "do ", said)), "s9 1, s10 1");

        for (const failure of failures)
            console.warn(failure);
        console.log(`for-you-filter: ${failures.length} of the cases failed`);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}
