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
                failures.push(`${what}: said ${JSON.stringify(said)}, meant ${JSON.stringify(meant)}`);
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
            { name: "google", people: [{ emails: ["a@x"], name: "A" }, { emails: ["b@x"], name: "B" }] },
            { name: "vault", people: [{ emails: ["b@x", "a@x"], name: "Alex",
                facts: [{ label: "Team", value: "Shell" }] }] }
        ]);
        check("two found to be one", joining.map(person => [person.name, person.emails, person.from]),
            [["A", ["a@x", "b@x"], ["google", "vault"]]]);
        check("their facts kept", joining[0].facts, [{ label: "Team", value: "Shell" }]);

        // How they stand: the first source that says.
        const stands = [{ "ben@example.com": { said: "in a call", order: 1, favourite: true } },
            { "ben@example.com": { said: "away", order: 2 } }];
        const ben = everyone[1];
        check("the first source that says", Person.standing(ben, stands),
            { said: "in a call", order: 1, favourite: true, recent: false });
        check("nothing said", Person.standing(everyone[2], stands),
            { said: "", order: 4, favourite: false, recent: false });
        check("listed: a favourite", Person.listed(ben, Person.standing(ben, stands)), true);
        check("listed: dealt with lately", Person.listed(everyone[2], Person.standing(everyone[2], [])),
            true);
        check("not listed: only known", Person.listed(ben, Person.standing(ben, [])), false);

        // In order: favourites, then whoever can be reached, then by name.
        const standOf = person => Person.standing(person, [{
            "ada@example.com": { said: "available", order: 0 },
            "ben@example.com": { said: "away", order: 2, favourite: true }
        }]);
        check("favourites, then reachable, then by name", names(Person.ordered(everyone, standOf)),
            ["ben@example.com", "Ada Lovelace", "grace@example.com"]);

        // The drawer.
        check("their details", Person.details(everyone[0]), {
            title: "Ada Lovelace",
            facts: [{ label: "Emails", value: "ada@example.com, ada@home.example" },
                { label: "Last Mail", value: "Lunch on Friday" }],
            body: ""
        });
        check("one address", Person.details(everyone[2]).facts[0],
            { label: "Email", value: "grace@example.com" });

        // Acts: the main ways to reach them first, in source order, then the rest.
        check("acts in the one order", Person.acts([
            [{ name: "Call Them", main: true }, { name: "Make Them a Favourite" }],
            [{ name: "Write a Mail", main: true }, { name: "See Their Meetings" }],
            [{ name: "Open in Slack", main: true }]
        ]).map(act => act.name),
            ["Call Them", "Write a Mail", "Open in Slack", "Make Them a Favourite",
                "See Their Meetings"]);

        if (failures.length > 0) {
            console.warn(`${failures.length} of ${asked} failed:\n${failures.join("\n")}`);
            Qt.exit(1);
        } else {
            console.log(`${asked} of ${asked} held`);
            Qt.exit(0);
        }
    }
}
