// One person per email address, however many sources know them: who
// they are, how they stand, who's listed and in what order, what the
// drawer says, and the order of their acts, run in Qt's own JavaScript
// engine, the one the shell runs it in. The
// `the-people-are-one-person-each` check copies Person.js beside this file
// and runs `qml -platform offscreen person.qml`.
import QtQuick
import "Person.js" as Person

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

    function run() {
        const failures = [];
        let asked = 0;
        const check = (what, said, meant) => {
            asked += 1;
            if (JSON.stringify(said) !== JSON.stringify(meant))
                failures.push(
                    `${what}: said ${JSON.stringify(said)}, meant ${JSON.stringify(meant)}`);
        };
        const names = persons => persons.map(person => person.name);

        // Tuple knows Ada by both her addresses; Google has mailed her at
        // one, in capitals, and met Grace, whom Tuple doesn't know.
        const tuple = { name: "tuple", people: [
            { emails: ["ada@example.com", "ada@home.example"], name: "Ada Lovelace" },
            { emails: ["ben@example.com"], name: "" }
        ] };
        const google = { name: "google", people: [
            { emails: ["ADA@home.example"], name: "Ada L.", recent: true,
                facts: [{ label: "Last Mail", value: "Lunch on Friday" }] },
            { emails: ["grace@example.com"], name: "grace@example.com", recent: true },
            { emails: [], name: "Nobody" }
        ] };
        const everyone = Person.merged([tuple, google]);
        check("one person each, by any address", names(everyone),
            ["Ada Lovelace", "ben@example.com", "grace@example.com"]);
        check("every address, the first first", everyone[0].emails,
            ["ada@example.com", "ada@home.example"]);
        check("the sources that know them, in order", everyone[0].from, ["tuple", "google"]);
        check("each source's facts", everyone[0].facts,
            [{ label: "Last Mail", value: "Lunch on Friday" }]);
        check("dealt with lately, by any source", everyone.map(person => person.recent),
            [true, false, true]);
        check("a real name wins over an address", names(Person.merged([google, tuple]))[0],
            "Ada L.");

        // A source that learns two are one joins them.
        const joining = Person.merged([
            { name: "google", people: [{ emails: ["a@x"], name: "A" },
                { emails: ["b@x"], name: "B" }] },
            { name: "vault", people: [{ emails: ["b@x", "a@x"], name: "Alex",
                facts: [{ label: "Team", value: "Shell" }] }] }
        ]);
        check("two found to be one",
            joining.map(person => [person.name, person.emails, person.from]),
            [["A", ["a@x", "b@x"], ["google", "vault"]]]);
        check("their facts kept", joining[0].facts, [{ label: "Team", value: "Shell" }]);

        // How they stand: the first source that says.
        const stands = [{ "ben@example.com": { said: "in a call", order: 1, favourite: true } },
            { "ben@example.com": { said: "away", order: 2 } }];
        const ben = everyone[1];
        check("what most limits reaching them", Person.standing(ben, stands),
            { said: "in a call", order: 1, favourite: true, recent: false });
        // Slack, asked first, against Tuple (VKCF3Z, e633d330).
        const slackThen = (slack, tuple) => Person.standing(ben,
            [{ "ben@example.com": slack }, { "ben@example.com": tuple }]);
        const active = { said: "active", order: 0, mark: "\u{1f334}" };
        check("a call beats active, with Slack's mark",
            slackThen(active, { said: "in a call", order: 1 }).said, "in a call \u{1f334}");
        check("a huddle beats available",
            slackThen({ said: "in a huddle", order: 1 }, { said: "available", order: 0 }).said,
            "in a huddle");
        check("away beats available",
            slackThen({ said: "away", order: 2 }, { said: "available", order: 0 }).said, "away");
        check("a tie goes to the source asked first",
            slackThen(active, { said: "available", order: 0 }).said, "active \u{1f334}");
        check("active beats a program closed",
            slackThen(active, { said: "offline", order: 3 }).said, "active \u{1f334}");
        check("a favourite from any source",
            slackThen(active, { said: "available", order: 0, favourite: true }).favourite, true);
        check("a mark alone follows the other's words",
            slackThen({ said: "", order: 4, mark: "\u{1f334}" }, { said: "away", order: 2 }),
            { said: "away \u{1f334}", order: 2, favourite: false, recent: false });
        check("nothing said", Person.standing(everyone[2], stands),
            { said: "", order: 4, favourite: false, recent: false });
        check("listed: a favourite", Person.listed(ben, Person.standing(ben, stands), 0), true);
        check("listed: dealt with lately",
            Person.listed(everyone[2], Person.standing(everyone[2], []), 0), true);
        check("not listed: only known", Person.listed(ben, Person.standing(ben, []), 0), false);

        // In order: favourites, then by name, however they stand, since that
        // changes while the list is open.
        const standOf = person => Person.standing(person, [{
            "grace@example.com": { said: "available", order: 0 },
            "ben@example.com": { said: "away", order: 2, favourite: true }
        }]);
        check("favourites, then by name", names(Person.ordered(everyone, standOf, 0)),
            ["ben@example.com", "Ada Lovelace", "grace@example.com"]);

        // By the last thing between them and Joel, newest first, by what
        // every source says: A is in a meeting with him now, B's mail came
        // a minute ago, and C is only asked to a meeting to come, so isn't
        // listed but is ordered last.
        const minute = 60 * 1000;
        const now = 100 * minute;
        const coming = { from: now + 9 * minute, to: now + 39 * minute };
        const lately = Person.merged([
            { name: "google", people: [
                { emails: ["b@x"], name: "B", spans: [{ from: now - minute, to: now - minute }] },
                { emails: ["a@x"], name: "A", spans: [coming] },
                { emails: ["c@x"], name: "C", spans: [coming] }] },
            { name: "vault", people: [{ emails: ["a@x"], name: "A",
                spans: [{ from: now - minute, to: now + minute }, { from: 0, to: NaN }] }] }
        ]);
        check("when everything with them is, by every source", lately[1].spans,
            [coming, { from: now - minute, to: now + minute }]);
        check("the last thing with each", lately.map(person => Person.last(person, now)),
            [now - minute, now, null]);
        check("someone only asked to what's to come isn't listed",
            lately.map(person => Person.listed(person, Person.standing(person, []), now)),
            [true, true, false]);
        check("the last first", names(Person.ordered(lately,
            person => Person.standing(person, []), now)), ["A", "B", "C"]);
        const cFavourite = [{ "c@x": { said: "", order: 4, favourite: true } }];
        check("a favourite before the last", names(Person.ordered(lately,
            person => Person.standing(person, cFavourite), now)), ["C", "A", "B"]);

        // How each stands is read once each as they're ordered, not at each
        // of the sort's comparisons: read there, ordering the 500 Joel lists
        // took most of every opening of the launcher (2026-11-10).
        const crowd = Person.merged([{ name: "crowd", people: Array.from({ length: 200 },
            (_, at) => ({ emails: [`p${(at * 37) % 200}@x`], name: `P${(at * 37) % 200}` })) }]);
        let stood = 0;
        const counted = Person.ordered(crowd, person => {
            stood += 1;
            return Person.standing(person, []);
        }, now);
        check("how each stands, read once each", [stood, counted.length], [200, 200]);
        check("and still in order", names(counted).slice(0, 3), ["P0", "P1", "P2"]);
        // So the order is kept until the clock alone changes it: A's meeting
        // ending, then the one to come starting.
        check("the order changes as a meeting ends", Person.changesAt(lately, now),
            now + minute);
        check("then as one starts", Person.changesAt(lately, now + minute), now + 9 * minute);
        check("and never with nothing to come", Person.changesAt(crowd, now) === Infinity, true);

        // The drawer.
        check("their details", Person.details(everyone[0]), {
            title: "Ada Lovelace",
            facts: [{ label: "Emails", value: "ada@example.com, ada@home.example" },
                { label: "Last Mail", value: "Lunch on Friday" }],
            body: ""
        });
        check("one address", Person.details(everyone[2]).facts[0],
            { label: "Email", value: "grace@example.com" });

        // Someone a source knows by no address, only by an id of its own,
        // is a person of its alone; someone known by neither isn't anyone.
        const alone = Person.merged([{ name: "tuple", people: [
            { emails: [], id: 108, name: "Hal" }, { emails: [], name: "Nobody" }] }]);
        check("one known by an id alone", alone.map(person => [person.key, person.name,
            person.emails]), [["tuple:108", "Hal", []]]);
        check("and how they stand, by it", Person.standing(alone[0],
            [{ "tuple:108": { said: "away", order: 2, favourite: true } }]).said, "away");
        check("with no address in their details", Person.details(alone[0]).facts, []);
        check("one with an address is known by it", everyone[2].key, "grace@example.com");

        // Acts: the main ways to reach them first, in source order, then the rest.
        check("acts in the one order", Person.acts([
            [{ name: "Write in Slack", main: true }],
            [{ name: "Call Them", main: true }, { name: "Make Them a Favourite" }],
            [{ name: "Write a Mail", main: true }, { name: "See Their Meetings" }]
        ]).map(act => act.name),
            ["Write in Slack", "Call Them", "Write a Mail", "Make Them a Favourite",
                "See Their Meetings"]);

        // What's between them, sorted by what it is rather than told as one
        // Recent; `listed` stands in for Recent.js's, newest first, leaving
        // out what's shown.
        const listed = (sources, shown) => [].concat(...sources)
            .filter(one => !shown.includes(one.key)).sort((a, b) => b.at - a.at);
        const keys = entries => entries.map(one => one.key);
        const between = Person.between([
            [{ key: "retro", at: 3, kind: "met" }, { key: "lunch", at: 5, kind: "wrote" }],
            [{ key: "dm", at: 4, kind: "wrote" }, { key: "call", at: 6, kind: "met" },
                { key: "old", at: 1 }]
        ], [[{ key: "lunch", at: 5 }]], listed);
        check("what waits", keys(between.waiting), ["lunch"]);
        check("meetings and calls, newest first", keys(between.met), ["call", "retro"]);
        check("what was written, what waits left out, one of no kind written",
            keys(between.messages), ["dm", "old"]);
        const when = at => `at ${at}`;
        const words = { one: "{when} \u00b7 1:1", theirs: "they wrote \u00b7 {when}",
            mine: "you wrote \u00b7 {when}" };
        check("when they last met", Person.metSide({ at: 3 }, when, words), "at 3");
        check("and that it was the two of them", Person.metSide({ at: 6, one: true }, when,
            words), "at 6 \u00b7 1:1");
        check("who wrote last", [Person.wroteSide({ at: 4 }, when, words),
            Person.wroteSide({ at: 4, mine: true }, when, words)],
            ["they wrote \u00b7 at 4", "you wrote \u00b7 at 4"]);
        const waits = { needs: "needs you \u00b7 {what} \u00b7 {when}",
            sent: "you wrote \u00b7 {what} \u00b7 {when}", mail: "mail" };
        check("what waits says it needs him, what it is and when", [
            Person.waitSide({ at: 2, what: "mail" }, when, waits),
            Person.waitSide({ at: 2, what: "Slack", mine: true }, when, waits)
        ], ["needs you \u00b7 mail \u00b7 at 2", "you wrote \u00b7 Slack \u00b7 at 2"]);
        check("alone, it says what its heading would have", [
            Person.aloneSide("at 3", { alone: "met \u00b7 {side}" }),
            Person.aloneSide("at 4", {})
        ], ["met \u00b7 at 3", "at 4"]);
        const holding = entries => ({ name: "Held", count: entries.length });
        const standing = entry => ({ name: entry.row.name, side: "alone" });
        check("nothing stands for nothing", Person.placed([], holding, standing), []);
        check("one thing stands as itself",
            Person.placed([{ row: { name: "Lunch" } }], holding, standing),
            [{ name: "Lunch", side: "alone" }]);
        check("more step into them all",
            Person.placed([{ row: {} }, { row: {} }], holding, standing),
            [{ name: "Held", count: 2 }]);

        // Their time on their row, only where it's apart from Joel's
        // (VKCF3Z, 0cea8a93): where they are by their zone's last part.
        const away = "{place} \u00b7 {time}";
        check("someone elsewhere says where and the time there", [
            Person.awaySide("Europe/London", 300, "17:02", away),
            Person.awaySide("America/Argentina/Buenos_Aires", -60, "13:02", away)
        ], ["London \u00b7 17:02", "Buenos Aires \u00b7 13:02"]);
        const labels = { waiting: "Waiting", next: "Next Meeting", met: "Last Met" };
        const said = at => `at ${at}`;
        check("the first of each, the oldest wait and the newest met", Person.firsts([
            { met: { at: 5, name: "Retro" }, wait: { at: 7, name: "Lunch" } },
            null,
            { met: { at: 9, name: "Call" }, wait: { at: 3, name: "Thanks!" } }
        ], { name: "Standup", side: "14:00" }, said, labels), [
            { label: "Waiting", value: "Thanks! \u00b7 at 3" },
            { label: "Next Meeting", value: "Standup \u00b7 14:00" },
            { label: "Last Met", value: "Call \u00b7 at 9" }
        ]);
        check("none where there's nothing", Person.firsts([{ met: null, wait: null }], null,
            said, labels), []);
        check("someone in Joel's time says none",
            Person.awaySide("America/Toronto", 0, "12:02", away), "");

        if (failures.length > 0) {
            console.warn(`${failures.length} of ${asked} failed:\n${failures.join("\n")}`);
            Qt.exit(1);
        } else {
            console.log(`${asked} of ${asked} held`);
            Qt.exit(0);
        }
    }
}
