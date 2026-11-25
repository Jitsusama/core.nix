// Talk to It on a session that asks Joel something, run in Qt's own JavaScript engine, the one
// the shell runs it in: what it asks shown whole as the talk's first exchange, with what it
// takes; his words answering it, a choice by its label or value or his own words where it
// takes them, and nothing where it takes neither; and his answer sent, then answered or
// working, or not delivered and why. The `a-talk-answers-what-its-session-asks` check copies
// launcher/TalkAnswers.js, the TalkLine.js, Told.js and TalkNotes.js it reads and
// tests/lib/Expect.js beside this file and runs `qml -platform offscreen talk-answers.qml`.
import QtQuick
import "Expect.js" as Expect
import "launcher/TalkAnswers.js" as Answers

QtObject {
    Component.onCompleted: {
        try {
            // The words as launcher/words/agents.json's verb:agent:talk says them, with the
            // stages Orchestrators' talks say.
            const words = { asks: "{name} asks you", choices: "{choices}, or your own words",
                only: "{choices}", sent: "sent", unsent: "not delivered", answered: "answered",
                working: "working on it", unheard: "not known whether it arrived" };
            // As the agents daemon serves a want (modules/agents/src/wants.rs), for the
            // session Joel was asked by from the realm's map on 24 November.
            const asked = "2. The signing pause: yes, no or partly? The commits stay unsigned "
                + "until you say, and partly keeps the old ones as they are.";
            // Its id is its kind and when it began, as wants.rs names it.
            const since = new Date(2026, 10, 24, 13, 9).toISOString();
            const want = { title: "Sign the Commits", asked, said: asked, since,
                answers: { want: `question@${since}`, own: true, choices: [
                    { value: "yes", label: "Yes", recommended: true },
                    { value: "no", label: "No" },
                    { value: "partly", label: "Partly" }] } };
            const dayOf = () => "";
            // As Agents.qml's answersOf hands it over: the want, and whether it's asking.
            const asks = (shown, asking) => ({ want: shown, asking });

            // Joel at 13:15 on 2026-11-24: "talking to the agent is worse than talking to the
            // void via that interface. no feedback at all".
            Expect.same("what it asks is the talk's first exchange, whole, with what it takes",
                Answers.question(asks(want, true), "Sign the Commits", words, dayOf), {
                    line: "Sign the Commits asks you", answers: [asked], waiting: "",
                    stands: "Yes \u00b7 No \u00b7 Partly, or your own words", by: "",
                    side: "13:09" });
            // A long question is elided at eight lines (TalkSpoken.qml), the line under it
            // never, so what it takes can't be cut off with Return still answering it.
            const long = Array.from({ length: 12 }, (_, at) => `line ${at}`).join("\n");
            Expect.same("what it takes stands apart from a long question",
                Answers.question(asks(Object.assign({}, want, { whole: long }), true), "S",
                    words, dayOf).answers, [long]);
            Expect.same("one saying nothing of what it asks still says what it takes",
                Answers.question(asks(Object.assign({}, want, { asked: "Sign the Commits",
                    said: "" }), true), "S", words, dayOf), {
                    line: "S asks you", answers: [], waiting: "",
                    stands: "Yes \u00b7 No \u00b7 Partly, or your own words", by: "",
                    side: "13:09" });
            Expect.same("one whose short line is its title but says more shows its line",
                Answers.question(asks(Object.assign({}, want, { asked: "Sign the Commits",
                    said: "Sign them?" }), true), "S", words, dayOf).answers,
                ["Sign the Commits"]);
            Expect.same("one taking only its choices says only them",
                Answers.question(asks(Object.assign({}, want, { answers: Object.assign({},
                    want.answers, { own: false }) }), true), "S", words, dayOf).stands,
                "Yes \u00b7 No \u00b7 Partly");
            Expect.same("one offering no answers still shows what it asks, and no takes",
                ["answers", "stands"].map(key => Answers.question(asks(Object.assign({}, want,
                    { answers: null }), true), "S", words, dayOf)[key]), [[asked], ""]);
            // Its whole question, as the harness keeps it beside the short line (85PKB2,
            // 2026-11-24): its lines and indentation kept, its blank lines before and spaces
            // after let go.
            const wholly = "Which signing pause?\n\n- Yes: sign all\n  - the old ones too\n- No";
            Expect.same("a want saying more than its short line shows it whole",
                Answers.question(asks(Object.assign({}, want, { whole: `\n${wholly}  \n` }), true),
                    "S", words, dayOf).answers, [wholly]);
            Expect.same("a whole question is told where its short line is the title",
                Answers.question(asks(Object.assign({}, want, { asked: "Sign the Commits",
                    said: "", whole: wholly }), true), "S", words, dayOf).answers[0], wholly);
            Expect.same("its first line's indentation is kept, its blank lines around dropped",
                Answers.whole({ whole: "\n \n  - low\n- high \n" }), "  - low\n- high");
            Expect.same("no want, no whole", Answers.whole(null), "");
            Expect.same("a blank whole leaves the short line",
                Answers.question(asks(Object.assign({}, want, { whole: " \n " }), true), "S",
                    words, dayOf).answers[0], asked);
            Expect.same("a name with $& in it goes in as written",
                Answers.question(asks(want, true), "A $& B", words, dayOf).line,
                "A $& B asks you");
            Expect.same("a day before the clock where it's another day",
                Answers.question(asks(want, true), "S", words, () => "yesterday").side,
                "yesterday 13:09");
            for (const [why, shown, asking] of [
                ["nothing while it asks nothing", want, false],
                ["nothing for no want", null, true],
                ["nothing where it says nothing of what nor offers answers", Object.assign({},
                    want, { asked: "Sign the Commits", said: "", answers: null }), true]
            ]) {
                Expect.same(why, Answers.question(asks(shown, asking), "S", words, dayOf),
                    null);
            }

            Expect.same("its answers while it asks", Answers.asked(want, true), want.answers);
            for (const [why, shown, asking] of [
                ["one not asking, whose words go as a talk", want, false],
                ["no answers", Object.assign({}, want, { answers: null }), true],
                ["no choices", Object.assign({}, want, { answers: { want: "q@1", choices: [] } }),
                    true],
                ["no want id", Object.assign({}, want, { answers: Object.assign({},
                    want.answers, { want: "" }) }), true]
            ]) {
                Expect.same(`nothing to answer for ${why}`, Answers.asked(shown, asking), null);
            }

            const answers = want.answers;
            Expect.same("a choice by its label alone, in any case",
                Answers.answerOf("  some of THEM ", { want: "q@1", own: false, choices: [
                    { value: "partly", label: "Some of them" }] }),
                { value: "partly", label: "Some of them" });
            Expect.same("a label before another choice's value",
                Answers.answerOf("no", { want: "q@1", own: false, choices: [
                    { value: "no", label: "Not now" }, { value: "never", label: "No" }] }),
                { value: "never", label: "No" });
            Expect.same("a choice by its value",
                Answers.answerOf("NO", { want: "q@1", own: false, choices: [
                    { value: "no", label: "Not now" }] }), { value: "no", label: "Not now" });
            Expect.same("his own words where it takes them",
                Answers.answerOf("yes, but only the last three", answers),
                { other: "yes, but only the last three", label: "yes, but only the last three" });
            Expect.same("nothing where it takes only its choices",
                Answers.answerOf("maybe", Object.assign({}, answers, { own: false })), null);
            Expect.same("nothing for blanks", Answers.answerOf("   ", answers), null);

            const at = new Date(2026, 10, 24, 13, 20).getTime();
            const one = Answers.said(" Partly ", at, answers.want);
            Expect.same("an answer reads sent until the node puts it",
                Answers.shown(one, false, words, dayOf),
                { line: "Partly", answers: [], waiting: "", stands: "sent", by: "",
                    side: "13:20" });
            const put = Answers.put(one, true, "");
            Expect.same("then answered once it's in the session's inbox",
                Answers.shown(put, false, words, dayOf).stands, "answered");
            Expect.same("working, breathing, while the session works on it",
                [Answers.shown(put, true, words, dayOf).waiting,
                    Answers.shown(put, true, words, dayOf).stands], ["working on it", ""]);
            Expect.same("not delivered, and why, where the node didn't put it",
                Answers.shown(Answers.put(one, false, "its want has gone"), true, words, dayOf)
                    .stands, "not delivered \u00b7 its want has gone");
            Expect.same("not delivered alone where no why came",
                Answers.shown(Answers.put(one, false, ""), false, words, dayOf).stands,
                "not delivered");
            // The agents daemon gone while it was asked: the node may have put it or not.
            const unheard = Answers.put(one, false, "", true);
            Expect.same("not known whether it arrived where no answer came back",
                Answers.shown(unheard, true, words, dayOf).stands,
                "not known whether it arrived");
            Expect.same("never not delivered where it may have been",
                Answers.shown(unheard, false, words, dayOf).stands.includes("not delivered"),
                false);
            Expect.same("one not known shows under a later ask for a quarter hour",
                [at + 1000, at + 20 * 60 * 1000].map(nowMs =>
                    Answers.standing([unheard], "q@2", nowMs)), [[unheard], []]);

            // A session's answers kept and shown (Agents.qml's answerIn and answerTalks).
            const later = Answers.said("No", at + 1000, answers.want);
            Expect.same("a new answer goes first", Answers.kept([one], later), [later, one]);
            Expect.same("one put stays in its place, a newer one said since kept first",
                Answers.kept([later, one], put), [later, put]);
            const three = [3, 2, 1].map(n => Answers.said(`${n}`, at + n * 1000, "q@2"));
            Expect.same("only the newest three are kept",
                Answers.kept(three, later).map(kept => kept.words), ["No", "3", "2"]);
            Expect.same("one dropped never comes back once put",
                Answers.kept(three, Answers.put(Answers.said("0", at, "q@2"), true, "")),
                three);
            const shownAt = at + 20 * 60 * 1000;
            // Read inside the quarter hour, so the other is let go for its want alone.
            Expect.same("while it asks, only answers to what it asks show",
                Answers.standing([three[0], put], answers.want, at + 5 * 60 * 1000), [put]);
            Expect.same("once it asks nothing, one said a quarter hour ago no longer shows",
                [at + 15 * 60 * 1000 - 1, at + 15 * 60 * 1000].map(nowMs =>
                    Answers.standing([put], "", nowMs)), [[put], []]);
            Expect.same("once it asks nothing, those said in the last quarter hour show",
                Answers.standing([three[0], put], "", at + 10 * 60 * 1000), [three[0], put]);
            Expect.same("and none said longer ago", Answers.standing([put], "", shownAt), []);
            Expect.same("while it asks what he can't answer here, none answered show",
                Answers.standing([put], null, at + 1000), []);
            // Refused for answering an ask since replaced, as the node refuses one (wants.rs).
            const refused = Answers.put(one, false, "it asks something else now");
            Expect.same("one not delivered shows under the ask that replaced its own",
                Answers.standing([three[0], refused], "q@2", at + 1000), [three[0], refused]);
            Expect.same("and under one he can't answer here",
                Answers.standing([refused], null, at + 1000), [refused]);
            Expect.same("but not once a quarter hour has gone",
                Answers.standing([refused], "q@2", shownAt), []);

            // What the Talk shows answers to (Agents.qml's answerTalks).
            Expect.same("the id of what it asks while it asks one he can answer",
                Answers.shownWant(asks(want, true), answers), answers.want);
            Expect.same("null while it asks one he can't answer here",
                Answers.shownWant(asks(Object.assign({}, want, { answers: null }), true), null),
                null);
            Expect.same("nothing once it asks nothing, parked among them",
                Answers.shownWant(asks(want, false), null), "");

            // What his Return answers (Agents.qml's talked): what the Talk held drew, or for a
            // walk from a terminal, which draws none, what it asks now.
            const first = { want: "q@1", own: false, choices: [{ value: "a", label: "A" }] };
            const second = Object.assign({}, first, { want: "q@2" });
            Expect.same("a held Talk that drew one ask answers it though it asks another now",
                Answers.answering("ASKS", "ASKS", { ASKS: first }, second), first);
            Expect.same("a held Talk that drew it asking nothing to answer talks to it",
                Answers.answering("ASKS", "ASKS", { ASKS: null }, second), null);
            Expect.same("a walk, with no Talk held, answers what it asks now",
                Answers.answering(undefined, "ASKS", { ASKS: first }, second), second);
            Expect.same("as does one while another's Talk is held",
                Answers.answering("OTHER", "ASKS", { ASKS: first }, second), second);
            Expect.same("and one whose Talk drew nothing yet",
                Answers.answering("ASKS", "ASKS", {}, second), second);

            // What a Talk drew its Return answering (Agents.qml's answerTalks).
            Expect.same("a Talk that drew the question answers it",
                Answers.drawn({ by: "them" }, first), first);
            Expect.same("one that drew no question answers nothing, though it asks",
                Answers.drawn(null, first), null);
            Expect.same("as does one asking nothing",
                Answers.drawn({ by: "them" }, undefined), null);

            // What his Return does (Agents.qml's given).
            Expect.same("his own Return answers what the Talk drew",
                Answers.onReturn(first, true), { act: "answer", answers: first });
            Expect.same("a walk that would answer is refused",
                Answers.onReturn(second, false), { act: "refused" });
            Expect.same("his Return to a Talk that drew no question talks",
                Answers.onReturn(null, true), { act: "talk" });
            Expect.same("as does a walk to one asking nothing",
                Answers.onReturn(undefined, false), { act: "talk" });

            // Whether his words stay to be given back (Agents.qml's answerIn).
            Expect.same("words refused under the same ask stay",
                Answers.keepsWords(false, first, first), true);
            Expect.same("words put go", Answers.keepsWords(true, first, first), false);
            Expect.same("words refused once it asks another go",
                Answers.keepsWords(false, second, first), false);
            Expect.same("words refused once it asks nothing go",
                Answers.keepsWords(false, null, first), false);
            const lost = Answers.put(Answers.said("A", at, "q@1"), false, "it asks something else");
            const lostToo = Answers.put(Answers.said("B", at, "q@1"), false, "", true);
            const meant = Answers.put(Answers.said("C", at, "q@2"), false, "");
            Expect.same("words kept for an ask it no longer asks are let go",
                Answers.stale([lost, lostToo, meant, put], "q@2"), ["A", "B"]);
            Expect.same("as are all kept once it asks nothing",
                Answers.stale([lost, meant], ""), ["A", "C"]);
            Qt.exit(0);
        } catch (error) {
            Expect.fail(error.message);
        }
    }
}
