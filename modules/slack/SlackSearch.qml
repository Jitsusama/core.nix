// Search Slack, the act that ends every search while Slack is signed in
// (VKCF3Z, d7c0f933): "Search Slack for ‹words›", last of all (the
// launcher's `last`). Nothing is asked as Joel types, since Slack allows
// a grid about twenty searches a minute; Return steps in and asks Slack's
// daemon once (src/daemon.rs), a dimmed stand-in holding the place while
// it's asked, as People's do (#1367), and the messages found light there.
// Each is a Slack message's one row, as everywhere (SlackMessage.qml):
// named by who said it, its first line after its name, or from the first
// word matched where the match is past it, its side when; Return opens it in
// Slack, and inside are Open in Slack, its drawer the messages around it,
// then Copy the Link. Slack too busy says so over Try Again; Slack's
// daemon gone says it isn't answering. Only ever read.
import qs
import Quickshell
import QtQuick

Provider {
    id: provider

    what: "slack message"

    Daemon {
        id: slack

        socket: `${Quickshell.env("XDG_RUNTIME_DIR") ?? ""}/daemons/slack-unreads.sock`
        kind: "slack-message"
    }

    // Offered only while the daemon answers and Slack is signed in, since
    // a row is never listed that can't do what it says; what the daemon
    // last said of Slack is kept through a gap, so both are read.
    readonly property bool signedIn: slack.state === "answering"
        && slack.sources?.slack === "answering"

    // What each search found, by its words: { state: "asking" | "found" |
    // "failed", records, code }, kept while the shell runs, so stepping
    // out and in again asks nothing.
    property var searches: ({})
    // The messages around each, by key, once asked: a list, or null while
    // asked.
    property var near: ({})

    // The row that ends a top-level search, read from what's held alone,
    // since the launcher asks it every time the list is made.
    function last(typed) {
        const words = typed.trim();
        if (words === "" || !signedIn)
            return null;
        const key = "verb:slack:search";
        const inside = () => provider.found(words);
        // Return steps in by itself, having no act of its own, so the step
        // holds the very row the launcher gave, knowing who ended the
        // search with it; a `run` stepping in would hold one that doesn't,
        // and the step wouldn't be made again once Slack answers.
        return provider.thing({
            key: `${key}:${words.toLowerCase()}`,
            name: Words.name(key, "", { words: words }),
            glyph: Words.glyph(key),
            hue: "action",
            side: "",
            note: "",
            verb: "more",
            // What's found inside are things with acts of their own.
            nested: true,
            verbs: inside,
            targets: inside
        });
    }

    // A Slack message's words beside a row, and the sentences it tells in
    // a row's place.
    function said(word) {
        return Words.of("slack:message").said[word] ?? "";
    }

    function tells(word) {
        return Words.of("slack:message").tells[word] ?? "";
    }

    // Joel, as he leads a note or a line: "You".
    function you() {
        const word = said("you");
        return word.charAt(0).toUpperCase() + word.slice(1);
    }

    // What a search holds: asked the first time it's stepped into.
    function found(words) {
        const search = searches[words];
        if (search === undefined) {
            seek(words);
            return [waiting("searching")];
        }
        if (search.state === "asking")
            return [waiting("searching")];
        if (search.state === "failed")
            return search.code === "too-many"
                ? [waiting("busy"), again(words)]
                : [waiting(search.code === "down" ? "down" : "failed"), again(words)];
        // Nothing found is said where the first would be, as a sentence,
        // with no act under it, since searching again is just typing; an
        // empty list would say it only once the placeholder, held dimmed
        // until a key (Steady.js), had gone.
        if (search.records.length === 0)
            return [waiting("none", words)];
        // The first lights where the placeholder stood, as a page found
        // does (VKCF3Z, c3240f08), taking over its place while keeping its
        // own key, so it's the one message For You lists; the rest follow.
        return search.records.map((record, at) => SlackMessage.row(record, provider.thing, {
            takesOver: at === 0 ? first : undefined,
            around: one => provider.around(one)
        }));
    }

    // The key the placeholder holds, whose place the first message found
    // takes.
    readonly property string first: "slack:search:first"

    // Asks Slack's daemon once for these words; Provider's own `ask` is
    // for URLs.
    function seek(words) {
        provider.keep(words, { state: "asking", records: [], code: "" });
        slack.ask({ ask: "search", words: words }, answer => {
            if (answer === null)
                provider.keep(words, { state: "failed", records: [], code: "down" });
            else if (answer.error)
                provider.keep(words, { state: "failed", records: [], code: answer.error.code });
            else
                provider.keep(words, { state: "found", code: "",
                    // Where each matched, worked out once as the answer
                    // comes rather than each time the list is made.
                    records: (answer.records ?? []).map(record => Object.assign({}, record, {
                        matched: SlackMessage.noted(record, words)
                    })) });
        });
    }

    function keep(words, search) {
        searches = Object.assign({}, searches, { [words]: search });
        provider.changed();
    }

    // A dimmed row holding the first message's place: while Slack is asked
    // a Slack message "searching Slack", and once it couldn't answer, the
    // sentence saying why, over Try Again, or that nothing was found for
    // `words`. Return on it does nothing.
    function waiting(word, words) {
        const asking = word === "searching";
        return provider.thing({
            key: first,
            name: asking ? Words.name("slack:message")
                : tells(word).replace("{words}", words ?? ""),
            glyph: Words.glyph("slack:message"),
            hue: "program",
            side: asking ? said(word) : "",
            gone: true,
            run: () => {}
        });
    }

    function again(words) {
        const key = "verb:slack:search:again";
        return provider.thing({
            key: key,
            name: Words.name(key),
            verb: Words.verb(key),
            glyph: Words.glyph(key),
            hue: "action",
            stays: true,
            // Asked again from here, the selection stays on it.
            run: () => provider.seek(words)
        });
    }

    // The messages around one, asked once it's stepped into: its drawer
    // says the message alone until they come.
    function around(record) {
        const lone = { title: record.place, facts: [], body: record.text };
        if (!(record.key in near)) {
            near = Object.assign({}, near, { [record.key]: null });
            slack.ask({ ask: "get", keys: [record.key] }, answer => {
                const messages = answer?.records?.[0]?.around ?? [];
                provider.near = Object.assign({}, provider.near, { [record.key]: messages });
                provider.changed();
            });
            return lone;
        }
        const messages = near[record.key];
        if (!messages || messages.length === 0)
            return lone;
        // Only Joel's own are named: Slack gives the others by an id, and
        // in a channel they're many.
        return {
            title: record.place,
            facts: [],
            body: messages.map(one => one.mine ? `${you()}: ${one.text}` : one.text).join("\n\n")
        };
    }
}
