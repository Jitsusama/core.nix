// What the launcher's Choose by Ear works out apart from drawing: which
// step it's at, what each of its rows says and what Return on one asks
// the noise-cancelled daemon, from the daemon's one `trial:now` record
// (the trial protocol, QEST-20261111-JCWNFD's trial-protocol.md), so a
// check can run it in Qt's own engine (tests/by-ear.qml). Sound.qml
// draws the rows; NoiseCancelling.qml holds the record.
//
// A row here is plain: { key, id, state, fill, side, note, act, pick,
// back }, `id` the vocabulary's thing it's named by, with `state` and
// `fill` handed to Words.name, `act` what Return asks ({ act, clip }) or
// null for a figure, `pick` the act a step inside a clip holds, and
// `back` whether Return then steps back out. Everything said comes from
// `said`, the sound place's words, so nothing here is worded.
//
// The launcher keeps a list steady under the cursor, a row gone staying
// dimmed and a new one coming at the end until a key makes it afresh
// (Steady.js), so the step is laid out to change in place: one row
// first, by one key, saying where the trial stands (Record, Recording,
// Cleaning, Record Again, Picked), and the clips after it, each keeping
// its key from blind to revealed.
.pragma library
.import "Numbers.js" as Numbers

// The stages the record goes through, as the protocol names them.
const STAGES = ["idle", "recording", "cleaning", "ready", "picked", "refused"];
// The cleaners, as the record names them; a cleaner the record names
// that isn't one of these is said by its own name.
const CLEANERS = ["raw", "deepfilternet", "deepfilternet-gentle", "rnnoise"];
// The cleaner that cancels nothing: picking it changes nothing.
const RAW = "raw";
const A_SECOND = 1000;

// The record, if it's whole enough to draw from: its stage one the
// protocol has. Anything else is drawn as nothing held.
function whole(record) {
    return record !== null && typeof record === "object" && STAGES.includes(record.stage)
        ? record : null;
}

// What the vocabulary's words say for `id`, with what's in braces filled.
function told(said, id, fill) {
    let words = said[id] ?? id;
    for (const key of Object.keys(fill ?? {})) {
        words = words.replace(`{${key}}`, fill[key]);
    }
    return words;
}

// A cleaner as Joel reads it: "DeepFilterNet, gentle".
function cleaner(said, name) {
    return CLEANERS.includes(name) ? told(said, `cleaner-${name}`) : String(name ?? "");
}

// A cleaner's cost, a share of one core, as the launcher says a share
// (Numbers.significant): 0.012 as "1.20% of a core while cleaning".
function cost(said, share) {
    const figure = typeof share === "number" && share > 0
        ? `${Numbers.significant(share * 100)}%` : "0%";
    return told(said, "by-ear-cost", { share: figure });
}

// What a clip's row says once picked: how its cleaner sounds overall to
// the scorer, out of 5 to one place, before what it costs; only the cost
// where the scorer gave it no mark, as before the marks arrive.
function scored(said, marks, share) {
    const spent = cost(said, share);
    const overall = marks?.overall;
    return typeof overall === "number"
        ? told(said, "by-ear-scored", { mark: overall.toFixed(1), cost: spent }) : spent;
}

// The whole seconds left of a recording at `now`, counted up so the
// last second reads 1 until it ends, never below none.
function left(record, now) {
    if (typeof record?.until !== "number") {
        return 0;
    }
    return Math.max(0, Math.ceil((record.until - now) / A_SECOND));
}

// What the Noise Cancelling part says: the cleaner the microphones run
// now, or, with nothing held, how the daemon stands ("not answering").
function running(said, record, state) {
    return record === null ? daemonSays(said, state) : cleaner(said, record.running);
}

// How the daemon stands, in the sound place's words: "starting" until it
// first answers, then "not answering" while it's gone (Daemon.qml).
function daemonSays(said, state) {
    return state === "not answering" ? told(said, "down") : told(said, "by-ear-starting");
}

// What Choose by Ear's own row says beside it: how the trial stands, or
// nothing while it's idle.
function standing(said, record) {
    switch (record?.stage) {
    case "recording":
        return told(said, "by-ear-recording");
    case "cleaning":
        return told(said, "by-ear-cleaning");
    case "ready":
        return told(said, "by-ear-ready");
    case "picked":
        return told(said, "by-ear-picked", { clip: record.picked?.clip ?? "" });
    default:
        return "";
    }
}

// The microphone recorded, as calls hear it, or the plain word for one.
function microphone(said, record) {
    return typeof record.microphone === "string" && record.microphone !== ""
        ? record.microphone : told(said, "by-ear-the-microphone");
}

// The key the row saying where the trial stands keeps, whatever it says.
const HEAD = "sound:by-ear:now";

// The rows of Choose by Ear's step, in the order Return meets them, from
// the record at `now`, with the daemon `state` for when none is held:
// where the trial stands, then the clips.
function rows(said, record, state, now) {
    if (record === null) {
        return [{
            key: HEAD, id: "sound:by-ear:down", side: daemonSays(said, state),
            note: state === "not answering" ? told(said, "by-ear-down") : "", act: null
        }];
    }
    switch (record.stage) {
    case "ready":
        return [recordRow(said, record, "again")]
            .concat((record.clips ?? []).map(clip => clipRow(said, record, clip)));
    case "picked":
        return pickedRows(said, record);
    default:
        return [head(said, record, now)];
    }
}

// Where a trial with no clips stands: Record, its count, its cleaning, or
// the daemon's reason for refusing, said plainly with nothing to do.
function head(said, record, now) {
    switch (record.stage) {
    case "recording":
        return {
            key: HEAD, id: "sound:by-ear:recording",
            side: told(said, "by-ear-left", { seconds: left(record, now) }),
            note: told(said, "by-ear-speak"), act: null
        };
    case "cleaning":
        return {
            key: HEAD, id: "sound:by-ear:cleaning",
            side: told(said, "by-ear-cleaning"), note: told(said, "by-ear-each"), act: null
        };
    case "refused":
        return {
            key: HEAD, id: "sound:by-ear:refused", side: String(record.why ?? ""),
            note: "", act: null
        };
    default:
        return recordRow(said, record, "");
    }
}

// Record, whose Return asks the daemon to record; with clips to hear, it
// records afresh, named so.
function recordRow(said, record, state) {
    return {
        key: HEAD, id: "sound:by-ear:record", state,
        side: told(said, "by-ear-seconds", { seconds: record.seconds ?? 0 }),
        note: told(said, "by-ear-speak"), act: { act: "record" }
    };
}

// Where a clip plays, once the daemon says: the output it plays on, on
// every clip, so Joel knows which to listen on before he plays one.
function where(said, record, playing) {
    const output = typeof record.output === "string" && record.output !== ""
        ? record.output : null;
    if (output === null) {
        return playing ? told(said, "by-ear-playing") : "";
    }
    return told(said, playing ? "by-ear-playing-on" : "by-ear-on", { output });
}

// A clip, heard blind: Return plays it, or plays it again from the start
// while it's the one playing, and a step in holds Pick.
function clipRow(said, record, clip) {
    const playing = record.playing === clip;
    return {
        key: `sound:by-ear:clip:${clip}`,
        id: "sound:by-ear:clip",
        state: playing ? "playing" : "",
        fill: { clip },
        side: where(said, record, playing),
        note: told(said, "by-ear-blind"),
        act: { act: "play", clip },
        // Picking drops the clips, so Pick steps back out to the trial,
        // onto the clip it was, which then says its cleaner.
        pick: {
            key: `sound:by-ear:pick:${clip}`, id: "sound:by-ear:pick",
            act: { act: "pick", clip }, back: true
        }
    };
}

// Once one's picked: what was picked and what follows, then every clip
// with its cleaner revealed, how it sounds once scored, and what that
// cleaner costs.
function pickedRows(said, record) {
    const chosen = record.picked?.cleaner ?? "";
    const mic = microphone(said, record);
    const picked = {
        key: HEAD,
        id: "sound:by-ear:picked",
        side: told(said, "by-ear-chosen", {
            clip: record.picked?.clip ?? "", cleaner: cleaner(said, chosen)
        }),
        note: told(said, chosen === RAW ? "by-ear-unchanged" : "by-ear-from-now",
            { microphone: mic }),
        act: null
    };
    const revealed = record.revealed ?? {};
    const costs = record.costs ?? {};
    const marks = record.marks ?? {};
    const clips = Object.keys(revealed).sort();
    return [picked].concat(clips.map(clip => ({
        key: `sound:by-ear:clip:${clip}`,
        id: "sound:by-ear:clip",
        state: "",
        fill: { clip },
        side: cleaner(said, revealed[clip]),
        note: scored(said, marks[revealed[clip]], costs[revealed[clip]]),
        act: null
    })));
}

// What leaving the step, by Escape or by closing, asks the daemon: to
// discard anything but idle, so no clip stays in its memory once Joel's
// gone, and the next opening starts afresh; nothing while it's idle.
function leaving(record) {
    return record !== null && record.stage !== "idle" ? { ask: "act", act: "discard" } : null;
}

function naming(words) {
    return null;
}

// What Return on a row asks the daemon, as the protocol's act is sent:
// { ask: "act", act, clip }, or null for a figure, which asks nothing.
function asked(row) {
    if (!row?.act) {
        return null;
    }
    return Object.assign({ ask: "act" }, row.act);
}
