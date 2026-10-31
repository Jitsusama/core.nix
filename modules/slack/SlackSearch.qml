// Search Slack, the act that ends every search while Slack is signed in
// (VKCF3Z, d7c0f933): "Search Slack for ‹words›", last of all (the
// launcher's `last`). Nothing is asked as Joel types, since Slack allows
// a grid about twenty searches a minute; Return steps in and asks Slack's
// daemon once (src/daemon.rs), a dimmed stand-in holding the place while
// it's asked, as People's do (#1367), and the messages found light there.
// Each is a Slack message: titled by its first words, beside it where it
// is, its note who said it and when; Return opens its conversation in
// Slack, and inside are Open in Slack, its drawer the messages around it,
// then Copy the Link. Slack too busy says so over Try Again; Slack's
// daemon gone says it isn't answering. Only ever read.
import qs
import Quickshell
import QtQuick
import "Ago.js" as Ago

Provider {
    id: provider

    what: "slack message"

    Daemon {
        id: slack

        socket: `${Quickshell.env("XDG_RUNTIME_DIR") ?? ""}/daemons/slack.sock`
        kind: "slack-message"
    }

    readonly property bool signedIn: slack.sources?.slack === "answering"

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
        const search = searches[words];
        const none = search?.state === "found" && search.records.length === 0;
        return provider.entering(provider.thing({
            key: `${key}:${words.toLowerCase()}`,
            name: Words.name(key, "", { words: words }),
            glyph: Words.glyph(key),
            hue: "action",
            side: "",
            note: "",
            // Nothing found is said in the step's place, as a sentence, with
            // no act under it, since searching again is just typing.
            empty: none ? said("none").replace("{words}", words) : ""
        }), () => provider.found(words));
    }

    function said(word) {
        return Words.of("slack:message").said[word] ?? "";
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
        // The first lights where the placeholder stood, by its key, as a
        // page found does (VKCF3Z, c3240f08); the rest follow it.
        return search.records.map((record, at) => provider.message(record,
            at === 0 ? first : record.key));
    }

    // The key the placeholder holds, and the first message found takes.
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
                provider.keep(words, { state: "found", records: answer.records ?? [], code: "" });
        });
    }

    function keep(words, search) {
        searches = Object.assign({}, searches, { [words]: search });
        provider.changed();
    }

    // A dimmed row holding the first message's place: while Slack is asked
    // a Slack message "searching Slack", and once it couldn't answer, the
    // sentence saying why, over Try Again. Return on it does nothing.
    function waiting(word) {
        const asking = word === "searching";
        return provider.thing({
            key: first,
            name: asking ? Words.name("slack:message") : said(word),
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

    // A message found, under `key`, opening its conversation in Slack.
    function message(record, key) {
        const open = () => Niri.spawn(["xdg-open",
            `slack://channel?team=${record.team}&id=${record.key.split(":")[1]}`]);
        const act = (key, fields) => provider.thing(Object.assign({
            key: key,
            name: Words.name(key),
            verb: Words.verb(key),
            glyph: Words.glyph(key),
            hue: "action"
        }, fields));
        const who = record.mine ? said("you") : record.who;
        return provider.thing({
            key: key,
            what: "slack message",
            thing: true,
            of: "program:slack",
            part: record.first,
            name: record.first,
            verb: Words.verb("verb:slack:message:open"),
            glyph: Words.glyph("slack:message"),
            hue: "program",
            side: record.place,
            note: `${who} \u00b7 ${Ago.ago(record.at, Date.now())}`,
            drawer: { title: record.place, facts: [], body: record.text },
            later: true,
            run: open,
            verbs: () => [
                act("verb:slack:message:open", {
                    later: true,
                    run: open,
                    drawer: provider.around(record)
                }),
                // Copy is last, as everywhere.
                act("verb:slack:message:link", {
                    now: true,
                    run: () => {
                        Quickshell.clipboardText = record.link;
                    }
                })
            ]
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
        const you = said("you");
        return {
            title: record.place,
            facts: [],
            body: messages.map(one => one.mine ? `${you}: ${one.text}` : one.text).join("\n\n")
        };
    }
}
