// A quick question answered in the field by a model, in a sentence or two:
// what's typed ending in "?", or starting with a question's word, three
// words or more. Other answers are asked first, so a sum or "is 10.0.0.1
// in 10.0.0.0/8" never reaches it. Nothing is asked as Joel types: the row
// offers to ask, and Return asks, the launcher staying open while the
// answer is written in place, the whole of it below the rows. The right
// arrow holds carrying the question on in a session of its own.
//
// It talks to one long-lived pi through pi's own RPC, a JSON object a
// line each way, as 85PKB2's contract sets out (pi-ask, the pi module's,
// which picks the machine's plan and model). Each question starts a new
// session first, so no answer carries the last and pi's context never
// grows; a new question while one is answered, or the launcher closing,
// stops the one being answered. pi is started once with the shell, and
// again 2 s after it ends.
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

    // What pi is waiting to be asked once the session it was told to
    // start is ready, or once the question it's stopping has stopped.
    property string next: ""
    property int count: 0
    property string starting: ""
    property bool answering: false
    property bool stopping: false

    function isQuestion(text) {
        const words = Fuzzy.words(text);
        if (words.length === 0 || !/[a-z]/i.test(text))
            return false;
        if (text.endsWith("?") && text.length > 2)
            return true;
        return words.length >= fewestWords
            && askingWords.includes(Fuzzy.plainly(words[0]).replace(/'/g, ""));
    }

    function answer(typed) {
        const question = typed.trim();
        if (!isQuestion(question))
            return null;
        const row = !pi.running ? {
            headline: Words.name("ask", "starting"),
            copy: "",
            rows: [],
            note: "it answers in a moment",
            glyph: Words.glyph("ask"),
            hue: "voice"
        } : question === posed ? reply(question) : offer(question);
        // Asked by a question's word rather than "?", it gives way to
        // anything found, so "do not disturb" is Do not disturb.
        return question.endsWith("?") ? row : Object.assign(row, { fallback: true });
    }

    // Before it's asked: Return asks, and the launcher stays to show it.
    function offer(question) {
        return {
            headline: Words.name("ask"),
            verb: Words.verb("ask"),
            copy: "",
            stays: true,
            run: () => provider.pose(question),
            rows: [onward(question, "")],
            note: "a quick answer, in a sentence or two",
            glyph: Words.glyph("ask"),
            hue: "voice"
        };
    }

    // As it's answered, and once it is: its first sentence as the row's
    // name, the whole of it below the rows, and Return copying it.
    function reply(question) {
        const whole = said.trim();
        const first = (whole.match(/^[\s\S]*?[.!?](?=\s|$)/) ?? [whole])[0];
        const took = Math.max(0, Math.round(((answeredAt || Date.now()) - askedAt) / 1000));
        const notes = {
            asking: "asking",
            answering: "answering",
            answered: `answered in ${took} s`,
            stopped: "stopped",
            failed: "it couldn't answer"
        };
        const failed = stage === "failed";
        return {
            headline: failed ? why : whole === "" ? "\u2026" : first,
            copy: failed ? "" : whole,
            reads: failed || first === whole ? "" : whole,
            asking: stage === "asking",
            rows: [onward(question, whole)],
            note: notes[stage] ?? "",
            glyph: Words.glyph("ask"),
            hue: failed ? "alert" : "voice"
        };
    }

    // Carrying it on in a session of its own, in a terminal in the home
    // folder, with what was answered so far.
    function onward(question, answered) {
        const text = answered === "" ? question
            : `${question}\n\nThe launcher answered: ${answered}`;
        return thing({
            key: "verb:ask:onward",
            verb: Words.verb("verb:ask:onward"),
            name: Words.name("verb:ask:onward"),
            glyph: Words.glyph("verb:ask:onward"),
            hue: "action",
            run: () => Niri.spawn(["kitty", "--directory", Quickshell.env("HOME"),
                settings.session ?? "pi", "--", text])
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
        told();
        if (answering || starting !== "") {
            next = question;
            stop();
            return;
        }
        begin(question);
    }

    function begin(question) {
        next = question;
        count += 1;
        starting = `new-${count}`;
        send({ id: starting, type: "new_session" });
    }

    function stop() {
        if (stopping || !answering)
            return;
        stopping = true;
        count += 1;
        send({ id: `stop-${count}`, type: "abort" });
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
        let event = null;
        try {
            event = JSON.parse(line);
        } catch (error) {
            return;
        }
        if (event.type === "response") {
            responded(event);
            return;
        }
        if (event.type === "message_update"
            && event.assistantMessageEvent?.type === "text_delta") {
            said += event.assistantMessageEvent.delta ?? "";
            stage = "answering";
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
        }
    }

    function responded(event) {
        const id = String(event.id ?? "");
        if (id === starting) {
            starting = "";
            // Asked again while it started, the last is the one asked.
            if (next === posed) {
                answering = true;
                count += 1;
                send({ id: `ask-${count}`, type: "prompt", message: next });
            }
        } else if (id.startsWith("ask-") && event.success === false) {
            answering = false;
            failed(event.error);
        } else if (id.startsWith("stop-")) {
            stopping = false;
            answering = false;
            if (next !== "" && next === posed && stage === "asking")
                begin(next);
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

    // Closed while it answers, the answer's not wanted.
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
        onExited: {
            provider.answering = false;
            provider.stopping = false;
            provider.starting = "";
            if (provider.stage === "asking" || provider.stage === "answering")
                provider.failed("it stopped");
            again.start();
        }
    }

    Timer {
        id: again
        interval: 2000
        onTriggered: pi.running = true
    }
}
