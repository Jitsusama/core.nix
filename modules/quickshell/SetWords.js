// The command line's :set, read as Vim reads its options (:help :set):
// each word an option turned on or off, set, moved by so much or said,
// against the options the launcher's providers give (Provider.qml's
// setOptions). Apart from the shell, so tests/command-line.qml can hold it
// in Qt's own engine beside the line it's typed on (Commands.js).
.pragma library
.import "Commands.js" as Commands

const space = /\s/;
const number = /^[0-9]+$/;
// A place in time given as a clock, m:ss or h:mm:ss, as a track's position
// is (`clock` options, the Media place's position=1:30).
const clockTime = /^(?:([0-9]+):)?([0-9]+):([0-5][0-9])$/;
const MINUTE = 60;
const HOUR = 3600;

// :set's words, split at spaces as Vim splits them, a backslash keeping
// the letter after it, so :set wifi=Home\ 5G names one network.
function setWords(arg) {
    const said = [];
    let word = "";
    let escaped = false;
    for (const letter of arg) {
        if (escaped) {
            word += letter;
            escaped = false;
        } else if (letter === "\\") {
            escaped = true;
        } else if (space.test(letter)) {
            if (word !== "") {
                said.push(word);
            }
            word = "";
        } else {
            word += letter;
        }
    }
    return word === "" ? said : said.concat([word]);
}

const setWord = /^(no|inv)?([a-z]+)(!|\?|\+=|-=|=)?([\s\S]*)$/;

// One of :set's words split into what it says: no or inv, the option's
// name, how it's set and the value, or null where it names no option. An
// option whose own name starts with no or inv is that option.
function setRead(word, options) {
    const found = word.match(setWord);
    if (!found) {
        return null;
    }
    const [, said = "", called, how = "", value = ""] = found;
    const whole = said !== "" && options[said + called] !== undefined;
    const name = whole ? said + called : called;
    const option = options[name];
    const given = how === "=" || how === "+=" || how === "-=";
    if (!option || (!given && value !== "")) {
        return null;
    }
    return { prefix: whole ? "" : said, name, how, value, option };
}

// A value given an option with =, += or -=: a change, or Vim's error. A
// switch whose value is a number (`takes` "number"), as dnd's minutes, reads
// it as a number option does.
function setValue(word, read) {
    const { name, how, value, option } = read;
    const invalid = { error: `E474: Invalid argument: ${word}` };
    const numbered = option.kind === "number" || option.takes === "number";
    if ((how !== "=" && !numbered) || (option.kind === "switch" && !option.takes)) {
        return invalid;
    }
    if (!numbered) {
        const choices = option.values ?? null;
        return choices !== null && !choices.includes(value) ? invalid
            : { option: name, how: "set", value };
    }
    return setNumber(word, read, invalid);
}

// A number given with =, += or -=, as volume=40 or dnd+=60: a change, or
// Vim's error where it's no number, = sets it past its ends, or it's only
// stepped (`stepsOnly`), as a board's lights that take no level.
function setNumber(word, read, invalid) {
    const { name, how, value, option } = read;
    const clock = option.clock === true ? clockTime.exec(value) : null;
    if (!(number.test(value) || clock)) {
        return { error: `E521: Number required after =: ${word}` };
    }
    if (how === "=" && option.stepsOnly) {
        return invalid;
    }
    const by = clock
        ? Number(clock[1] ?? 0) * HOUR + Number(clock[2]) * MINUTE + Number(clock[3])
        : Number(value);
    if (how === "=" && (by < (option.least ?? 0) || by > (option.most ?? by))) {
        return invalid;
    }
    const hows = { "=": "set", "+=": "add", "-=": "subtract" };
    return { option: name, how: hows[how], value: by };
}

// One of :set's words, against the option it names: a change, or Vim's
// error for it.
function setOne(word, options) {
    const read = setRead(word, options);
    if (read === null) {
        return { error: `E518: Unknown option: ${word}` };
    }
    const { prefix, name, how } = read;
    const switches = read.option.kind === "switch";
    const flipped = prefix !== "" || how === "!";
    if (flipped && (!switches || (how !== "" && how !== "!"))) {
        return { error: `E474: Invalid argument: ${word}` };
    }
    if (flipped) {
        const off = prefix === "no" && how === "";
        return { option: name, how: off ? "off" : "toggle", value: "" };
    }
    if (how === "?" || (how === "" && !switches)) {
        return { option: name, how: "show", value: "" };
    }
    return how === "" ? { option: name, how: "on", value: "" } : setValue(word, read);
}

// What :set's words ask, read against `options`, each an option's kind: a
// switch (on, off, or with `takes` a value, as :set wifi=Home-5G joins a
// network), a number between its least and most, or a choice among its
// values. Each word is on, off, toggle, set, add, subtract or show; all,
// or nothing after :set, shows every one. { changes: [{ option, how,
// value }] }, or { error } in Vim's words for the first word it can't
// take, so none of them changes.
function setting(arg, options) {
    const words = setWords(arg);
    if (words.length === 0) {
        return { changes: [{ option: "all", how: "show", value: "" }] };
    }
    const changes = [];
    for (const word of words) {
        const change = word === "all" ? { option: "all", how: "show", value: "" }
            : setOne(word, options);
        if (change.error !== undefined) {
            return change;
        }
        changes.push(change);
    }
    return { changes };
}

// What a change sets the option to, from where it stands now: { to }, or
// { error } for one that can't change while the shell runs (`fixed`), as
// the theme's background, which a switch of the system sets. A switch
// stands as true, false, or with `takes` the value it holds, as the
// network joined; a number is held within its least and most.
function toward(change, now, option) {
    if (option.fixed) {
        const said = change.how === "set" ? `${change.option}=${change.value}` : change.option;
        return { error: `E519: Option not supported: ${said}` };
    }
    switch (change.how) {
    case "on":
        return { to: true };
    case "off":
        return { to: false };
    case "toggle":
        return { to: now === false };
    case "add":
    case "subtract":
        return { to: moved(change, now, option) };
    default:
        return { to: change.value };
    }
}

// A number moved by += or -=, held within its least and most, or still
// null where it can't be read, the option's set stepping it by the change.
// A switch whose value is a number, on with none, as dnd held until it's
// ended, stays so, and off, += gives it that many, as dnd+=60 holds an
// hour.
function moved(change, now, option) {
    const adds = change.how === "add";
    if (now === null) {
        return null;
    }
    if (typeof now === "boolean") {
        return adds && !now ? change.value : now;
    }
    const to = Number(now) + (adds ? change.value : -change.value);
    return Math.max(option.least ?? to, Math.min(option.most ?? to, to));
}

// One option as Vim's :set volume? says it: a switch its name or its no,
// or its value where it holds one, as the network it's joined; null for
// one that can't be read, for the launcher to say so.
function shown(name, now, option) {
    if (now === null) {
        return null;
    }
    if (option.kind !== "switch") {
        return `${name}=${now}`;
    }
    if (now === false) {
        return `no${name}`;
    }
    return now === true ? name : `${name}=${now}`;
}

// What an option's answer that comes after the line is done does with
// its words, as a screen's setting answers once niri has (Launcher.qml's
// commandSetSteps): { say } while the prompt still holds the keys that
// ran it, so a refusal is said where the line was; { journal } once Joel
// has typed on, so a late one never writes over what he's doing, said
// in the journal instead; and null where it has no words.
function later(option, words, ranAt, keysNow) {
    if (words === "") {
        return null;
    }
    return keysNow === ranAt ? { say: words }
        : { journal: `launcher: ${option} refused after Joel typed on: ${words}` };
}

const escapedSpace = /\s/g;

// :set's arguments, as a space after it lists them: the options, in name
// order, each with where it stands, or after one's = the values it takes,
// its choices or what its `choices` gives, a space in one escaped as Vim
// escapes it. A choice given as { value, note } says its note beside it,
// as a screen's rotation=upright says "upright" is its row's words. A
// number has nothing to choose.
function setArguments(arg, options) {
    const word = Commands.lastWord(arg);
    const equals = word.indexOf("=");
    if (equals < 0) {
        return Object.keys(options).sort().filter(each => each.startsWith(word))
            .map(each => ({
                value: each,
                note: shown(each, options[each].now(), options[each]) ?? ""
            }));
    }
    const name = word.slice(0, equals);
    const option = options[name];
    if (!option) {
        return [];
    }
    const values = option.values ?? (option.choices ? option.choices() : []);
    const given = word.slice(equals + 1).toLowerCase();
    return values.map(each => typeof each === "string" ? { value: each, note: "" } : each)
        .map(each => ({
            value: `${name}=${each.value.replace(escapedSpace, "\\ ")}`,
            note: each.note ?? ""
        }))
        .filter(each => each.value.slice(equals + 1).toLowerCase().startsWith(given));
}
