// A stand-in People source that keeps a page for each person it knows,
// as the machine's own repository's sources may (Vault's), for
// tests/people.qml: it knows Ada by her address, and keeps a page for
// her alone, so People lists Open in Pages
// for her, and is still finding out for Cal, so his stands dimmed.
// It has had things with Ada and Ben too, as Google's and Slack's
// sources have, so their pages list what's between them: Ada has mail
// waiting on Joel, two meetings and two messages; Ben has nothing
// waiting, two meetings, the newest the two of them, and one message.
import qs
import QtQuick

PersonSource {
    called: "Pages"
    people: [{
        emails: ["ada@example.com"]
    }]

    // A day back is said "yesterday" whatever the hour, and ten days by
    // its date, which no case reads.
    readonly property real day: 86_400_000

    function page(person) {
        return person.emails.includes("ada@example.com") ? "https://pages.example.com/ada" : "";
    }

    function looking(person) {
        return person.emails.includes("cal@example.com");
    }

    function had(provider, key, name) {
        return provider.thing({ key: key, name: name, run: () => {} });
    }

    // A minute and a half back, read as a minute ago: People takes its now
    // before it asks here, so a mail dated a whole minute back read as
    // "just now" whenever a millisecond passed between (H6EDLC, 2026-11-12).
    readonly property real lunch: 90_000

    function unread(person, provider) {
        const now = Date.now();
        return person.emails.includes("ada@example.com")
            ? [{ at: now - lunch, row: had(provider, "mail:lunch", "Lunch on Friday"),
                what: "mail" }] : [];
    }

    function recent(person, provider) {
        const now = Date.now();
        if (person.emails.includes("ada@example.com"))
            return [
                { at: now - day, row: had(provider, "meeting:retro", "Retro"), kind: "met" },
                { at: now - 10 * day, row: had(provider, "meeting:ada", "Ada / Joel"),
                    kind: "met", one: true },
                { at: now - 5 * 60_000, row: had(provider, "slack:ada", "See you then"),
                    kind: "wrote", mine: true },
                { at: now - day, row: had(provider, "mail:trip", "Trip Notes"), kind: "wrote" },
                // What waits is never listed again under Messages.
                { at: now - lunch, row: had(provider, "mail:lunch", "Lunch on Friday"),
                    kind: "wrote" }
            ];
        if (person.emails.includes("ben@example.com"))
            return [
                { at: now - day, row: had(provider, "meeting:ben", "Ben / Joel"), kind: "met",
                    one: true },
                { at: now - 10 * day, row: had(provider, "meeting:planning", "Planning"),
                    kind: "met" },
                { at: now - day, row: had(provider, "slack:ben", "Thanks!"), kind: "wrote" }
            ];
        return [];
    }

    // The first of each for Ada's drawer, as her page lists them.
    function firsts(person, provider) {
        const now = Date.now();
        return person.emails.includes("ada@example.com") ? {
            met: { at: now - day, name: "Retro" },
            wait: { at: now - lunch, name: "Lunch on Friday", what: "mail" }
        } : { met: null, wait: null };
    }
}
