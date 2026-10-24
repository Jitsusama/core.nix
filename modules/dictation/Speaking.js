// What dictation's surface says (Speaking.qml), worked out apart from what
// draws it so tests/speaking.qml can hold each state to its words in Qt's
// own engine.
.pragma library

// Whisper hears at most thirty seconds, and the service stops listening
// there, so from twenty-five the time counts down what's left instead.
const SECOND = 1000;
const LIMIT = 30e3;
const WARNED = 25e3;
const MINUTE = 60;

// Why the service says it heard nothing, by the word after heard-nothing.
const NOTHING = ["short", "silent", "unclear"];

// A face with no time and no wave: typing, or a moment's answer, said
// muted when quiet.
function plain(glyph, hue, words, quiet) {
    return { glyph, hue, words, time: "", wave: false, quiet };
}

// How long it's listened, as a clock reads it, or, near the end, the
// seconds left, rounded up so the last says one.
function clock(elapsed) {
    if (elapsed >= WARNED) {
        return `${Math.max(0, Math.ceil((LIMIT - elapsed) / SECOND))} s`;
    }
    const seconds = Math.floor(elapsed / SECOND);
    return `${Math.floor(seconds / MINUTE)}:${String(seconds % MINUTE).padStart(2, "0")}`;
}

// The surface's face: its glyph by name, the glyph's colour role, its
// words, the time, whether the wave shows, and whether it's a moment's
// answer, said muted. Null shows nothing. What the service said after a
// state wins over it, since it's about the state just ending. Typing
// keeps listening's face until it has lasted long enough to see, so a
// quick one never flickers.
function said(state, elapsed, after, typingSeen) {
    if (after === "still-writing") {
        return plain("dictating", "voice", "still writing", false);
    }
    if (after === "muted") {
        return plain("muted", "machine", "the microphone is muted", true);
    }
    if (NOTHING.includes(after)) {
        return plain("dictating", "muted", "heard nothing", true);
    }
    if (state === "listening" || (state === "transcribing" && !typingSeen)) {
        return {
            glyph: "dictating",
            hue: elapsed >= WARNED ? "warning" : "voice",
            words: "listening",
            time: clock(elapsed),
            wave: true,
            quiet: false
        };
    }
    if (state === "transcribing") {
        return plain("dictating", "voice", "typing", false);
    }
    return null;
}

// The wave's bars with a peak heard in a step of time: the loudest in a
// step stands for it, a step nothing was heard in is silent, and only
// the last count steps are kept. A step before the last starts again,
// as a new listen does.
function waved(levels, peak, step, count) {
    const [last] = levels.slice(-1);
    if (last !== undefined && last.step === step) {
        return levels.slice(0, -1).concat([{ step, level: Math.max(last.level, peak) }]);
    }
    if (last === undefined || step < last.step) {
        return [{ step, level: peak }];
    }
    const from = Math.max(last.step + 1, step - count + 1);
    const silent = [];
    for (let quiet = from; quiet < step; quiet += 1) {
        silent.push({ step: quiet, level: 0 });
    }
    return levels.concat(silent, [{ step, level: peak }]).slice(-count);
}

// A bar's height for a level, from least to most. Speech peaks low, so
// the height follows the level's square root, a quarter reaching half.
function height(level, least, most) {
    return least + (most - least) * Math.sqrt(Math.min(1, Math.max(0, level)));
}
