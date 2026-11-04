// How the shell says each call call-transcripts wrote down: what it was,
// when and how long, its drawer's words set from markdown, and which
// calls are with someone or of a meeting, run in Qt's own JavaScript
// engine, the one the shell runs it in. The
// `the-shell-reads-the-transcripts` check copies Transcripts.js beside
// this file and runs `qml -platform offscreen transcripts.qml`.
import QtQuick
import "Transcripts.js" as Transcripts

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
        const check = (what, got, expected) => {
            asked++;
            if (JSON.stringify(got) !== JSON.stringify(expected))
                failures.push(`${what} gave ${JSON.stringify(got)}, not ${JSON.stringify(expected)}`);
        };
        const said = {
            meeting: "meeting",
            person: "call",
            call: "call",
            writing: "being written down",
            on: "not written down",
            yesterday: "yesterday",
            under: "under a minute",
            minutes: "{minutes} min",
            hours: "{hours} h",
            hoursMinutes: "{hours} h {minutes} min",
            length: "Length",
            with: "With",
            date: (date, format) => Qt.formatDate(date, format)
        };
        const minute = 60000;
        // Tuesday 3 November 2026, 15:00 local time, as the shell's clock.
        const now = new Date(2026, 10, 3, 15, 0).getTime();
        const at = (day, hour, minutes) => new Date(2026, 10, day, hour, minutes).getTime();
        const call = (fields) => Object.assign({
            key: "call:e1",
            kind: "call",
            title: "Planning",
            started: at(3, 14, 0),
            ended: at(3, 14, 32),
            event: "e1",
            person: null,
            folder: "/calls/a",
            transcript: "/calls/a/transcript.md",
            opening: null,
            summary: null,
            transcribing: false
        }, fields);

        // What it was, as its side says it.
        check("a meeting's kind", Transcripts.kind(call({})), "meeting");
        check("a Tuple call's kind", Transcripts.kind(call({ event: null, person: "ada@example.com" })),
            "person");
        check("a bare call's kind", Transcripts.kind(call({ event: null })), "call");

        // Its note: [fields, said]
        const notes = [
            [{}, "14:00 \u00b7 32 min"],
            [{ started: at(3, 16, 0), ended: at(3, 16, 5) }, "16:00 \u00b7 5 min"],
            [{ started: at(2, 9, 5), ended: at(2, 10, 10) }, "yesterday 09:05 \u00b7 1 h 5 min"],
            [{ started: new Date(2026, 9, 30, 9, 0).getTime(),
                ended: new Date(2026, 9, 30, 10, 0).getTime() }, "Friday 09:00 \u00b7 1 h"],
            [{ started: new Date(2026, 9, 20, 9, 0).getTime(),
                ended: new Date(2026, 9, 20, 9, 0).getTime() + 30000 },
                "20 Oct 09:00 \u00b7 under a minute"],
            [{ started: new Date(2025, 11, 31, 9, 0).getTime(),
                ended: new Date(2025, 11, 31, 9, 0).getTime() + 5 * minute },
                "31 Dec 2025 09:00 \u00b7 5 min"],
            [{ ended: null, transcribing: true }, "being written down"],
            [{ ended: null }, "14:00 \u00b7 not written down"]
        ];
        for (const [fields, expected] of notes)
            check(`the note of ${JSON.stringify(fields)}`, Transcripts.note(call(fields), now, said),
                expected);

        // Markdown as the drawer's styled text.
        const styles = [
            ["**Joel**, 0:00: Can we move it?\n\n**Others**, 0:04: Thursday works.",
                "<b>Joel</b>, 0:00: Can we move it?<br><br><b>Others</b>, 0:04: Thursday works."],
            ["# Decided\n\n- Move it to *Thursday*\n- Ada writes the notes",
                "<b>Decided</b><br><br>\u2022 Move it to <i>Thursday</i><br>\u2022 Ada writes the notes"],
            ["> \"We ship Friday.\"\n> Joel", "<i>\"We ship Friday.\"</i><br><i>Joel</i>"],
            ["A line\nwrapped by hand", "A line wrapped by hand"],
            ["1 < 2 & `x` > 0", "1 &lt; 2 &amp; x &gt; 0"],
            ["snake_case_name stays", "snake_case_name stays"],
            ["", ""]
        ];
        for (const [markdown, expected] of styles)
            check(`styling ${JSON.stringify(markdown)}`, Transcripts.styled(markdown), expected);

        // The drawer: the summary over how it opens, and who it was with.
        const summed = Transcripts.details(call({ summary: "- Ship it", opening: "**Joel**, 0:00: Hi" }),
            { title: "Planning", who: "" }, said, now);
        check("a summed-up call's drawer", summed, {
            eyebrow: "Tuesday 3 November \u00b7 14:00",
            title: "Planning",
            facts: [{ label: "Length", value: "32 min" }],
            body: "\u2022 Ship it",
            styled: true,
            sans: true
        });
        const opened = Transcripts.details(call({ event: null, person: "ada@example.com",
            opening: "**Ada**, 0:00: Hi", ended: null, transcribing: true }),
            { title: "Ada", who: "Ada Lovelace" }, said, now);
        check("a call being written's drawer", opened, {
            eyebrow: "Tuesday 3 November \u00b7 14:00",
            title: "Ada",
            facts: [{ label: "With", value: "Ada Lovelace" }],
            body: "<b>Ada</b>, 0:00: Hi",
            styled: true,
            sans: true
        });
        check("a call with someone People doesn't know",
            Transcripts.details(call({ person: "ada@example.com" }), { title: "Ada", who: "" },
                said, now).facts[1],
            { label: "With", value: "ada@example.com" });
        check("a call of another year's eyebrow",
            Transcripts.details(call({ started: new Date(2025, 11, 31, 9, 0).getTime() }),
                { title: "Planning", who: "" }, said, now).eyebrow,
            "Wednesday 31 December 2025 \u00b7 09:00");

        // Newest first, anything not a call left out; by someone; by a
        // meeting.
        const records = [
            call({ key: "call:e1", started: at(1, 9, 0) }),
            { key: "person:x", kind: "person" },
            call({ key: "call:person:ada@example.com:1", event: null, person: "ada@example.com",
                started: at(3, 9, 0) }),
            call({ key: "call:e2", event: "e2", started: at(2, 9, 0) })
        ];
        const newest = Transcripts.newestFirst(records);
        check("the calls newest first", newest.map(one => one.key),
            ["call:person:ada@example.com:1", "call:e2", "call:e1"]);
        check("the calls with Ada, by any address",
            newest.filter(one => Transcripts.withThem(one, ["ADA@example.com", "a@b.c"]))
                .map(one => one.key), ["call:person:ada@example.com:1"]);
        check("a meeting's call", Transcripts.ofEvent(newest, "e2")?.key, "call:e2");
        check("a meeting with no call", Transcripts.ofEvent(newest, "e9"), null);
        check("no meeting at all", Transcripts.ofEvent(newest, ""), null);

        // Said as warnings, since Qt keeps ordinary logging quiet.
        for (const failure of failures)
            console.warn(failure);
        console.warn(failures.length > 0 ? `${failures.length} went wrong of ${asked}`
            : `all ${asked} held`);
        Qt.exit(failures.length > 0 ? 1 : 0);
    }
}
