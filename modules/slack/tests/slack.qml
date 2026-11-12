// What Slack's source in People decides, run in Qt's own JavaScript
// engine, the one the shell runs it in: who's asked about and when, what
// stands in Write in Slack's place before Slack answers, and a
// conversation's drawer. The `slack-waits-in-its-place` check copies Slack.js beside this
// file and runs `qml -platform offscreen slack.qml`.
import QtQuick
import "Slack.js" as Slack

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
        const ada = { email: "ada@example.com", id: "U1", team: "T1", name: "Ada Lovelace",
            zone: "Europe/London", presence: "active", status: "Writing", emoji: "\u{1f4dd}" };
        const nobody = { email: "eve@elsewhere.example", id: null, team: "T1" };
        const known = { "ada@example.com": ada, "eve@elsewhere.example": nobody };
        const both = ["eve@elsewhere.example", "ada@example.com"];

        // What Slack told the people daemon, read back from its record.
        const told = Object.assign({}, ada);
        delete told.email;
        check("what Slack told the daemon, with the address its key holds",
            Slack.fromRecord({ key: "person:ada@example.com", sources: { slack: told } },
                "slack"),
            ada);
        check("nothing from a record Slack said nothing of",
            Slack.fromRecord({ key: "person:ada@example.com", sources: { tuple: { id: 7 } } },
                "slack"), null);
        check("nothing from a record kept by another source's id",
            Slack.fromRecord({ key: "person:tuple:7", sources: { slack: told } }, "slack"),
            null);

        check("found by any of their addresses", Slack.found(both, known), ada);
        check("not found by one Slack knows nobody by",
            Slack.found(["eve@elsewhere.example"], known), null);

        // In Write in Slack's place before Slack has found them.
        const unasked = ["grace@example.com"];
        check("looking while an address is still to be answered",
            Slack.waiting(unasked, known, {}, true), "looking");
        check("nothing waits with no sign-in to ask with",
            Slack.waiting(unasked, known, {}, false), "");
        check("missing once a step-in found nobody",
            Slack.waiting(["eve@elsewhere.example"], known, { "eve@elsewhere.example": true },
                true), "missing");
        check("no row for someone already known not to be there",
            Slack.waiting(["eve@elsewhere.example"], known, {}, true), "");
        check("no row for someone known by no address",
            Slack.waiting([], known, {}, true), "");

        // Asked as Joel steps in: once a minute, never for nobody.
        const minute = 60 * 1000;
        const now = 10 * minute;
        const when = (answered, asking) => ({ answered, asking, now, fresh: minute });
        check("someone unasked is asked", Slack.due(unasked, known, when({}, {})), unasked);
        check("an answer holds for a minute",
            Slack.due(["ada@example.com"], known,
                when({ "ada@example.com": now - minute + 1 }, {})), []);
        check("and is asked again after",
            Slack.due(["ada@example.com"], known,
                when({ "ada@example.com": now - minute }, {})), ["ada@example.com"]);
        check("an ask waits a minute for its answer, then goes again",
            [now - 1, now - minute].map(at =>
                Slack.due(unasked, known, when({}, { "grace@example.com": at }))),
            [[], unasked]);
        check("nobody is never asked about again",
            Slack.due(["eve@elsewhere.example"], known, when({}, {})), []);

        // Asked ahead: those listed, in order, each once.
        const listed = [{ emails: ["grace@example.com", "ada@example.com"] },
            { emails: ["hal@example.com", "grace@example.com"] }, { emails: [] }, null];
        check("those listed, unanswered and not yet sent, in order",
            Slack.ahead(listed, known, { "hal@example.com": true }), ["grace@example.com"]);

        // A restart's kept people come together: taken as one change.
        const burst = {};
        for (let at = 0; at < 86; at++)
            burst[`p${at}@example.com`] = { email: `p${at}@example.com`, id: `U${at}` };
        const once = Slack.settled(known, burst);
        check("answers that come together are one change",
            Object.keys(once ?? {}).length, 88);
        check("and what's already known is none", Slack.settled(once, burst), null);
        check("one new among the known is a change",
            Slack.settled(once, { "ada@example.com": Object.assign({}, ada, { presence: "away" }) })
                ?.["ada@example.com"].presence, "away");

        const talk = { channel: "D1", link: "", last: "See you", at: 2, messages: [
            { mine: true, text: "Lunch?", at: 1 }, { mine: false, text: "See you", at: 2 }] };
        const talking = Object.assign({}, ada, { conversation: talk });
        check("their conversation, by any address",
            Slack.talked(both, { "ada@example.com": talking }), talking);
        check("none until Slack finds one", Slack.talked(both, known), null);
        // Theirs last owes Joel an answer however old; his owes theirs
        // for two days.
        const days = 86_400_000;
        const lastMine = Object.assign({}, ada, { conversation: Object.assign({}, talk, {
            messages: talk.messages.slice().reverse() }) });
        check("who wrote last", [Slack.wroteLast(talking), Slack.wroteLast(lastMine)],
            [false, true]);
        check("theirs last owes him", Slack.owing(talking, 2 + 30 * days), true);
        check("his owes them a while", [Slack.owing(lastMine, 2 + days),
            Slack.owing(lastMine, 2 + 2 * days)], [true, false]);
        check("a conversation with nothing said owes nothing", Slack.owing(Object.assign({},
            ada, { conversation: Object.assign({}, talk, { messages: [] }) }), 2), false);
        check("its drawer, who said each, newest last", Slack.drawn(talking), {
            title: "Ada Lovelace",
            facts: [],
            body: "You: Lunch?\n\nAda Lovelace: See you"
        });

        if (failures.length > 0) {
            console.warn(`${failures.length} of ${asked} failed:\n${failures.join("\n")}`);
            Qt.exit(1);
        } else {
            console.log(`${asked} of ${asked} held`);
            Qt.exit(0);
        }
    }
}
