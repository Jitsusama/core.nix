// What the launcher's Choose by Ear works out, run in Qt's own JavaScript
// engine, the one the shell runs it in. The `choose-by-ear-walks-its-steps`
// check copies launcher/ByEar.js and launcher/Numbers.js beside this file
// and runs `qml -platform offscreen by-ear.qml` over trial records shaped
// as the noise-cancelled daemon sends them (the trial protocol): which step
// each stage is, what each row says, and what Return and leaving ask.
import QtQuick
import "launcher/ByEar.js" as ByEar
import "Expect.js" as Expect

QtObject {
    // A test that throws ends as a failure, rather than leaving qml running.
    Component.onCompleted: {
        try {
            run();
            Qt.exit(0);
        } catch (error) {
            Expect.fail(`the test threw: ${error}`);
        }
    }

    // The sound place's words, as launcher/words.json gives those read here.
    readonly property var said: ({
        down: "not answering",
        "cleaner-raw": "no noise cancelling",
        "cleaner-deepfilternet": "DeepFilterNet, full",
        "cleaner-deepfilternet-gentle": "DeepFilterNet, gentle",
        "cleaner-rnnoise": "RNNoise",
        "by-ear-starting": "starting",
        "by-ear-recording": "recording",
        "by-ear-cleaning": "cleaning",
        "by-ear-ready": "ready to hear",
        "by-ear-picked": "picked {clip}",
        "by-ear-seconds": "{seconds} s",
        "by-ear-left": "{seconds} s left",
        "by-ear-speak": "speak as you would on a call, with the room as it is",
        "by-ear-each": "with each cleaner, then shuffled as clips A to D",
        "by-ear-playing": "playing",
        "by-ear-on": "on {output}",
        "by-ear-playing-on": "playing on {output}",
        "by-ear-blind": "its cleaner is said once one is picked",
        "by-ear-chosen": "{clip} \u00b7 {cleaner}",
        "by-ear-from-now": "calls hear {microphone} through it from now",
        "by-ear-unchanged": "nothing changes: calls hear {microphone} as before",
        "by-ear-the-microphone": "the microphone",
        "by-ear-cost": "{share} of a core while cleaning",
        "by-ear-scored": "sounds {mark} of 5 \u00b7 {cost}",
        "by-ear-down": "its socket starts it on the next ask",
        "by-ear-spot-unnamed": "the laptop's name for here; give it yours, like the desk",
        "by-ear-spot-named": "picks here are kept under it, and stay when it's renamed"
    })

    // The record as the daemon sends it idle, with `fields` laid over.
    function record(fields) {
        return Object.assign({
            key: "trial:now", kind: "trial", heard: 1000, stage: "idle", why: null,
            microphone: null, seconds: 6, until: null, clips: [], playing: null,
            picked: null, revealed: null, costs: null, marks: null, running: "rnnoise"
        }, fields);
    }

    // What a step's rows say, as the launcher lists them.
    function says(rows) {
        return rows.map(row => [row.id, row.fill?.clip ?? "", row.side, row.note]);
    }

    function run() {
        const same = Expect.same;

        // Only a record whose stage the protocol has is drawn from.
        same("a whole record", ByEar.whole(record({})) !== null, true);
        same("a stage it doesn't know", ByEar.whole(record({ stage: "dancing" })), null);
        same("nothing", ByEar.whole(null), null);

        // The part says the cleaner the microphones run now, in Joel's words,
        // and with nothing held, how the daemon stands.
        same("running", ByEar.running(said, record({}), "answering"), "RNNoise");
        same("running none", ByEar.running(said, record({ running: "raw" }), "answering"),
            "no noise cancelling");
        same("running gentle",
            ByEar.running(said, record({ running: "deepfilternet-gentle" }), "answering"),
            "DeepFilterNet, gentle");
        same("not answering", ByEar.running(said, null, "not answering"), "not answering");
        same("starting", ByEar.running(said, null, "starting"), "starting");

        // Choose by Ear's own row says how the trial stands, nothing while idle.
        same("idle stands", ByEar.standing(said, record({})), "");
        same("ready stands", ByEar.standing(said, record({ stage: "ready" })), "ready to hear");
        same("picked stands", ByEar.standing(said, record({
            stage: "picked", picked: { clip: "B", cleaner: "raw" }
        })), "picked B");

        // Idle: one row, Record, whose Return asks the daemon to record.
        const idle = ByEar.rows(said, record({}), "answering", 0);
        same("idle", says(idle), [["sound:by-ear:record", "", "6 s",
            "speak as you would on a call, with the room as it is"]]);
        same("record asks", ByEar.asked(idle[0]), { ask: "act", act: "record" });

        // Recording counts down from `until` in whole seconds, never below
        // none, and Return on it asks nothing.
        const recording = record({ stage: "recording", until: 10000 });
        same("counts", says(ByEar.rows(said, recording, "answering", 4200)),
            [["sound:by-ear:recording", "", "6 s left",
                "speak as you would on a call, with the room as it is"]]);
        same("last second", ByEar.left(recording, 9999), 1);
        same("ended", ByEar.left(recording, 12000), 0);
        same("recording asks nothing",
            ByEar.asked(ByEar.rows(said, recording, "answering", 4200)[0]), null);

        same("cleaning", says(ByEar.rows(said, record({ stage: "cleaning" }), "answering", 0)),
            [["sound:by-ear:cleaning", "", "cleaning",
                "with each cleaner, then shuffled as clips A to D"]]);

        // Ready: Record Again where Record stood, then a row a clip, A to D;
        // Return plays one, and on the one playing plays it again, named
        // so; Pick is a step in.
        const ready = record({ stage: "ready", clips: ["A", "B", "C", "D"], playing: "A" });
        const shown = ByEar.rows(said, ready, "answering", 0);
        same("record again", [shown[0].id, shown[0].state, ByEar.asked(shown[0])],
            ["sound:by-ear:record", "again", { ask: "act", act: "record" }]);
        const clips = shown.slice(1);
        same("clips", clips.map(row => [row.fill.clip, row.state, row.side]),
            [["A", "playing", "playing"], ["B", "", ""], ["C", "", ""], ["D", "", ""]]);
        same("plays", ByEar.asked(clips[1]), { ask: "act", act: "play", clip: "B" });
        same("again plays", ByEar.asked(clips[0]), { ask: "act", act: "play", clip: "A" });
        same("picks", ByEar.asked(clips[1].pick), { ask: "act", act: "pick", clip: "B" });
        same("pick steps back", clips[1].pick.back, true);
        same("play stays", clips[1].back ?? false, false);
        same("blind", clips.every(row => !row.side.includes("RNNoise")), true);

        // Where the clips play, once the daemon says, on every clip, so Joel
        // knows which speaker or headset to listen on before he plays one.
        const heard = ByEar.rows(said, Object.assign({}, ready, { output: "Pebble V3" }),
            "answering", 0).slice(1);
        same("plays on", heard.map(row => [row.fill.clip, row.side]),
            [["A", "playing on Pebble V3"], ["B", "on Pebble V3"], ["C", "on Pebble V3"],
                ["D", "on Pebble V3"]]);

        // Picked: what was picked and that calls use it from now, then each
        // clip's cleaner and its cost as a share of a core.
        const picked = record({
            stage: "picked", microphone: "Shure MV7",
            picked: { clip: "B", cleaner: "deepfilternet-gentle" },
            revealed: {
                C: "deepfilternet", A: "rnnoise", B: "deepfilternet-gentle", D: "raw"
            },
            costs: { raw: 0, rnnoise: 0.012, deepfilternet: 0.31, "deepfilternet-gentle": 0.3 }
        });
        same("picked", says(ByEar.rows(said, picked, "answering", 0)), [
            ["sound:by-ear:picked", "", "B \u00b7 DeepFilterNet, gentle",
                "calls hear Shure MV7 through it from now"],
            ["sound:by-ear:clip", "A", "RNNoise", "1.20% of a core while cleaning"],
            ["sound:by-ear:clip", "B", "DeepFilterNet, gentle", "30.0% of a core while cleaning"],
            ["sound:by-ear:clip", "C", "DeepFilterNet, full", "31.0% of a core while cleaning"],
            ["sound:by-ear:clip", "D", "no noise cancelling", "0% of a core while cleaning"]
        ]);
        // Once scored, each clip says how it sounds overall, out of 5 and to
        // one place, before its cost; a cleaner the scorer missed says only
        // its cost.
        const scored = record(Object.assign({}, picked, {
            marks: {
                raw: { voice: 3.29, background: 3.26, overall: 2.68 },
                rnnoise: { voice: 3.34, background: 3.79, overall: 2.97 },
                deepfilternet: { voice: 3.42, background: 4.04, overall: 3.153 }
            }
        }));
        same("scored", says(ByEar.rows(said, scored, "answering", 0)).slice(1), [
            ["sound:by-ear:clip", "A", "RNNoise",
                "sounds 3.0 of 5 \u00b7 1.20% of a core while cleaning"],
            ["sound:by-ear:clip", "B", "DeepFilterNet, gentle", "30.0% of a core while cleaning"],
            ["sound:by-ear:clip", "C", "DeepFilterNet, full",
                "sounds 3.2 of 5 \u00b7 31.0% of a core while cleaning"],
            ["sound:by-ear:clip", "D", "no noise cancelling",
                "sounds 2.7 of 5 \u00b7 0% of a core while cleaning"]
        ]);

        // Each row keeps its key from step to step, so the launcher changes
        // it in place: where the trial stands first, then each clip.
        const keys = rows => rows.map(row => row.key);
        same("one head", [idle, ByEar.rows(said, recording, "answering", 0),
            ByEar.rows(said, record({ stage: "cleaning" }), "answering", 0),
            ByEar.rows(said, record({ stage: "refused", why: "a call is on" }), "answering", 0)]
            .map(rows => keys(rows)), [["sound:by-ear:now"], ["sound:by-ear:now"],
            ["sound:by-ear:now"], ["sound:by-ear:now"]]);
        same("clips keep their keys", keys(ByEar.rows(said, picked, "answering", 0)),
            keys(shown));
        same("revealed asks nothing",
            ByEar.rows(said, picked, "answering", 0).map(row => ByEar.asked(row)),
            [null, null, null, null, null]);

        // A raw pick says nothing changes, naming the microphone plainly
        // where the record doesn't.
        const raw = record({
            stage: "picked", picked: { clip: "D", cleaner: "raw" },
            revealed: { D: "raw" }, costs: { raw: 0 }
        });
        same("raw pick", ByEar.rows(said, raw, "answering", 0)[0].note,
            "nothing changes: calls hear the microphone as before");

        // Refused says its reason plainly, with nothing to do.
        const refused = ByEar.rows(said, record({ stage: "refused", why: "a call is on" }),
            "answering", 0);
        same("refused", says(refused), [["sound:by-ear:refused", "", "a call is on", ""]]);
        same("refused asks nothing", ByEar.asked(refused[0]), null);

        // With nothing held, one row says how the daemon stands.
        same("down", says(ByEar.rows(said, null, "not answering", 0)),
            [["sound:by-ear:down", "", "not answering", "its socket starts it on the next ask"]]);
        same("starting row", says(ByEar.rows(said, null, "starting", 0)),
            [["sound:by-ear:down", "", "starting", ""]]);
        // Its Return starts the daemon, since asking its socket does, and
        // asks the daemon nothing, there being none to ask.
        same("down starts it", ["not answering", "starting"].map(state => ((row) =>
            [row.starts, ByEar.asked(row)])(ByEar.rows(said, null, state, 0)[0])),
            [[true, null], [true, null]]);

        // Where Joel is, once the daemon says, second at every step: the
        // laptop's name for it until he gives it his own, Return asking him
        // for one, and then his, to rename. What he types goes to the daemon
        // as the name act, which trims and judges it.
        const here = { spot: "Main Floor", spot_named: false };
        const unnamed = ByEar.rows(said, record(here), "answering", 0);
        same("spot unnamed", says(unnamed)[1], ["sound:by-ear:spot", "", "Main Floor",
            "the laptop's name for here; give it yours, like the desk"]);
        same("spot asks for words", [unnamed[1].state, unnamed[1].names,
            ByEar.asked(unnamed[1])], ["", true, null]);
        same("spot named", ((row) => [row.state, row.side, row.note])(ByEar.rows(said,
            record({ spot: "the desk", spot_named: true }), "answering", 0)[1]),
            ["named", "the desk", "picks here are kept under it, and stay when it's renamed"]);
        same("names", ByEar.naming("the couch"), { ask: "act", act: "name", name: "the couch" });
        same("spot keeps its key", ["idle", "recording", "cleaning", "refused"].map(stage =>
            keys(ByEar.rows(said, record(Object.assign({ stage, why: "a call is on" }, here)),
                "answering", 0))), [["sound:by-ear:now", "sound:by-ear:spot"],
            ["sound:by-ear:now", "sound:by-ear:spot"], ["sound:by-ear:now", "sound:by-ear:spot"],
            ["sound:by-ear:now", "sound:by-ear:spot"]]);
        same("spot before the clips", keys(ByEar.rows(said, Object.assign({}, ready, here),
            "answering", 0)), ["sound:by-ear:now", "sound:by-ear:spot"]
            .concat(["A", "B", "C", "D"].map(clip => `sound:by-ear:clip:${clip}`)));
        same("spot before what was picked", keys(ByEar.rows(said,
            Object.assign({}, picked, here), "answering", 0)), ["sound:by-ear:now",
            "sound:by-ear:spot"].concat(["A", "B", "C", "D"].map(clip =>
            `sound:by-ear:clip:${clip}`)));
        same("no spot said", keys(idle), ["sound:by-ear:now"]);

        // Record records the input Choose by Ear is opened under, by its node,
        // the default or not, again too; opened for none, the default.
        const aimed = (laid, at) => ByEar.asked(ByEar.aimed(laid[at], "mv7"));
        same("record aims", aimed(ByEar.rows(said, record({}), "answering", 0), 0),
            { ask: "act", act: "record", microphone: "mv7" });
        same("record again aims", aimed(ByEar.rows(said, ready, "answering", 0), 0),
            { ask: "act", act: "record", microphone: "mv7" });
        same("a clip aims nothing", aimed(ByEar.rows(said, ready, "answering", 0), 1),
            { ask: "act", act: "play", clip: "A" });

        // The offer the daemon makes after a call on a microphone at a spot
        // never picked for there, while it stands and is whole; taking it or
        // putting it off asks the same act, and why it's there is the new
        // microphone's or the new spot's words.
        const offered = { microphone: "Shure MV7", node: "mv7", spot: "the desk",
            key: "home=6366013", new: "spot", since: 5000 };
        same("an offer", ByEar.offer(record({ offer: offered })), offered);
        same("no offer", ByEar.offer(record({ offer: null })), null);
        same("an offer with no node", ByEar.offer(record({
            offer: { microphone: "Shure MV7", spot: "the desk" } })), null);
        same("no record, no offer", ByEar.offer(null), null);
        same("taking", ByEar.taking(), { ask: "act", act: "offered" });
        const tells = { "why-microphone": "{microphone} is new",
            "why-spot": "{microphone} is new at {spot}",
            "why-noisier": "{spot} is noisier for {microphone}",
            "why-quieter": "{spot} is quieter for {microphone}" };
        same("a new spot", ByEar.offerWhy(tells, offered), "Shure MV7 is new at the desk");
        same("a new microphone", ByEar.offerWhy(tells,
            Object.assign({}, offered, { new: "microphone" })), "Shure MV7 is new");
        // A call far noisier or quieter than the pick was made in, by the
        // dB the daemon measured, louder above nought.
        same("a noisier room", ByEar.offerWhy(tells,
            Object.assign({}, offered, { new: "noise", by: 12 })),
            "the desk is noisier for Shure MV7");
        same("a quieter room", ByEar.offerWhy(tells,
            Object.assign({}, offered, { new: "noise", by: -11 })),
            "the desk is quieter for Shure MV7");
        same("a noise offer is whole", ByEar.offer(record({
            offer: Object.assign({}, offered, { new: "noise", by: 12 }) })),
            Object.assign({}, offered, { new: "noise", by: 12 }));

        // Leaving discards anything but idle, and asks nothing of an idle
        // trial or a daemon not there.
        same("leaving idle", ByEar.leaving(record({})), null);
        same("leaving nothing", ByEar.leaving(null), null);
        same("leaving ready", ByEar.leaving(ready), { ask: "act", act: "discard" });
        same("leaving recording", ByEar.leaving(recording), { ask: "act", act: "discard" });
        same("leaving picked", ByEar.leaving(picked), { ask: "act", act: "discard" });
    }
}
