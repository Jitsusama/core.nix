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
                working: "working on it" };
            // As the agents daemon serves a want (modules/agents/src/wants.rs), for the
            // session Joel was asked by from the realm's map on 24 November.
            const asked = "2. The signing pause: yes, no or partly? The commits stay unsigned "
                + "until you say, and partly keeps the old ones as they are.";
            const want = { title: "Sign the Commits", asked, said: asked,
                since: new Date(2026, 10, 24, 13, 9).toISOString(),
                answers: { want: "question@2026-11-24T18:09:00.000Z", own: true, choices: [
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
            Expect.same("while it asks, only answers to what it asks show",
                Answers.standing([three[0], put], answers.want, shownAt), [put]);
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
            Qt.exit(0);
        } catch (error) {
            Expect.fail(error.message);
        }
    }
}
