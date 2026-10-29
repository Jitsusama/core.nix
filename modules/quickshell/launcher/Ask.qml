// A quick question answered in the field by a model, in a sentence or two:
// what's typed ending in "?", or starting with a question's word, three
// words or more. A request in plain words, two or more that aren't a
// question, is offered too, when nothing else is found: pi-ask does it
// through the launcher's catalogue, so "brightness to 40%" sets it, which
// the search alone can't (Joel, 2026-10-26: "i thought the point was to use
// an llm for that?"). Other answers are asked first, so a sum or "is 10.0.0.1
// in 10.0.0.0/8" never reaches it. Nothing is asked as Joel types: the row
// offers to ask, and Return asks, the launcher staying open while the
// answer is written in place, the whole of it below the rows. The right
// arrow holds talking it over with the orchestrator running, and carrying
// the question on in a session of its own.
//
// It talks to pi through pi's own RPC, a JSON object a line each way, as
// 85PKB2's contract sets out (pi-ask, the pi module's, which picks the
// machine's plan and model). Each question goes to a pi of its own, so no
// answer carries the last and pi's context never grows: one is started
// with the shell and waits, and once it's asked, the next is started as
// its answer ends, so pi's start, a second or more, is never waited on.
// A new pi rather than RPC's new session, since a work machine's proxy
// extension keeps the session it started with, and a prompt after a new
// session reached no model at all (2026-10-26). A new question while one
// is answered, or the launcher closing, stops that pi.
//
// A pi that ends before it's ready says why, as its last line of errors,
// in the row, so Ask never waits on one that can't start; it's started
// again after 2 s, then longer each time it ends that way, up to a minute,
// and at once when Return asks.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "../Fuzzy.js" as Fuzzy

Provider {
    id: provider

    // Starting a question with one of these, three words or more, asks
    // it, as ending in "?" does.
    readonly property var askingWords: ["what", "whats", "how", "why", "who", "when", "where",
        "which", "can", "could", "should", "would", "does", "do", "is", "are", "will"]
    readonly property int fewestWords: 3
    // A request takes two, so "dim screen" is one and a word alone is
    // left to the search.
    readonly property int fewestRequestWords: 2

    // The question asked last, what's been said to it, and how it's going:
    // "" before any, "asking", "answering", "answered", "stopped" or
    // "failed", with why when it failed. Provider's own `asked` and
    // `ask` are its cache of what URLs said, so these are named apart.
    property string posed: ""
    property string said: ""
    property string stage: ""
    property string why: ""
    property real askedAt: 0
    property real answeredAt: 0

    // The question waiting for a pi that hasn't been asked one, whether
    // this one has, and whether it's being ended to start the next.
    property string waiting: ""
    property bool spent: false
    property bool renewing: false
    property int count: 0
    property bool answering: false

    // How the last pi ended: its last line of errors, and how many in a
    // row ended before saying anything, which sets how long until the
    // next is started.
    property string lastError: ""
    property int failures: 0
    property bool spoke: false
    readonly property int firstWait: 2000
    readonly property int longestWait: 60000

    function isQuestion(text) {
        const words = Fuzzy.words(text);
        if (words.length === 0 || !/[a-z]/i.test(text))
            return false;
        if (text.endsWith("?") && text.length > 2)
            return true;
        return words.length >= fewestWords
            && askingWords.includes(Fuzzy.plainly(words[0]).replace(/'/g, ""));
    }

    function isRequest(text) {
        return /[a-z]/i.test(text) && Fuzzy.words(text).length >= fewestRequestWords;
    }

    function answer(typed) {
        const question = typed.trim();
        const asked = isQuestion(question);
        if (!asked && !isRequest(question))
            return null;
        const row = question === posed && stage !== "" ? reply(question)
            : offer(question, asked);
        // Asked by a question's word rather than "?", or a request, it gives
        // way to anything found, so "do not disturb" is Do not disturb.
        return question.endsWith("?") ? row : Object.assign(row, { fallback: true });
    }

    // Whether the last pi ended before it said anything, so the next may
    // not start either: the row says why until one does.
    readonly property bool down: failures > 0

    // Before it's asked: Return asks, and the launcher stays to show it.
    // While pi can't start, it says why, and Return tries again at once.
    function offer(question, asked) {
        return {
            headline: Words.name("ask"),
            verb: Words.verb("ask"),
            copy: "",
            stays: true,
            run: () => provider.pose(question),
            rows: talkOn(question, "").concat([onward(question, "")]),
            note: down ? `it can't start: ${lastError || "pi ended"}`
                : asked ? "a quick answer, in a sentence or two"
                : "done through the launcher, where it can be",
            glyph: Words.glyph("ask"),
            hue: down ? "alert" : "voice"
        };
    }

    // As it's answered, and once it is: its first sentence as the row's
    // name, the whole of it below the rows, and Return copying it. One that
    // failed says why, and Return asks it again rather than closing.
    function reply(question) {
        const whole = said.trim();
        const first = (whole.match(/^[\s\S]*?[.!?](?=\s|$)/) ?? [whole])[0];
        const took = Math.max(0, Math.round(((answeredAt || Date.now()) - askedAt) / 1000));
        const notes = {
            asking: "thinking",
            answering: "answering",
            answered: `answered in ${took} s`,
            stopped: "stopped",
            failed: "it couldn't answer"
        };
        const failed = stage === "failed";
        const again = failed || (stage === "stopped" && whole === "");
        return Object.assign({
            headline: failed ? why : whole === "" ? "\u2026" : first,
            copy: failed ? "" : whole,
            reads: failed || first === whole ? "" : whole,
            asking: stage === "asking",
            rows: talkOn(question, whole).concat([onward(question, whole)]),
            note: notes[stage] ?? "",
            glyph: Words.glyph("ask"),
            hue: failed ? "alert" : "voice"
        }, again ? {
            verb: Words.verb("ask"),
            stays: true,
            run: () => provider.pose(question)
        } : stage === "asking" || stage === "answering" ? {
            // Return while it's still answering leaves it answering.
            stays: true,
            run: () => {}
        } : {});
    }

    // The question, and what was answered so far, to carry on elsewhere.
    function carried(question, answered) {
        return answered === "" ? question : `${question}\n\nThe launcher answered: ${answered}`;
    }

    // Talking it over with an orchestrator (P9R1KI, 2026-10-26: a row
    // under Ask's answer sending the question on to it): the first one
    // running, in Orchestrators' own order, so one waiting on Joel comes
    // first. Its words go to its inbox as Talk's do, and the launcher lands
    // inside it, where the talk says how it went as its session replies.
    // With none running, there's no row.
    function talkOn(question, answered) {
        const source = (launcher?.sources ?? []).find(one => one.settings.name
            === "orchestrators") ?? null;
        const row = source ? source.all.find(one => one.stands !== "off") ?? null : null;
        const one = row ? source.listed.find(listed => `orchestrator:${listed.address}`
            === row.key) ?? null : null;
        if (one === null)
            return [];
        return [thing({
            key: "verb:ask:talk",
            verb: Words.verb("verb:ask:talk"),
            name: Words.name("verb:ask:talk").replace("{name}", row.name),
            glyph: Words.glyph("verb:ask:talk"),
            hue: "action",
            stays: true,
            run: () => {
                Sessions.talk(one.address, carried(question, answered));
                // By the id the launcher holds it by, which for a running
                // one is its session's, not its key.
                launcher.goTo({ trail: ["collection:agents", "collection:orchestrators",
                    launcher.steadyId(row)], spot: "" });
            }
        })];
    }

    // Carrying it on in a session of its own, in a terminal in the home
    // folder, with what was answered so far.
    function onward(question, answered) {
        const text = carried(question, answered);
        return thing({
            key: "verb:ask:onward",
            verb: Words.verb("verb:ask:onward"),
            name: Words.name("verb:ask:onward"),
            glyph: Words.glyph("verb:ask:onward"),
            hue: "action",
            run: () => Sessions.tell(text, Sessions.home, settings.session)
        });
    }

    function send(command) {
        pi.write(`${JSON.stringify(command)}\n`);
    }

    function pose(question) {
        posed = question;
        said = "";
        why = "";
        stage = "asking";
        askedAt = Date.now();
        answeredAt = 0;
        waiting = question;
        told();
        if (pi.running && !spent)
            put();
        else
            renew();
    }

    // The question waiting, to a pi that's running and hasn't been asked.
    // pi reads it once it's ready, so it may go as soon as pi starts.
    function put() {
        if (waiting === "")
            return;
        spent = true;
        answering = true;
        count += 1;
        send({ id: `ask-${count}`, type: "prompt", message: waiting });
        waiting = "";
    }

    // A pi of its own for what's asked next: the one running ended first,
    // or, with none, one started now.
    function renew() {
        if (pi.running) {
            renewing = true;
            pi.running = false;
            return;
        }
        again.stop();
        pi.running = true;
    }

    // Closed while it answers, the answer's not wanted.
    function stop() {
        waiting = "";
        if (stage === "asking" || stage === "answering") {
            stage = "stopped";
            answeredAt = Date.now();
            told();
        }
        if (answering)
            renew();
    }

    function failed(reason) {
        why = String(reason ?? "it couldn't answer");
        stage = "failed";
        answeredAt = Date.now();
        told();
    }

    // Each line pi writes: a command's response, or an event of the
    // question being answered.
    function heard(line) {
        // One being ended says nothing more of the question after it.
        if (renewing)
            return;
        let event = null;
        try {
            event = JSON.parse(line);
        } catch (error) {
            return;
        }
        // A complaint an extension prints as it loads isn't pi ready.
        if (event.type === "extension_error")
            return;
        spoke = true;
        failures = 0;
        lastError = "";
        if (event.type === "response") {
            if (String(event.id ?? "").startsWith("ask-") && event.success === false) {
                answering = false;
                failed(event.error);
                renew();
            }
            return;
        }
        if (event.type === "message_update"
            && event.assistantMessageEvent?.type === "text_delta") {
            said += event.assistantMessageEvent.delta ?? "";
            stage = "answering";
            told();
        } else if (event.type === "tool_execution_start") {
            // What came before a call is only about making it ("Let me set
            // it to 40%"), which the prompt alone didn't stop, so the row
            // says what was done.
            said = "";
            told();
        } else if (event.type === "message_end" && event.message?.role === "assistant"
            && event.message.stopReason === "error") {
            failed(event.message.errorMessage);
        } else if (event.type === "agent_settled") {
            answering = false;
            if (stage === "answering" || stage === "asking") {
                stage = event.aborted === true ? "stopped" : "answered";
                answeredAt = Date.now();
                told();
            }
            renew();
        }
    }

    // A word a moment, rather than every few letters the model writes, so
    // the launcher's rows are made again no faster than they're read.
    function told() {
        if (!telling.running)
            telling.start();
    }

    Timer {
        id: telling
        interval: 100
        onTriggered: provider.answered()
    }

    Connections {
        target: provider.launcher
        ignoreUnknownSignals: true
        function onOpenChanged() {
            if (!provider.launcher.open)
                provider.stop();
        }
    }

    Process {
        id: pi
        command: [provider.settings.command ?? "pi-ask"]
        running: true
        stdinEnabled: true
        stdout: SplitParser {
            splitMarker: "\n"
            onRead: line => provider.heard(line)
        }
        stderr: SplitParser {
            splitMarker: "\n"
            onRead: line => {
                const plain = line.trim().replace(/^error:\s*/i, "");
                if (plain !== "")
                    provider.lastError = plain;
            }
        }
        onStarted: {
            provider.spent = false;
            provider.spoke = false;
            provider.put();
        }
        onExited: {
            provider.answering = false;
            const meant = provider.renewing;
            provider.renewing = false;
            if (!meant && !provider.spoke)
                provider.failures += 1;
            if (!meant && (provider.stage === "asking" || provider.stage === "answering"))
                provider.failed(provider.lastError || "it stopped");
            if (meant) {
                Qt.callLater(() => pi.running = true);
                return;
            }
            again.interval = Math.min(provider.longestWait,
                provider.firstWait * 2 ** Math.max(0, provider.failures - 1));
            again.start();
            provider.told();
        }
    }

    Timer {
        id: again
        interval: provider.firstWait
        onTriggered: pi.running = true
    }
}
