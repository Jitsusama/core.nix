// The launcher on Super+Space, the desktop's command palette: type what you
// want, a window, an application, an action or what a key does, pick it
// with the arrows, and Return opens it, or lands on it where it would
// change something. The line is modal, as Vim's / line is (Prompt.qml,
// QueryKeys.js): it opens on the find, where letters narrow the rows and
// Ctrl+Y takes the grey completion it shows, and Escape gives normal, where
// nothing types and the keys move through the rows as Vim's do through a
// buffer, with counts. Each action shows its chord, so keys are learned by
// use.
//
// It opens as fast as niri can hand it the keys. Super+Space says "toggle"
// down a socket the shell keeps open (launcher-say, in home.nix), not
// through a second Quickshell that has to start first. Its surface stays
// mapped all along, a single clear pixel that takes neither the keys nor
// the pointer while it's closed. Opening only grows it, gives it the keys
// and shows the card, whose field already holds the focus. A letter typed
// straight after the chord then reaches the field, not the window below.
//
// What it finds comes from its providers, one file each in launcher/, in
// the order jitsusama.quickshell.launcher.providers lists them (rendered
// into Providers.qml), so a kind of thing is added or removed in one line.
//
// Return goes into a place or a thing with acts, and opens a program, a
// window or a file; a row's other acts are its verbs as commands, on the
// : line. What's held shows as chips in the field; - goes up, onto the
// row come from, with its find. Escape leaves a line or cancels what's
// begun, as Vim's does, and at rest does nothing; :q, ZZ or Space w close
// it.
//
// What everything is called is the vocabulary's (Words.qml): its names,
// the words that find it, its parts, and the categories it's in. A
// category is a level too, found by its word, and its word and a space
// step into it, what's typed then searching only inside it; commands is
// the one listing every typed command, Tab putting its start in the field.
//
// What it finds is one row a thing, in an order that changes only when Joel
// changes it (launcher/Rank.js): the same letters land on the same row
// however the desktop changes around them, a list that changes while it's
// shown keeps its order (launcher/Steady.js), and the list keeps its height
// while it's open, so nothing moves under his eyes.
//
// It's the palette kind (docs/design.md, Shape and Kinds): the accent frame,
// since it holds the keys; a command line over a list, split by a hairline;
// every row a glyph in its column, a name, and its chord or what it is.
import Quickshell
import Quickshell.Io
import Quickshell.Services.UPower
import Quickshell.Wayland
import QtQuick
import KeyTimes
// Quickshell makes a folder's files types only once something imports it,
// and the providers are made from launcher/ by name, so it's imported here.
import qs.launcher
import "Bands.js" as Bands
import "Columns.js" as Columns
import "Commands.js" as Commands
import "Fuzzy.js" as Fuzzy
import "ListedKeys.js" as ListedKeys
import "Operators.js" as Operators
import "Finding.js" as Finding
import "Glide.js" as Glide
import "Confirm.js" as Confirm
import "Changes.js" as Changes
import "Hearing.js" as Hearing
import "QueryKeys.js" as QueryKeys
import "ModeMarks.js" as ModeMarks
import "Reading.js" as Reading
import "SetWords.js" as SetWords
import "CommandBinding.js" as CommandBinding
import "launcher/Catalogue.js" as Catalogue
import "launcher/Rank.js" as Rank
import "launcher/Showing.js" as Showing
import "launcher/Standing.js" as Standing
import "launcher/Steady.js" as Steady
import "launcher/RowsKept.js" as RowsKept
import "launcher/Becoming.js" as Becoming
import "launcher/Pacing.js" as Pacing
import "launcher/Presented.js" as Presented
import "launcher/Wanted.js" as Wanted
import "launcher/Order.js" as Order
import "launcher/PlaceRow.js" as PlaceRow
import "launcher/KeysHeld.js" as KeysHeld
import "launcher/RowsPlace.js" as RowsPlace

Scope {
    id: root

    required property var lock
    required property var grid
    property bool open: false
    // Whether the card is shown from what it kept while its first frame is
    // drawn: home as the last opening read it, each row saying how it stands
    // as still being read (keepHome). `open` follows once that frame is
    // drawn, and all that wakes as the launcher opens wakes then (filled).
    // Through a storm of windows changing, every core busy, an opening took
    // 81 to 258 ms to its frame in a VM, 25 to 48 of it the shell waking for
    // it before it could draw; on Joel's laptop one took 6,184 ms
    // (2026-11-10).
    property bool opening: false
    // Shown at all: open, or drawing its first frame from what it kept. For
    // bindings only: a handler of open's change can run before this has
    // followed it, so functions ask open || opening themselves.
    readonly property bool cardUp: open || opening
    // Whether it has let go of the keys while still drawn, as Return on a
    // row that acts at once does, so the window has them back in the next
    // frame and the closing's own work waits until after it (run).
    property bool lettingGo: false

    // Wide enough for a long action and its chord at the large size, or
    // as wide as a narrow screen gives, the margin the card keeps from the
    // screen's edge (drawerMargin) either side, so a narrow screen never
    // puts its right edge past the screen's.
    readonly property int width: !screenHere ? 680
        : Math.min(680, Math.floor(screenHere.width - 2 * drawerMargin))
    // How many results show at once; the rest are a scroll away. Eight,
    // or as many as keep the card inside a small screen, a screen margin
    // above and below it, three at least (VKCF3Z, 2026-11-05: panels
    // never run off the screen). A page's gap is allowed for whether it
    // shows or not, so what's found never changes how many.
    readonly property int rows: !screenHere ? 8 : Math.max(3, Math.min(8, Math.floor(
        (screenHere.height - 2 * Theme.space.rung.screen - cardChrome - Theme.space.beat)
            / rowTall)))
    // How tall the card is but for its rows: its frame, the line and its
    // hairline, the ties either side of the rows, and the foot and its
    // hairline, which no screen changes. Summed from its parts rather than
    // read off the card, whose height follows the rows a layout late, so
    // how many rows fit never reads itself (the net, 2026-11-10: Qt warned
    // of a binding loop for rows at bf0d237ec).
    readonly property real cardChrome: 2 * Theme.border + 2 * card.above + headTall
        + Theme.border + 2 * Theme.space.rung.tie + Theme.border + statusLine.tall
    // How tall the drawer stands in the card's place: down to the rows'
    // end, so the foot stays in sight under it, saying how to come back.
    readonly property real rowsEnd: Theme.border + headTall + Theme.border
        + Theme.space.rung.tie + listBox.height
    // The card's words but its foot, which give way to the drawer where it
    // takes their place, fading on the drawer's own curve as it comes.
    property real wordsShown: drawerOver && drawerHolds ? 0 : 1

    Behavior on wordsShown {
        NumberAnimation {
            duration: root.drawerHolds ? Theme.motion.medium : Theme.motion.short
            easing.type: Easing.BezierSpline
            easing.bezierCurve: Theme.motion.curve
        }
    }

    // The vocabulary (Words.qml), handed to the providers' common code
    // (Provider.qml), which can't import qs itself: Provider is the type
    // every provider is made from, and importing the shell's module there
    // made Machine's "what's running" answer nothing in the desktop test,
    // with no warning, as if its Jobs were another, unfed one.
    readonly property var vocabulary: Words
    // The theme's marks, handed over the same way, for a glyph Provider
    // gives of its own.
    readonly property var marks: Theme.glyph

    // The providers, made from their files as the shell starts, each given
    // its settings and weight; one that fails to load is said and skipped.
    property var sources: []

    Component.onCompleted: {
        // Made now, so it hears the first key pressed (key-times/).
        KeyTimes.now();
        const made = [];
        for (const listed of Providers.providers) {
            const file = Qt.createComponent(`launcher/${listed.file}`);
            if (file.status !== Component.Ready) {
                console.error(`The launcher's ${listed.name} couldn't load: ${file.errorString()}`);
                continue;
            }
            const source = file.createObject(root, {
                settings: listed,
                weight: listed.weight,
                launcher: root
            });
            source.answered.connect(() => root.reanswer(source));
            source.changed.connect(() => root.refreshed(source));
            source.foundMore.connect(root.foundMore);
            source.thingsChanged.connect(root.stale);
            // Counted, so a closing can say which sources changed since
            // the last (madeSaid).
            const name = listed.name;
            const count = () => {
                root.changesIn[name] = (root.changesIn[name] ?? 0) + 1;
            };
            source.thingsChanged.connect(count);
            source.answered.connect(count);
            source.changed.connect(count);
            made.push(source);
        }
        sources = made;
        freshen();
    }

    // What the first provider to answer makes of what's typed, or null.
    // The selection starts on its headline, at -1, above the rows.
    property var answer: null
    // What the search found, below an answer's rows.
    property var found: []

    function answering(letters, fresh) {
        if (held.length > 0 && category !== null)
            return answeringIn(category, letters);
        if (held.length > 0 || letters.trim() === "")
            return null;
        for (const source of sources) {
            const said = source.answer(letters);
            if (said)
                return heard(said, source, letters);
        }
        // Typed so far, a question's start asks it, as `question` says.
        const text = Fuzzy.plainly(letters);
        if (text.length < 3)
            return null;
        for (const source of sources) {
            const question = source.questions.find(asked => asked.startsWith(text)
                && !oldWords[asked]);
            if (question === undefined)
                continue;
            const said = started(source, question, fresh);
            if (said)
                return Object.assign(heard(said, source, question), { question: question });
        }
        return null;
    }

    // An answer as the launcher keeps it: who gave it, and to what words,
    // so it can be asked again while it's stepped into and say what it says
    // now. Made over the provider's own, rather than copied, so parts it
    // makes only once they're looked at (Machine.qml's fresh) still are.
    function heard(said, source, words) {
        return Object.assign(Object.create(said), { asker: source, heard: words });
    }

    // Whether an answer has parts, without making them: one whose parts are
    // made only once they're looked at says it has some.
    function parted(said) {
        return !!said && (said.parted === true || said.rows.length > 0);
    }

    // What each word asking an answer names it, as the vocabulary does, so
    // a step into an answer is named as the thing it is ("cpu" is
    // Processor), not as what was typed.
    // The names things had, as they're typed, each as it was written, so
    // a name had asks only whole ("sto" asks nothing of the disk) and its
    // answer says what it was once called.
    readonly property var oldWords: {
        const out = {};
        for (const entry of Object.values(Words.things))
            for (const old of entry.once ?? [])
                out[old.toLowerCase()] = old;
        return out;
    }

    readonly property var answerNames: {
        const out = {};
        for (const id of Object.keys(Words.things)) {
            const entry = Words.of(id);
            for (const asked of entry.answers)
                out[asked] = entry.name;
        }
        return out;
    }

    // The question the answer was asked by when what's typed only starts
    // it, as "batt" starts "battery", or nothing. Its headline is chosen
    // only when nothing found starts a word the same way (compose), and
    // Tab finishes the question.
    readonly property string question: answer ? answer.question ?? "" : ""

    // Each letter of a question's start asks the same question, so its
    // answer is kept a moment rather than read again, as sensors are, on
    // every key. A late answer is always asked afresh.
    property var lastStarted: null
    function started(source, question, fresh) {
        const now = Date.now();
        const last = lastStarted;
        if (fresh && last && last.source === source && last.question === question && now - last.at < 2000)
            return last.said;
        const said = source.answer(question);
        lastStarted = { source: source, question: question, said: said, at: now };
        return said;
    }

    // Whether a name has a word starting with what's typed, as a row the
    // search found for that reason would.
    function startsAWord(name, letters) {
        const text = Fuzzy.plainly(letters);
        const folded = String(name).toLowerCase();
        for (let at = folded.indexOf(text); at >= 0; at = folded.indexOf(text, at + 1))
            if (at === 0 || !/[a-z0-9]/.test(folded[at - 1]))
                return true;
        return false;
    }

    // An answer with detail is one row above what was found, named by what
    // it says, its detail a step in, so a question never lists a column of
    // bare figures. A sum's, with none, is said in the field. Asked afresh,
    // the selection goes to the top; when a late answer arrives, a
    // selection already moved stays on the row it was on.
    function compose(fresh) {
        const composing = Date.now();
        const before = matches.length > 0 && matches[0].answered ? 1 : 0;
        const said = answering(typed, fresh);
        const askedAll = Date.now();
        const listed = root.found.concat(stillActed(root.found));
        // An answer only there when nothing else is, as asking a model by a
        // question's first word is, gives way to anything found. A thing
        // found by its meaning is a guess below it, not a find: it comes
        // late, and taking the answer away then would move the row above it.
        answer = said && said.fallback
            && (listed.some(row => !row.category && !row.byMeaning) || gathered.length > 0)
            ? null : said;
        // An answer still being asked for holds its row from the keystroke,
        // saying so, and one that held a row keeps it until a key, even if
        // it comes to nothing: added only once it came, its row pushed
        // every row below it down a moment after typing, twice for the
        // weather (Joel, 2026-10-25: "it needs to be fucking stable").
        const holds = !!answer && (answer.asking === true || (!fresh && before === 1));
        // One that came to nothing is a row too, saying why once, where its
        // asking row was, rather than "= no forecast" in the field beside
        // the aside saying Open-Meteo didn't answer (VKCF3Z, 2026-11-02).
        const missed = !!answer && answer.missed === true;
        const rows = parted(answer) || holds || missed ? [answered(answer)] : [];
        // A category named as a thing is, or as an answer asks, gives way to
        // it, so "sound" is Sound and "weather" the weather; its word and a
        // space still step into it.
        // At home, every category is listed, to be browsed.
        const things = new Set(listed.filter(row => !row.category && !row.byMeaning)
            .map(row => Fuzzy.plainly(row.name)));
        const found = listed.filter(row => !row.category || atHome
            || !(things.has(Fuzzy.plainly(row.name)) || (answer && !answer.question)));
        // A thing shows once: a collection found whole stands for what's in
        // it, so neither its own rows, nor the things that live in it
        // (`inside` names it), nor a shortcut that opens it, whose chord its
        // row already draws, are listed beside it.
        const whole = new Set(found.filter(row => row.key.startsWith("collection:"))
            .map(row => row.opens));
        const once = row => row.key.startsWith("collection:")
            || !(whole.has(row.inside ?? "") || whole.has(row.opens ?? ""));
        // The row it was on, by what it is, so rows that come late, as a
        // collection's or an answer's do, never move it.
        const was = !fresh && current >= 0 && matches[current]
            ? Rank.identity(matches[current]) : "";
        // And a thing found more than one way is one row, as the one
        // with the most to say.
        // A program's own keys named as it is are the program's, a step in.
        const below = underFirst(Rank.keysHeld(Rank.once(Rank.placed(found.filter(once),
            gathered.filter(row => !whole.has(row.from.settings.name)), latch))));
        // Searched again only to say what's there now, it keeps the order
        // it's shown in (reshow), unless a place still being made is said
        // again in its own order (ordering).
        const placedAll = Date.now();
        // What ends a search at the top comes after it all, never counted as
        // found, so an answer given only when nothing is still shows.
        const ending = held.length === 0 && typed.trim() !== "" ? endings(typed) : [];
        list.keptLast = null;
        matches = live((steadying && !ordering ? steadied(rows.concat(below)) : rows.concat(below))
            .concat(ending));
        // An answer only started gives way to a name that starts the same,
        // its first word: "pow" is still Power Off. A later word starting
        // so is no name Joel was typing, and gave "batt" to a copy that
        // said "50% · on battery", above the battery's own answer.
        const meant = answer && !(answer.question && below.length > 0
            && String(below[0].name).toLowerCase().startsWith(Fuzzy.plainly(typed)));
        if (fresh || (current === -1 && !answer))
            current = meant ? (rows.length > 0 ? 0 : -1) : answer ? rows.length : 0;
        else if (current === -1 && rows.length > 0)
            current = 0;
        else if (current >= 0)
            current = Rank.pinned(matches, was, current + rows.length - before);
        composeParts = { asking: askedAll - composing, placing: placedAll - askedAll,
            showing: Date.now() - placedAll, kept: list.keptLast };
    }

    // The rows with what the first thing found by its name holds listed
    // under it, as a program found from the top lists its open tabs or its
    // folders (`under`): what it holds now first, what it has open and then
    // the first few of its parts, Calendar's today's meetings or Mail's
    // newest, as open windows are for a program, then its Recent, each in
    // its own order, its own row first so Return still does what it does
    // (VKCF3Z, 2026-11-07, c7d9e2bd). Only
    // at the top, for what's typed, and only the first, so a word that
    // names several lists one's.
    readonly property int underMost: 3
    function underFirst(rows) {
        const at = held.length > 0 || typed.trim() === "" ? -1
            : rows.findIndex(row => typeof row.under === "function" && !row.byWord
                && (row.tier ?? 1) <= 1);
        if (at < 0) {
            underListed = "";
            return rows;
        }
        // One found by itself as well stays where it was found, so nothing
        // is listed twice.
        const shown = new Set(rows.map(row => Rank.identity(row)));
        const parts = (partsOfRows[rows[at].key] ?? []).slice(0, underMost);
        const own = rows[at].under();
        const open = own.open.slice(0, underMost);
        const holds = open.concat(parts, own.recent.slice(0, underMost - open.length))
            .filter(row => !shown.has(Rank.identity(row)))
            .map(row => underIt(row, (rows[at].depth ?? 0) + 1));
        underListed = holds.length > 0 ? rows[at].key : "";
        return rows.slice(0, at + 1).concat(holds, rows.slice(at + 1));
    }
    // Which program lists what it holds under it now, by its key, so its
    // own say counts by kind rather than naming a row shown under it
    // (VKCF3Z, 2026-11-07: "a say never names what a row under it names").
    property string underListed: ""
    // A row listed under its program, a depth in, as a session sits under
    // its orchestrator: that alone ties it to the program, with no rule or
    // name repeated (VKCF3Z, 2026-11-07). A copy, so the row stays as it
    // is wherever else it shows, kept on the row itself, so one a source
    // keeps is copied once rather than at every key; a WeakMap crashed the
    // shell once its rows were collected (hiddenWords).
    function underIt(row, depth) {
        if (row.underCopy?.depth !== depth)
            row.underCopy = Object.assign({}, row, { depth: depth, underCopy: undefined });
        return row.underCopy;
    }

    // The rows sources end a search with (Provider.qml's last), each
    // knowing who gave it and for what, so it's asked for again when that
    // source changes.
    function endings(letters) {
        const ending = [];
        for (const source of sources) {
            const row = source.last(letters);
            if (row)
                ending.push(Object.assign({}, row, { endedBy: source, endedFor: letters }));
        }
        return ending;
    }

    // What the last composing spent its time on: the answers asked, the rows
    // placed among them, then the list shown, its rows drawn afresh and
    // everything that reads them; a typical key spent 4 ms on all three
    // together, with nothing to say which (launcher-trials.json, 2026-10-30).
    property var composeParts: null

    // An answer as a row: what it says as its name, Return copying it or
    // doing what it does, and its detail behind the right arrow, each part
    // copied the same way. Its kind says it's an answer.
    function answered(said) {
        // One whose parts wait to be looked at says its own glyph and hue,
        // and one that came to nothing reads as nothing found does.
        const first = said.missed === true
            ? { glyph: Theme.glyph.nothingFound, hue: "voice" }
            : said.parted === true ? {} : said.rows[0] ?? {};
        const asked = said.question ?? Fuzzy.plainly(said.heard ?? typed);
        return {
            // Nothing to learn: an answer is always first for its question.
            key: "",
            is: "answer",
            answered: true,
            nested: parted(said),
            // One whose parts are read only as they're looked at, as
            // Machine's CPU lists the processes, which the walk for where a
            // row lives never makes (Catalogue.js's followsInto).
            parted: said.parted === true,
            // Named by what it says, not by what was typed, so nothing in
            // it is marked as matched.
            plain: true,
            name: said.headline,
            chip: asked,
            title: said.title ?? answerNames[asked] ?? root.capitalised(asked),
            // Asked by a name it had, it says so, as a row found by one does.
            once: oldWords[asked] ? [oldWords[asked]] : [],
            glyph: said.glyph ?? first.glyph,
            graph: said.graph,
            warns: said.warns === true,
            hue: said.hue ?? first.hue,
            swatch: first.swatch,
            // Who answered, to what, and whether it moves, so a step held
            // in it is asked again as it changes; and what it's about,
            // so a provider with several answers, as Machine's Processor
            // and Memory, knows which is held.
            asker: said.asker ?? null,
            heard: said.heard ?? "",
            // An answer that is a thing, as a pi's whole words are, says so.
            thing: said.thing ?? false,
            // One with more to say than its words, as the desk light's,
            // gives its drawer, as the headline it is when asked does.
            drawer: said.drawer,
            // Its name says why it came to nothing, so nothing else does.
            missed: said.missed === true,
            live: said.live ?? null,
            about: said.about ?? "",
            side: "",
            chord: [],
            verb: said.verb ?? ((said.copy ?? said.headline) === "" ? "close" : "copy"),
            now: true,
            // One whose doing takes a while keeps the launcher open, as
            // asking a question does, its whole words below the rows.
            stays: said.stays ?? false,
            // Whether it says it changes nothing, as asking a question.
            changes: said.changes,
            reads: said.reads ?? "",
            note: said.note ?? "",
            copies: said.run ? "" : said.copy ?? said.headline,
            run: said.run ?? (() => {
                const copy = said.copy ?? said.headline;
                if (copy !== "")
                    Quickshell.clipboardText = copy;
            }),
            // One held while it's still asked for has nothing in it yet.
            verbs: parted(said) ? () => said.rows : undefined
        };
    }

    // A provider has more to say: out at the top, its answer is asked
    // again; stepped in, so is whatever of its answers is held or listed.
    // With nothing typed nothing is asked, so home isn't made again: it
    // was, each time the machine's queue changed, and a new list scrolled
    // back to the top with the selection out of sight below it (Joel,
    // 2026-10-25: "jumpy ... at home arrowing around"). An answer asked
    // again keeps the list where it was scrolled, as a refresh does.
    function reanswer(source) {
        // An answer's rows are said only by asking it, so what it holds
        // agrees as it's asked again, shown or not, as the desk light's
        // Brightness does once litra says so.
        heldAgreed([source]);
        // At home nothing is asked again, even of a place that's one answer
        // (Disk), so home is never made again under him.
        if (!open || (held.length === 0 && typed.trim() === ""))
            return;
        // Timed from its daemon's line to the frame niri presents with it,
        // as a change is (refreshed): Power's, Heat's and every answer's
        // moves came here untimed (RCTRF8, 2026-11-11). One that shows
        // nothing new is let go untimed, as a refresh's is.
        const place = source.settings.name;
        updatedFrom(source);
        Report.say(`asked again by ${place}`);
        if (held.length > 0) {
            updateDrawn(place, relive(source));
            return;
        }
        const at = current;
        // Said again in place, as a refresh is: composed afresh, the rows
        // took their ranked order and a row d had just closed went, since
        // the waits answer each time the machine's queue moves (the moves-
        // windows walk, 2026-11-05).
        steadying = true;
        try {
            compose(false);
        } finally {
            steadying = false;
        }
        root.showMoved(at);
        updateDrawn(place, true);
    }

    // After a refresh, the list stays where it was and moves only to show a
    // selection the refresh itself moved, through reveal, so it glides and
    // never restarts a glide headed there. Brought to the selection on
    // every refresh instead, the list snapped mid-glide to where the glide
    // was going and was pulled back for the rest of it, each second inside
    // Machine, and a list scrolled by hand went back under the pointer
    // (Joel, 2026-10-29: "a lot of scrolling focus quirkiness").
    function showMoved(was) {
        if (current >= 0 && current !== was)
            list.reveal(current);
    }

    // An answer row asked again, as it says now, keeping where it's held
    // and how its step is named, or null when it no longer answers.
    function again(row) {
        const said = row.asker ? row.asker.answer(row.heard) : null;
        // Held whole or with its parts, as resolved() holds it.
        if (!parted(said) && said?.whole !== true)
            return null;
        return inPlace(Object.assign(answered(heard(said, row.asker, row.heard)), {
            key: row.key,
            is: row.is,
            chip: row.chip,
            title: row.title,
            gone: said.gone === true
        }), row.placed);
    }

    // An answer a category holds, named as the vocabulary names it, what it
    // says beside: a list by name keeps it in one place however its figure
    // changes, where "Sunday 25 October 2026" moved every day. Out at the
    // top, an answer is named by what it says, as it's what was asked.
    function inPlace(row, name) {
        if (!name)
            return row;
        return Object.assign(row, { name: name, side: row.name, placed: name });
    }

    // What a source's answers say now, wherever they're shown below the
    // top: an answer stepped into, its parts and what's held in them made
    // again, or a category's own answers, listed or asked in it. What's
    // shown refreshes in place, as `refreshed` says. It answers whether
    // anything shown was made again, for the updates measure (reanswer).
    function relive(source) {
        if (asking)
            return false;
        const steps = held.slice();
        const at = steps.findIndex(step => step.row.asker === source);
        const last = steps.length - 1;
        if (at >= 0) {
            const was = steps[at];
            const row = again(was.row);
            if (!row)
                return false;
            steps[at] = Object.assign({}, was, {
                row: row,
                rows: Steady.kept(was.rows, led(row, row.verbs()), pairedId)
            });
            // The list it was stepped into from says it as it is now too,
            // for the way back out.
            if (at > 0)
                steps[at - 1] = Object.assign({}, steps[at - 1], {
                    rows: steps[at - 1].rows.map(one => one === was.row ? row : one)
                });
            if (!remade(steps, at + 1))
                return false;
        } else if (category !== null && (steps[last].rows.some(row => row.asker === source)
            || (answer && answer.asker === source))) {
            // A row a condition holds back stays dimmed as it's said again,
            // or each new reading would light it for a moment. A row its
            // asker no longer answers is kept as it was, dimmed or not: if
            // the condition moves meanwhile, conditionMoved remakes the step
            // from the place's members, so it lights again once its asker
            // answers, and stays dimmed only while the asker is silent.
            const place = steps[last].row.category ?? category;
            steps[last] = Object.assign({}, steps[last], {
                rows: steps[last].rows.map(row => row.asker === source
                    ? heldBack(place, again(row) ?? row) : row)
            });
        } else {
            // A member that didn't answer as its place was made, as Firmware
            // while fwupd is first asked, or Power's Kept From Resting
            // before its reading, has no row there to say again, so the
            // place is made again from its members, the row in its place.
            const place = steps[last].row.category ?? category;
            const member = !!place && members(place).some(row => row.asker === source);
            if (member)
                placesRead();
            return member;
        }
        held = steps;
        reshow(steps.length);
        return true;
    }

    // The headline as a row of its own, for Return and the footer. Return
    // copies it, unless the answer says what it does instead, as setting
    // the brightness to what was typed does.
    // In a step whose line holds its words, the row those words make, as a
    // save dialog's name: the line is the row Return stands on, rather
    // than the name said again below it (VKCF3Z, #1812). It's made again
    // as the rows are, since the words change it.
    property var lined: null
    // A step's line row stands at -1, above the rows, as an answer's
    // headline does, so a refresh or a folder's read landing keeps it.
    readonly property bool lineHeads: answer === null && headline !== null
    readonly property var headline: !answer ? (holdsWords ? lined : null) : {
        name: answer.headline,
        // An answer still being asked for has nothing to copy yet.
        verb: answer.verb ?? ((answer.copy ?? answer.headline) === "" ? "close" : "copy"),
        now: true,
        stays: answer.stays ?? false,
        thing: answer.thing ?? false,
        // An answer that's a thing with more to say than its words, as the
        // desk light's is, gives its drawer as any thing's row does.
        drawer: answer.drawer,
        reads: answer.reads ?? "",
        note: answer.note ?? "",
        // Asked by a name it had, it says so, as a row found by one does.
        once: oldWords[Fuzzy.plainly(typed)] ? [oldWords[Fuzzy.plainly(typed)]] : [],
        copies: answer.run ? "" : answer.copy ?? answer.headline,
        run: answer.run ?? (() => {
            const copy = answer.copy ?? answer.headline;
            if (copy !== "")
                Quickshell.clipboardText = copy;
        })
    }
    readonly property var selected: current === -1 ? headline : matches[current] ?? null

    // What sort of thing the selection is, in a word, while something is
    // typed: an answer, a collection, or its provider's kind. Inside a
    // collection or a thing's verbs, what's held already says it.
    readonly property string kind: {
        if (typed === "" || held.length > 0 || !selected)
            return "";
        // One that came to nothing isn't an answer, and says only why.
        if (selected === headline || selected.answered)
            return answer && answer.missed === true ? "" : "answer";
        if (selected.category)
            return "category";
        // A collection's own row has no provider's kind; a shortcut that
        // opens one is still a shortcut.
        if (selected.what)
            return selected.what;
        return selected.opens ? "collection" : "";
    }

    // A collection's members stay out of the main search, but a thing it
    // offers beside them, such as turning Wi-Fi off, is found like any, and
    // so is the collection itself, by its title or any of its words, so
    // typing what it holds goes straight to it.
    // A desktop action another row stands for (Provider.qml's draws) is
    // found only as that row.
    readonly property var standIns: sources.reduce((all, source) => all.concat(source.draws), [])
    // Every source's things, then each collection and category, as one
    // list: what the search finds from, and what keyed, partsOfRows and
    // namedAs are made of. It's joined by hand rather than bound, since a
    // binding subscribes to everything it reads however deep, and every
    // source's things and each collection's note read live state (the
    // machine queue, Niri's focus, the sessions): bound, it was made
    // again whole on each of those with the launcher closed, two thirds
    // of the shell's idle processor (2026-10-29). A change marks it
    // stale; it's joined again as the launcher opens, before anything
    // asks the catalogue from outside, and while open once the changes
    // settle, in one pass, so home, stepping in and a refresh in place
    // always read it whole.
    property var choices: []
    property bool choicesStale: true
    // How many times it's been joined, for the check that a closed
    // launcher joins nothing.
    property int choicesJoined: 0
    function stale() {
        choicesStale = true;
        // While a kept opening's sources wake, each would join it whole
        // again in its own slice, so it's joined once the last has woken.
        if (open && !waking)
            Qt.callLater(freshen);
    }
    function freshen() {
        if (!choicesStale)
            return;
        choicesStale = false;
        const out = [];
        for (const source of sources) {
            for (const row of source.things) {
                if (!(row.key.startsWith("action:") && standIns.includes(row.name)))
                    out.push(withSynonyms(row));
            }
        }
        // One that's no place of its own (`placed: false`), as Notices,
        // whose notices For You holds, is never found by its name, though
        // what it holds is (gathering).
        for (const source of collections.filter(one => one.collection.placed !== false))
            out.push(withSynonyms(named(source)));
        for (const source of collections.filter(one => one.collection.placed === false)) {
            const led = ledFound(source);
            if (led !== null)
                out.push(led);
        }
        for (const row of categories)
            out.push(withSynonyms(row));
        choicesJoined += 1;
        choices = out;
    }

    // Where a collection that's no place of its own leads, as one found row,
    // found by the collection's own words, its name, noun and the names it
    // once had: "notices" and "notifications" find For You's Desktop, where
    // Super+N goes, whether or not For You lists it now, and never by the
    // name it leads to, which that row answers, so there's never two
    // (VKCF3Z, 2026-11-09: a rename never loses a row). It's the leading
    // source's own row, its key and drawer, said where it is, and Return,
    // Tab and Ctrl+] step in as `into` does (`ledBy`, read by `steppedAsLed`).
    function ledFound(source) {
        const name = source.settings.name;
        const { leading, rows } = leadingTo(name) ?? { leading: null, rows: [] };
        const row = rows[rows.length - 1];
        if (row === undefined)
            return null;
        const found = Object.assign({}, row, {
            ledBy: name,
            side: [leading.collection?.title ?? leading.settings.name]
                .concat(rows.slice(0, -1).map(one => one.name)).join(" \u203a "),
            verb: "go to",
            stays: true,
            weight: 1,
            prepared: Fuzzy.prepare(""),
            keywords: withSynonyms(named(source)).keywords,
            named: true
        });
        found.run = () => root.into(name);
        return found;
    }

    // A row found for a place that lives deeper, as "notices" finds For
    // You's Desktop, steps in through the rows above it, by Return, Tab or
    // Ctrl+], so - climbs back out the way it went in. It's asked before
    // the row's own ways in, which it carries from the row it copies.
    function steppedAsLed(row) {
        if (held.length > 0 || row.ledBy === undefined)
            return false;
        root.into(row.ledBy);
        return true;
    }

    // The source that leads a name deeper (`leadsTo`) and the rows it
    // leads through, asked once, or null when none does.
    function leadingTo(name) {
        for (const one of sources) {
            const rows = typeof one.leadsTo === "function" ? one.leadsTo(name) : null;
            if (rows !== null)
                return { leading: one, rows: rows };
        }
        return null;
    }
    readonly property var collections: sources.filter(source => source.collection)

    // Each row by its key, and the rows that are parts of another (`of`
    // names it), such as Slack's theme of Slack, by the key of what they're
    // part of: listed inside it, and found from outside only by a word past
    // its name, so "slack" is Slack and "slack theme" the theme.
    readonly property var keyed: {
        const out = {};
        for (const choice of choices)
            out[choice.key] = choice;
        return out;
    }
    readonly property var partsOfRows: {
        const out = {};
        for (const choice of choices)
            if (choice.of)
                out[choice.of] = (out[choice.of] ?? []).concat([choice]);
        return out;
    }

    // Whether a part is left out of a search, every word typed only
    // naming what it's part of.
    function partHidden(choice, typedWords) {
        const whole = keyed[choice.of];
        if (!whole)
            return false;
        const named = Fuzzy.words(whole.name);
        return typedWords.every(word => named.some(one => one.startsWith(word)));
    }

    // A spot: a part or a figure a level inside a thing a category holds,
    // a headset's Sample rate or a screen's Colour depth, found from the
    // root by its name, saying where it lives, and Return goes there, the
    // crumbs filled and the spot selected, so ← goes back up through each
    // (Joel, 2026-10-25: "being able to drill right down by typing in the
    // details from a root view that would put me in the hierarchy"). Only
    // what a thing reads or holds is a spot; what it does, such as
    // Disconnect, is said once for each thing and would be found many
    // times over. Made a few seconds after the shell starts and a moment
    // after each closing (spotsLater), and by the first search of three
    // letters or more when none were made yet,
    // of the collections whose things are devices, few and read whole, so
    // a search never reads them again.
    readonly property var spotCollections: ["sound", "bluetooth", "displays", "wifi",
        "keyboard", "cameras"]
    readonly property int spotsFewest: 3
    property var spots: []
    property bool spotsMade: false

    // Each held thing's spots as last made, by where it's held
    // (`computer/sound`, `home/machine:disk`), with how many times its
    // source had changed then (changesIn), so a closing makes again only
    // those whose source changed since. It made all of them each time:
    // Sound's 111 rows on every closing of Joel's, with no change to Sound
    // (2026-11-03). A source that says what its spots are made of, in a
    // `spotsStamp` of its own that a figure moving never changes, as
    // Displays and Wi-Fi do, has them made again only once that changes,
    // so a light or a signal moving remakes nothing as the launcher closes,
    // and their figures a minute later (spotsFreshened), if it stayed
    // closed: closings made Displays' 87 rows and Wi-Fi's 17 to 65 again
    // as Joel went back to typing (2026-11-10), each its lights or its
    // signal read while it showed. What the last making made and kept, for
    // the closing's report.
    property var spotsKept: ({})
    property var spotsSaid: ({ made: [], kept: [] })

    // An answer's parts are spots too, as Battery's Health is, from the
    // answers a place holds and those home is, such as Disk: asked already
    // as their places are made, so finding them asks nothing more. Each
    // place's collections' spots come before its answers'.
    function spotsFrom(freshening = false) {
        const kept = {};
        const said = { made: [], kept: [] };
        const piece = (where, id, make) => {
            const name = Words.of(id).provider ?? id;
            const source = sources.find(one => one.settings.name === name) ?? null;
            const stamp = source !== null ? root.changesIn[name] ?? 0 : -1;
            const shape = source !== null && (source.spotsStamp ?? "") !== ""
                ? source.spotsStamp : null;
            const before = root.spotsKept[where];
            // Its make, where it says one, and then, freshening, its
            // figures; else, closing, any change, and freshening, none.
            const same = before !== undefined && (shape === null
                ? freshening || before.stamp === stamp
                : before.shape === shape && (!freshening || before.stamp === stamp));
            kept[where] = same ? before : Object.assign({ stamp: stamp, shape: shape }, make());
            (same ? said.kept : said.made).push(where);
            return kept[where];
        };
        let out = [];
        for (const shown of categories) {
            const pieces = holdings(shown.category).filter(id => !root.placeNamed(id))
                .map(id => piece(`${shown.category.id}/${id}`, id,
                    () => root.heldSpots(shown, id)));
            for (const each of pieces.filter(one => !one.answers))
                out = out.concat(each.spots);
            for (const each of pieces.filter(one => one.answers))
                out = out.concat(each.spots);
        }
        for (const id of Words.home.filter(one => !root.placeNamed(one)))
            out = out.concat(piece(`home/${id}`, id, () => {
                const place = root.resolved(id);
                return { answers: true,
                    spots: place && place.answered ? root.answerSpots(place, [place]) : [] };
            }).spots);
        spotsKept = kept;
        spotsSaid = said;
        return out;
    }

    // The spots of one thing a place holds: a device collection's, each
    // thing's parts, or an answer's parts.
    function heldSpots(shown, id) {
        const row = resolved(id);
        const spots = [];
        if (row && row.from && spotCollections.includes(row.opens))
            for (const thing of row.verbs())
                for (const part of (thing.verbs ? thing.verbs() ?? [] : []).concat(partsIn(thing)))
                    if (isSpot(part))
                        spots.push(spotOf(part, [shown, row, thing]));
        const answers = !!row && row.answered === true;
        return { answers: answers, spots: answers ? answerSpots(row, [shown, row]) : spots };
    }

    // What's asked whole at the top, by any thing's words: a part named
    // so, as the processor's Cores, is found as that answer, never as a
    // second row beside it.
    readonly property var answerWords: {
        const out = {};
        for (const entry of Object.values(Words.things))
            for (const word of entry.answers ?? [])
                out[Fuzzy.plainly(word)] = true;
        return out;
    }

    function answerSpots(said, trail) {
        return (said.verbs ? said.verbs() ?? [] : [])
            .filter(part => isSpot(part) && !answerWords[Fuzzy.plainly(part.name)])
            .map(part => spotOf(part, trail));
    }

    function isSpot(part) {
        return !!part && (part.name ?? "") !== ""
            && (part.figure === true || !!part.targets || !!part.verbs || !!part.nested);
    }

    function spotOf(part, trail) {
        // A place inside another says it: Battery's Health is in Machine,
        // then Power, then Battery.
        const above = trail[0].category
            ? placesAbove(trail[0].category).map(place => place.name ?? place.id) : [];
        const spot = {
            key: `spot:${trail.map(row => steadyId(row)).join("/")}/${steadyId(part)}`,
            name: part.name,
            glyph: part.glyph,
            hue: part.hue,
            graph: part.graph,
            side: above.concat(trail.map(row => row.name)).join(" \u203a "),
            chord: [],
            // Go To, as a window's is, since both take Joel to where it
            // is, and its side already names where (VKCF3Z, 2026-11-07).
            verb: "go to",
            // Where it is, and so where Return goes, by what each step is.
            trail: trail.map(row => steadyId(row)),
            spot: steadyId(part),
            stays: true,
            prepared: Fuzzy.prepare(part.name),
            once: oldNames(part)
        };
        spot.olds = spot.once.length > 0 ? Fuzzy.prepare(spot.once.join(" ")) : null;
        spot.run = () => root.goTo(spot);
        return spot;
    }

    // The spots what's typed names, best first, once it's three letters or
    // more, but for one a row found already says: a thing found at the top
    // is its own row.
    function spotted(letters) {
        if (Fuzzy.plainly(letters).length < spotsFewest)
            return [];
        if (!spotsMade) {
            spots = spotsFrom();
            spotsMade = true;
            Report.say("spots made on a key");
        }
        const found = [];
        for (const spot of spots) {
            // By its name, or else by a name it had.
            const byName = Fuzzy.match(spot.prepared, words, false);
            const m = byName.score < 0 && spot.olds ? Fuzzy.match(spot.olds, words, false) : byName;
            if (m.score >= 0)
                found.push({ item: spot.prepared, score: m.score, grade: m.grade, row: spot });
        }
        return Fuzzy.keep(found).map(f => Object.assign({ grade: f.grade, tier: 2 }, f.row));
    }

    // The things what's typed means, from each source that finds by meaning
    // (`meant`), as modules/meaning's does: only ever after everything its
    // letters found, the spots and the collections gathered included, and
    // none of those again, so a guess never hides or outranks a match
    // (Joel, 2026-11-07: "never over truthfulness"). Asked by what a source
    // can do, as Capture asks `takenByText`, since no source is named here.
    function meantBelow(letters, listed) {
        const shown = new Set(listed.map(row => Rank.identity(row)));
        return sources.filter(source => typeof source.meant === "function")
            .reduce((all, source) => all.concat(source.meant(letters, shown)), []);
    }

    // Goes to a spot: from the root into each step of its trail, found again
    // by what it is, then onto the spot itself. A step gone since, such as a
    // headset turned off, stops it where it got to. Each step's row is
    // selected before it's stepped into, and the first step keeps what was
    // typed and selected before it, so - climbs back out
    // through every step and onto the search it came from, as any way in
    // does (Joel, 2026-10-26: every way in climbs back out the same way).
    // The steps are one jump, as Vim's :tag is, so Ctrl+O goes straight
    // back to the search.
    function goTo(spot) {
        oneJump(() => walkTo(spot));
    }

    function walkTo(spot) {
        const came = here();
        held = [];
        restart("");
        for (const step of spot.trail) {
            // From the top, a place or an answer home is, such as Disk;
            // inside a collection, its things and then its parts, as it
            // shows them, so Agents leads on to Quests.
            const from = held.length > 0 ? held[held.length - 1].row.from : null;
            const rows = held.length === 0 ? categories.concat(home())
                : from ? offered.concat(partsOf(from)) : offered;
            const row = rows.find(one => steadyId(one) === step) ?? null;
            const listedAt = matches.findIndex(one => steadyId(one) === step);
            if (listedAt >= 0)
                current = listedAt;
            const first = held.length === 0;
            if (row === null || !enter(row))
                return;
            if (first)
                held = [Object.assign({}, held[0], came)].concat(held.slice(1));
        }
        const at = matches.findIndex(row => steadyId(row) === spot.spot);
        if (at >= 0) {
            current = at;
            list.positionViewAtIndex(at, ListView.Contain);
        }
    }

    // The catalogue (launcher/Catalogue.js): every place and every thing's
    // actions, walked from home by the rows the launcher shows, so what asks
    // from outside does what Return would. Each id is the row's path, its
    // steps' steady ids joined by a "\u203a", which no key holds.
    readonly property string catalogueStep: "\u203a"

    // Set while the catalogue is walked from outside, so a collection that
    // makes its rows only while the launcher is open (Agents) makes them
    // for the walk too.
    property bool cataloguing: false

    // The rows a terminal's `launcher show` last walked through (`walked`)
    // and listed (`listed`), held a few seconds after it asks, so a
    // provider that reads only while its rows are seen, as Machine reads
    // its daemon and its disks, reads for a terminal with the launcher
    // closed, and the terminal's next ask has the figures (Y2YBTS's
    // review of #2404). Let go once nothing has asked for a while, so a
    // daemon read for one show isn't read for ever.
    property var terminalSees: ({ walked: [], listed: [] })

    Timer {
        id: terminalSeeing

        interval: 3000
        onTriggered: root.terminalSees = { walked: [], listed: [] }
    }

    function catalogue() {
        cataloguing = true;
        try {
            return catalogueWalked();
        } finally {
            cataloguing = false;
        }
    }

    function catalogueWalked() {
        freshen();
        const out = [];
        const seen = new Set();
        const walk = (rows, above, ids, names, depth) => {
            for (const row of rows.filter(one => Catalogue.listed(one))) {
                const path = ids.concat([steadyId(row)]);
                const id = path.join(catalogueStep);
                if (!seen.has(id)) {
                    seen.add(id);
                    out.push(Catalogue.entry(row, id, names));
                    if (Catalogue.descends(row, depth, above))
                        walk(catalogueHolds(row, above === null || collects({ row: above })),
                            row, path, names.concat([row.name]), depth + 1);
                }
            }
        };
        walk(home(), null, [], [], 0);
        return out;
    }

    // Every row Return does nothing on, and how many rows there were,
    // from home down as stepping in
    // shows them, each with where it sits, for the check that fails any
    // (Joel, 2026-10-26: every row has a Return act). It walks deeper
    // than the catalogue and into each program, window and quest, but not
    // into the long lists, the clipboard's or the emoji, whose rows are
    // all made one way and are proven offscreen.
    readonly property int actlessDeepest: 6

    // Each thing the vocabulary places that is an answer, by its name, as
    // Time, Date and Weather are: its one row is the answer as
    // `resolved` places it, wherever it shows (Joel, 2026-10-28: "we need
    // to not have different versions of these sort of elements"). A part,
    // whose id is its holder's and its own, as a display's Brightness is,
    // shows as its holder makes it, so it isn't one.
    readonly property var answeredThings: {
        const out = {};
        for (const id of Object.keys(Words.things)) {
            const entry = Words.of(id);
            if ((entry.answers ?? []).length > 0 && (entry.in ?? "") !== "" && !id.includes(":"))
                out[entry.name] = id;
        }
        return out;
    }

    // Whether a row bears an answered thing's name without being that
    // thing's own row, which is a surface drawing its own version of it.
    // What an answer holds is its own, as the time's Date is the date
    // where that time is, so it isn't asked of.
    function ownVersion(row, above) {
        const id = answeredThings[row.placed ?? row.name];
        return id !== undefined && above?.answered !== true && row.is !== `answer:${id}`
            && row.key !== `place:${id}`;
    }

    // The kinds made by one component each, by how their rows are keyed,
    // and the component that makes them (docs/design.md, one component per
    // thing): a mail, a meeting, a file, a folder, a site or a Slack
    // conversation row made anywhere but by its component is a version a
    // surface drew of it.
    readonly property var components: [
        { keyed: /^mail:/, made: "mail" },
        { keyed: /^slack:conversation:/, made: "slackConversation" },
        { keyed: /^slack:message:/, made: "slackMessage" },
        { keyed: /^calendar:meeting:/, made: "meeting" },
        { keyed: /^file:/, made: "file" },
        { keyed: /^folder:/, made: "folder" },
        { keyed: /^site:/, made: "site" }
    ]

    // A row of a kind its component makes, made otherwise, or null.
    function uncomposed(row) {
        const kind = components.find(one => one.keyed.test(row.key ?? ""));
        return kind && row.component !== kind.made ? kind.made : null;
    }

    function actless() {
        const out = [];
        const own = [];
        const loose = [];
        let composed = 0;
        // Every step whose first row is a copy where more is held, which
        // docs/design.md's Return rule never allows ("Copy is never the
        // first row of a step, unless it's the only one, and is always the
        // last"; Joel, 2026-10-28: "the copy at the top of most screens is
        // confusing").
        const copyFirst = [];
        // Every row whose Return steps in over an act of its own, where the
        // step doesn't lead with that act, so it couldn't be chosen inside
        // (Joel, 2026-10-28: "enter is supposed to be the default action,
        // but that means that action should be selectable within").
        const skipped = [];
        // What each row's drawer would show beside the row's own words, so
        // a check can judge for itself that none only repeats its row.
        const drawers = [];
        // Every row holding something for the drawer, though it isn't a
        // thing, so the drawer never shows it: a maker that forgot to mark
        // its thing, or words with nowhere to be read (VKCF3Z, 2026-10-29:
        // a row gets no drawer unless it's one of the kinds).
        const undrawn = [];
        const seen = new Set();
        const walk = (rows, above, ids, names, depth) => {
            for (const row of rows.filter(one => !!one && (one.name ?? "") !== "")) {
                const path = ids.concat([steadyId(row)]);
                const id = path.join(catalogueStep);
                if (!seen.has(id)) {
                    seen.add(id);
                    if (!Catalogue.acts(row))
                        out.push({ id, words: row.name, in: names.join(" \u203a ") });
                    if (ownVersion(row, above))
                        own.push(`${names.concat([row.name]).join(" \u203a ")}`
                            + ` (${row.is ?? row.key})`);
                    const kind = uncomposed(row);
                    if (kind !== null)
                        loose.push(`${names.concat([row.name]).join(" \u203a ")}`
                            + ` (${row.key}, not made as a ${kind})`);
                    else if (row.component)
                        composed += 1;
                    const drawn = drawerOf(row);
                    if (drawn)
                        drawers.push({
                            in: names.concat([row.name]).join(" \u203a "),
                            row: [row.name, row.side, row.note]
                                .map(words => String(words ?? "").trim()),
                            title: String(drawn.title ?? "").trim(),
                            // The moment's plate says its words and its
                            // facts, in groups, as a thing's says its own.
                            body: String(drawn.body ?? drawn.say ?? "").trim(),
                            facts: (drawn.facts ?? []).concat(...(drawn.groups ?? []))
                                .filter(fact => fact).length,
                            picture: (drawn.picture ?? "") !== ""
                        });
                    // A plated row, Now at home, and a place naming what
                    // it holds have a drawer as a thing does (drawerOf).
                    if (!drawable(row)
                        && (!!row.drawer || (row.reads ?? "") !== ""
                        || (row.picture ?? "") !== ""))
                        undrawn.push(`${names.concat([row.name]).join(" \u203a ")}`
                            + ` (${row.key ?? row.is})`);
                    const deeper = depth < actlessDeepest
                        && !(row.opens && Catalogue.long(row.opens));
                    const inside = deeper
                        ? catalogueHolds(row, above === null || collects({ row: above })) : [];
                    if (inside.length > 1 && copying(inside[0]))
                        copyFirst.push({ id, words: row.name, in: names.join(" \u203a ") });
                    if (inside.length > 0 && stepsOver(row)
                        && !Catalogue.saysAct(inside[0], verbOf(row).toLowerCase()))
                        skipped.push({
                            id,
                            words: `${row.name} (${verbOf(row)}, led by ${inside[0].name})`,
                            in: names.join(" \u203a ")
                        });
                    // A row whose Return does one of its acts holds that
                    // act in its step too, so it can be chosen there.
                    if (deeper && row.returns
                        && !inside.some(one => one.key === row.returns.key))
                        skipped.push({
                            id,
                            words: `${row.name} (${verbOf(row)}, not held inside)`,
                            in: names.join(" \u203a ")
                        });
                    walk(inside, row, path, names.concat([row.name]), depth + 1);
                }
            }
        };
        walk(home(), null, [], [], 0);
        // How many it walked too, so a check that finds none knows it
        // looked.
        return { walked: seen.size, rows: out, ownVersions: own, copyFirst, skipped,
            uncomposed: loose, composed: composed, drawers, undrawn };
    }

    // Whether Return on a row steps in where the row has an act of its own
    // it would otherwise do, which the step must then lead with: not a
    // copy, which comes after the acts, nor a row that only steps in, asks
    // first or asks for words.
    function stepsOver(row) {
        const verb = verbOf(row).toLowerCase();
        // A row with no act of its own, as a place or a group, steps over
        // nothing: stepping in is all its Return does.
        return !!row.run && stepsIn(row) && !leadsElsewhere(row) && !row.confirm && !row.given
            && !["", "more", "see all", "copy", "close"].includes(verb);
    }

    // Whether a row is an act of copying, as a step's Copy is, rather than
    // a value that copies what it says. One marked `use` is what its thing
    // is for, so it leads, as an entry's Copy the Password does (design.md).
    function copying(row) {
        return !!row && row.figure !== true && !row.answered && verbOf(row) === "copy"
            && /^copy\b/i.test(row.name ?? "") && row.use !== true;
    }

    // What a row holds as `enter` shows it stepped into: from home or a
    // collection (`things`), what can be done to it, then its parts, a
    // collection's own among them, as a step into it lists them (Agents'
    // Quests, Jobs, Runs and AI spend); from a thing's own verbs,
    // what each is done to. One whose rows can't be made out of the
    // launcher's step holds nothing here.
    function catalogueHolds(row, among) {
        try {
            const things = asThing(row, among);
            const making = things ? row.verbs : row.targets;
            const made = making ? making() ?? [] : [];
            const own = things || copiesOnly(row) ? led(row, made) : made;
            const places = things && row.from ? partsOf(row.from) : [];
            return things ? own.concat(places, partsIn(row)) : own;
        } catch (error) {
            return [];
        }
    }

    // The rows an id is the path to, from home, or null for one not there.
    function catalogued(id) {
        let rows = home();
        let things = true;
        const path = [];
        for (const step of String(id).split(catalogueStep)) {
            const row = rows.find(one => Catalogue.listed(one) && steadyId(one) === step) ?? null;
            if (row === null)
                return null;
            path.push(row);
            rows = catalogueHolds(row, things);
            things = collects({ row: row });
        }
        return path.length > 0 ? path : null;
    }

    // Does what an id names, as asked from outside, saying what it did: a
    // place opens, an action is done, and one that ends something is shown
    // ready, selected, for Joel's own Return. Said in the journal too, as
    // the launcher's picks are.
    function catalogueRun(id, words) {
        cataloguing = true;
        try {
            return catalogueRunWalked(id, words);
        } finally {
            cataloguing = false;
        }
    }

    function catalogueRunWalked(id, words) {
        freshen();
        const path = catalogued(id);
        if (path === null)
            return { error: `the launcher has nothing called ${id}` };
        const target = path[path.length - 1];
        const said = path.map(row => row.name).join(" \u203a ");
        const ids = path.map(row => steadyId(row));
        console.info(`launcher: asked from outside to ${Catalogue.kindOf(target)} ${said}`);
        if (Catalogue.kindOf(target) === "place") {
            open = true;
            goTo({ trail: ids, spot: "" });
            return { done: true, said: `opened ${said}` };
        }
        if (Catalogue.disruptive(target)) {
            open = true;
            goTo({ trail: ids.slice(0, -1), spot: ids[ids.length - 1] });
            return { done: false, offered: true, said: `${said}, ready for Return` };
        }
        // What Return does with it, as it does from the rows.
        const act = returnOf(target);
        if (act.given) {
            if ((words ?? "") === "")
                return { error: `${said} takes words: ${act.ask ?? ""}` };
            act.given(words);
            return { done: true, said: said };
        }
        // As Return does it, save that what's typed, Ask's question, picks
        // nothing: one whose outcome takes a while keeps the launcher, and
        // the rest close it first.
        trials.picked(target, "", path.length - 1);
        if (act.stays) {
            act.run();
            return { done: true, said: said };
        }
        if (act.now)
            act.run();
        const shown = open;
        open = false;
        if (act.now)
            return { done: true, said: said };
        if (act.later)
            runOnceGone(act, shown);
        else
            act.run();
        return { done: true, said: said };
    }

    // What a place or a row shows, asked from a terminal
    // (launcher/Showing.js): walked from home as the catalogue walks, by
    // every named row, figures among them, so a terminal says what the
    // launcher would and nothing a provider doesn't. Gives where it is
    // (`in`, its steps' names, and `path`, their steady ids), the row with
    // its drawer (`row`, null at home) and the rows a step into it holds;
    // or, where a step names nothing, that step (`missing`), the names it
    // might have meant (`near`) and where it was looked for. A path is
    // asked starting with a step's mark, since qs takes no empty argument,
    // so home is that mark alone and an empty step is no step.
    // `everyDrawer`, for `launcher show --drawers`, gives each row's
    // drawer too, which is only built where it's asked for. `namesOnly`,
    // for completion, leaves terminalSees as it was, so a Tab starts no
    // daemon reading for figures it never shows (design.md: nothing
    // polled that needn't be).
    function shownToTerminal(path, everyDrawer, namesOnly) {
        cataloguing = true;
        try {
            return shownWalked(path, everyDrawer === true, namesOnly === true);
        } finally {
            cataloguing = false;
        }
    }

    function shownWalked(path, everyDrawer, namesOnly) {
        freshen();
        const walked = walkedByName(String(path ?? "").split(catalogueStep));
        // A row a read makes, as a joined network's Speed is made from
        // wifid's detail of it, is missed until the read comes, so the walk
        // as far as it went is seen too, and the miss says whether what it
        // looked among is still being read, for the terminal to ask again.
        if (walked.missing !== undefined)
            return {
                missing: walked.missing,
                near: walked.near,
                in: walked.in,
                reading: seenByTerminal(walked.trail, walked.looked, namesOnly)
            };
        const { trail, rows, among } = walked;
        const ids = trail.map(one => steadyId(one));
        // Only the row the path names says its drawer, since a listed row's
        // would be built for every row a long place holds, for one show;
        // `everyDrawer` (`launcher show --drawers`) asks for each row's too.
        // A drawer read as it's first asked for, a turn later, as a
        // keyboard's, is still being read until it's in (drawerOf's `told`),
        // so the terminal asks again for it.
        const told = { unread: false };
        const entry = (one, at, drawn) => Showing.shaped(one, {
            id: at.concat([steadyId(one)]).join(catalogueStep),
            kind: Catalogue.kindOf(one),
            verb: verbOf(one),
            holds: showsMore(one),
            drawer: drawn ? drawerOf(one, told) : null
        });
        const last = trail.length > 0 ? trail[trail.length - 1] : null;
        // A thing's own acts are what `launcher do` does rather than rows to
        // read, so they're said apart (Showing.isAct); a place's or a
        // collection's rows are its things.
        const acting = last !== null && asThing(last, among) && !last.category
            && !collects({ row: last });
        const verbs = acting ? led(last, last.verbs?.() ?? []).map(one => steadyId(one)) : [];
        const isAct = one => Showing.isAct(one, verbs, steadyId(one));
        const sayable = rows.filter(one => !!one && !one.gone && (one.name ?? "") !== "");
        // Built before `reading` is reckoned, since building a drawer may
        // start its read.
        const shown = {
            in: trail.map(one => one.name),
            path: ids,
            row: last === null ? null : entry(last, ids.slice(0, -1), true),
            rows: sayable.filter(one => !isAct(one)).map(one => entry(one, ids, everyDrawer)),
            acts: sayable.filter(one => isAct(one)).map(one => entry(one, ids, false))
        };
        const reading = seenByTerminal(trail, sayable, namesOnly) || told.unread;
        return Object.assign({ reading: reading }, shown);
    }

    // Counts what a terminal's show walked through and listed as seen,
    // unless it's completion asking (`namesOnly`), and says whether a
    // provider of a row among them is still reading it for this look, so
    // the terminal asks again for its figures; another's reading never
    // holds it up. A row is the provider's when it's live from it, as
    // Machine's are, or is the collection it opens, as Wi-Fi is for its
    // networks.
    function seenByTerminal(trail, listed, namesOnly) {
        if (!namesOnly) {
            terminalSees = { walked: trail, listed: listed };
            terminalSeeing.restart();
        }
        const seen = trail.concat(listed);
        return sources.some(source => source.stillReading === true
            && seen.some(one => one?.live === source || one?.from === source));
    }

    // The rows steps name from home, each by its steady id or its name as
    // Showing.step finds it, with what the last holds (`trail`, `rows`);
    // or, where a step names nothing, that step, the names it might have
    // meant and where it was looked for, by name (`in`) and as the rows
    // walked (`trail`) and looked among (`looked`). An empty step is no
    // step. `among` says whether the last was found among things, as
    // asThing reads it.
    function walkedByName(steps) {
        let rows = home();
        let things = true;
        let among = true;
        const trail = [];
        for (const said of steps.filter(one => String(one).trim() !== "")) {
            const found = Showing.step(rows.filter(one => !!one && !one.gone), said,
                one => steadyId(one));
            if (!found.row)
                return { missing: said, near: found.near, in: trail.map(one => one.name),
                    trail: trail, looked: rows.filter(one => !!one && !one.gone) };
            trail.push(found.row);
            among = things;
            rows = catalogueHolds(found.row, things);
            things = collects({ row: found.row });
        }
        return { trail, rows, among };
    }

    // The rows a path of steady ids names from home, as walkedByName walks
    // names, for the y a terminal gives (launcher do): each found by its id
    // alone, so a row moved or gone since it was asked about is missed,
    // said by the step it was typed as, and never done in another's place
    // (Y2YBTS's review of #2421).
    function walkedById(ids, steps) {
        let rows = home();
        let things = true;
        let among = true;
        const trail = [];
        for (let at = 0; at < ids.length; at++) {
            const found = rows.find(one => !!one && !one.gone && steadyId(one) === ids[at]);
            if (!found)
                return { missing: String(steps[at] ?? ids[at]), near: [],
                    in: trail.map(one => one.name) };
            trail.push(found);
            among = things;
            rows = catalogueHolds(found, things);
            things = collects({ row: found });
        }
        return { trail, rows, among };
    }

    // Does what a terminal names (launcher do), asked as JSON, { steps,
    // path, words, sure }, since qs takes no empty argument: its steps
    // walked by name as show walks them, so it offers what : offers, each
    // act of what it's in. One the line would ask about first, or that
    // ends something, isn't done until the terminal says it's sure: it
    // answers with the launcher's own question (`asks`), asked as the line
    // asks it, naming what it's an act of, and the steady ids of what it
    // asked about (`path`), which the sure ask walks instead. A place opens
    // in the launcher, as the catalogue's run opens one.
    function doneFromTerminal(asked) {
        cataloguing = true;
        try {
            const said = JSON.parse(asked);
            const steps = said.steps ?? [];
            const path = Array.isArray(said.path) ? said.path : [];
            freshen();
            return doneWalked(path.length > 0 ? walkedById(path, steps) : walkedByName(steps),
                said.words ?? "", said.sure === true);
        } catch (error) {
            // The shell's own failure, said in its log in the engine's words;
            // the terminal says it in words.json's (Y2YBTS's review of #2421).
            console.warn(`launcher: a terminal's do failed: ${error}`);
            return { failed: true };
        } finally {
            cataloguing = false;
        }
    }

    // Does what's walked, as doneFromTerminal says; none walked, it's
    // `nothing`, which the terminal says in its own words.
    function doneWalked(walked, words, sure) {
        if (walked.missing !== undefined)
            return { missing: walked.missing, near: walked.near, in: walked.in };
        const trail = walked.trail;
        if (trail.length === 0)
            return { nothing: true };
        const target = trail[trail.length - 1];
        const holder = trail.length > 1 ? trail[trail.length - 2] : null;
        const names = trail.map(one => one.name);
        const ids = trail.map(one => steadyId(one));
        console.info(`launcher: asked from a terminal to ${Catalogue.kindOf(target)}`
            + ` ${names.join(" \u203a ")}`);
        if (Catalogue.kindOf(target) === "place") {
            open = true;
            goTo({ trail: ids, spot: "" });
            return { done: true, opened: true, in: names };
        }
        const act = returnOf(target);
        const asking = CommandBinding.asking({ command: commandNamed(act).command }, [act],
            { all: 1, bang: false }, Options.commands ?? {});
        if (!sure && (asking.ask !== "" || Catalogue.disruptive(target) || actEnds(target))) {
            // As the line asks it (commandAsked): a row that's its own act,
            // as a window's Close, is an act of what holds it; any other is
            // its own act's thing.
            const of = act === target ? askedOfHolder(holder, target) : target;
            return {
                asks: Confirm.question([{ ends: act.ends || verbOf(act), asked: askedOf(act),
                    name: act.askedAs || of.name, kind: kindOf(of), leaves: act.leaves ?? "" }],
                    Words.field.operators ?? {}),
                in: names,
                path: ids
            };
        }
        if (act.given) {
            if (words === "")
                return { takes: act.ask ?? "", in: names };
            act.given(words);
            return { done: true, in: names };
        }
        // As the catalogue's run does it, as Return would.
        trials.picked(target, "", trail.length - 1);
        if (act.stays) {
            act.run();
            return { done: true, in: names };
        }
        if (act.now)
            act.run();
        const shown = open;
        open = false;
        if (act.now)
            return { done: true, in: names };
        if (act.later)
            runOnceGone(act, shown);
        else
            act.run();
        return { done: true, in: names };
    }

    // Whether a row holds more to show a step into, by what it is rather
    // than by making what it holds, so a place's rows are said without
    // asking each for its own, as a processor's would list every process.
    function showsMore(row) {
        return !!row.verbs || !!row.targets || !!row.nested || !!row.category || !!row.opens
            || !!row.from || (partsOfRows[row.key] ?? []).length > 0;
    }

    // A row's parts as they're named inside it: "Theme" in Slack.
    function partsIn(row) {
        return (partsOfRows[row.key] ?? []).map(part => Object.assign({}, part, {
            name: part.part ?? part.name
        }));
    }

    // The categories (Words.qml), each a row found by its word, whose step
    // in lists what it holds; its word and a space step in from anywhere,
    // and what's typed after searches only inside it.
    readonly property var categories: Words.categories.map(category => {
        // A place wears its own mark, as Machine wears the computer's, so
        // home's places aren't one tag three times over (VKCF3Z,
        // 2026-11-08); one the vocabulary gives none wears the tag.
        const mark = Theme.glyph[category.glyph ?? ""] ?? Theme.glyph.category;
        const row = {
            key: `category:${category.id}`,
            name: category.name ?? category.id,
            verb: "open",
            plain: true,
            category: category,
            glyph: mark,
            hue: "voice",
            chord: [],
            side: "",
            chip: category.id,
            ask: "what in it?",
            lead: mark,
            leadHue: "voice",
            // What it holds are things with verbs of their own, as a
            // collection's are.
            nested: true,
            named: true,
            weight: 1,
            verbs: () => root.members(category)
        };
        row.prepared = Fuzzy.prepare(row.name);
        return row;
    })

    // Where Joel is, as the field shows it: each step as its row named it,
    // in the rows' own case (Shortcuts, Desktop), so the path reads as the
    // rows he picked did. A step named as the one before it (the Shortcuts
    // category's Shortcuts) is said once. A collection standing in a place
    // of its own, as the file picker's lists /, says it after its title
    // while nothing below it is held, as each folder below says its own,
    // so the path never loses where he is (VKCF3Z, 2026-11-05).
    readonly property var path: {
        const steps = held.reduce((said, step) => {
            const own = step.row.title ?? step.row.name;
            const last = said.length > 0 ? said[said.length - 1] : "";
            return own.toLowerCase() === last.toLowerCase() ? said : said.concat([own]);
        }, []);
        const place = held.length === 1 ? held[0].row.from?.collection?.place : undefined;
        return typeof place === "string" && place !== "" ? steps.concat([place]) : steps;
    }

    // Words with their first letter a capital, as a row's name starts.
    function capitalised(words) {
        return words === "" ? "" : words[0].toUpperCase() + words.slice(1);
    }

    // The category held, or null.
    readonly property var category: held.length > 0 ? held[held.length - 1].row.category ?? null
        : null

    // When a place holds something only some of the time, as Power holds
    // Kept From Resting only on battery: the vocabulary names the
    // condition (a category's `while`, by what it holds), and this says
    // what each name means, so words.json never carries code and a new
    // condition is one line here. Each is read as a place resolves, never
    // through a provider's answer, so nothing a provider says can loop
    // back into what it's asked for.
    //
    // Whether a thing is there is decided as the place is made
    // (VKCF3Z, 2026-10-29). One whose condition stops holding while it's
    // shown stays where it was, dimmed, as Steady keeps any row gone, and
    // says why in the place's own words (its `said`, by the condition's
    // name). One whose condition starts holding while it's shown comes at
    // the end, after the commands, as anything new in a list does (the
    // Lists rule in docs/design.md), and takes its place the next time
    // the place is opened. That's on purpose: nothing moves under Joel.
    readonly property var conditions: ({
        "on battery": () => UPower.onBattery
    })

    // Whether a place holds this now. A condition with no meaning here
    // holds its thing back, so a word missing from the table shows nothing.
    // The condition is looked up and then called, never called through
    // ?.() on the lookup, which Qt 6's QML compiler crashes compiling.
    function holdsNow(category, id) {
        const named = (category.while ?? {})[id];
        if (named === undefined)
            return true;
        const condition = conditions[named];
        return typeof condition === "function" && condition() === true;
    }

    // What a place holds now, in its order: what its words name, less
    // what a condition holds back.
    function holdings(category) {
        return (category.holds ?? []).filter(id => holdsNow(category, id));
    }

    // What the place shown holds now, by id: for a provider that reads
    // something only while it's shown, as Machine reads the processor's
    // telemetry for Kept From Resting. It's made from the words and the
    // conditions alone, never from an answer.
    readonly property var placed: category === null ? [] : holdings(category)

    // The category a word and a space step into: its own word, or one of
    // its others typed whole.
    // A place by its name, its one word, so its name and a space step into
    // it: "machine " is Machine.
    function categoryFor(word) {
        const said = word.toLowerCase();
        return Words.categories.find(category => category.name.toLowerCase() === said) ?? null;
    }

    // What a place holds, in the order the vocabulary gives, which is by
    // importance and relation (attachments/collection-order.md under the
    // launcher's quest; Joel, 2026-10-25: "ordered intelligently by
    // importance and relation"): each thing as it's found anywhere, only
    // those this machine has, then the typed commands whose answers are
    // this place's, last, so they remind him what can be typed here.
    function members(category) {
        return holdings(category).map(id => resolved(id)).filter(row => row !== null)
            .concat(commandsIn(category));
    }

    // The typed commands a place lists, in its order.
    function commandsIn(category) {
        const named = category.commands ?? [];
        return commands().filter(row => named.includes(row.key))
            .sort((a, b) => named.indexOf(a.key) - named.indexOf(b.key));
    }

    // What a category holds, named in its drawer so one key onto it says
    // what's there: its members' names, as their rows name them ("Agents ·
    // AI spend · Calls"), but for one its own name already says (Sound's
    // Sound), never after its name (VKCF3Z, dae7fb67). Read from the
    // vocabulary as its row is made, so no answer is asked for it. Its typed commands are rows
    // inside it but not named here: the words they start with read as
    // stray lowercase words among the places (VKCF3Z, 2026-11-05: "Wi-Fi,
    // Address, Speed, ip, is, split, mac").
    function holding(category) {
        const names = holdings(category).map(id => heldName(id)).filter(name => name !== "");
        const own = (category.name ?? category.id).toLowerCase();
        return names.filter(name => name.toLowerCase() !== own);
    }

    // How a place stands, where the vocabulary names a provider that says
    // so (a category's `stands`): docs/design.md's "a place that measures
    // something says how it stands instead", as Power's row says the charge and the
    // profile where Machine holds it, the way Processor's says how busy it
    // is (2026-10-28: "having power as a top under machine with just a list
    // of options in the hint instead of treated like cpus and stuff is
    // weird"). Asked as its row is made; "" when nothing says, and its
    // right says nothing, its drawer naming what it holds (VKCF3Z, dae7fb67).
    function standsOf(category) {
        if (!category.stands)
            return "";
        const source = sources.find(one => one.settings.name === category.stands);
        return source?.stands?.() ?? "";
    }

    // Names as one line, as many whole as fit in `room` characters, as a
    // place's row says them (PlaceRow.js), for Now's rows to say their
    // parts the same way.
    function listed(cased, room) {
        return PlaceRow.listed(cased, room);
    }

    // The name of a thing a category holds, as it's shown now, or "" where
    // this machine has none or its name waits for a fact ("Sign in to
    // {network}").
    function heldName(id) {
        const place = placeNamed(id);
        if (place)
            return place.name;
        const known = keyed[id];
        if (known && !known.category)
            return known.name;
        const entry = Words.of(id);
        const there = sources.some(one => one.settings.name === (entry.provider ?? id));
        return there && !entry.name.includes("{") ? entry.name : "";
    }

    // A category another holds, as Machine holds Power, by its id: its own
    // row, stepped into as one at home is. Undefined for anything else.
    function placeNamed(id) {
        return categories.find(row => row.category.id === id);
    }

    // A place's drawer's eyebrow, its kind, as a program's says "program"
    // (VKCF3Z, 00ca9231).
    readonly property string placeSaid: (Words.field.drawer ?? {}).place ?? ""

    // A thing the vocabulary names, as a row: a category, the row of that
    // key, a collection whole, an answer to its question, or a provider's
    // things as one row holding them. Null where this machine has none.
    function resolved(id) {
        const place = placeNamed(id);
        if (place) {
            // How it stands at its right, as Power's charge, or nothing
            // where it only groups others, and what it holds by name in
            // its drawer, at home and inside Machine alike (VKCF3Z,
            // dae7fb67).
            const names = holding(place.category);
            return Object.assign({}, place, {
                side: PlaceRow.side(standsOf(place.category), ""),
                count: "",
                drawer: PlaceRow.drawer(place.name, names, placeSaid)
            });
        }
        const known = keyed[id];
        if (known && !known.category)
            return known;
        const entry = Words.of(id);
        const source = sources.find(one => one.settings.name === (entry.provider ?? id));
        if (!source)
            return null;
        if (source.collection && !entry.question)
            return whole(source, false);
        const question = entry.question ?? source.questions[0];
        if (question !== undefined) {
            const said = source.answer(question);
            // An answer is held with its parts, or whole where it says it
            // stands on its own, as Kept From Resting does in Power with
            // nothing keeping it from resting. One that can't be read now
            // stays where it is, dimmed, so the rows under it don't move.
            return parted(said) || said?.whole === true
                ? inPlace(Object.assign(answered(heard(said, source, question)), {
                    is: `answer:${id}`,
                    chip: question,
                    gone: said.gone === true
                }), entry.name.includes("{") ? "" : entry.name) : null;
        }
        const things = source.things.filter(row => !(row.key.startsWith("action:")
            && standIns.includes(row.name)));
        if (things.length === 0)
            return null;
        const row = {
            key: `group:${id}`,
            name: entry.name,
            verb: "open",
            plain: true,
            glyph: Words.glyph(id),
            hue: "voice",
            chord: [],
            // How many at its right, where a row's figure stands (VKCF3Z,
            // 2026-11-05: one place for each kind of fact).
            side: String(things.length),
            chip: entry.name.toLowerCase(),
            title: entry.name,
            nested: true,
            group: true,
            verbs: () => things
        };
        row.prepared = Fuzzy.prepare(row.name);
        return row;
    }

    // Every command that takes something after it, its form shown, from
    // the providers this machine has: Return or Tab puts its start in the
    // field, out at the top, so it's finished there. What it does is said
    // after its form, where a row says what it is or does, never at its
    // right, which holds a figure or a state (VKCF3Z, 2026-11-06).
    function commands() {
        return Words.commands.filter(command => sources.some(source => source.settings.name
            === command.provider)).map(command => {
            const row = {
                key: command.id,
                name: `${command.start}${command.form}`.trim(),
                form: command.form,
                start: command.start,
                verb: "type",
                glyph: Theme.glyph.command,
                hue: "action",
                chord: [],
                side: "",
                count: command.says ?? "",
                plain: true
            };
            row.prepared = Fuzzy.prepare(row.name);
            return row;
        });
    }

    // A collection's parts the vocabulary names: Agents holds its tasks,
    // helpers and spend, the Keyboard its keyboards and shortcuts.
    function partsOf(source) {
        return Words.of(source.settings.name).parts.map(id => resolved(id))
            .filter(row => row !== null);
    }

    // Rows found by what's typed, inside a step, a category or a
    // collection's parts, in the top level's tiers (Rank.js): a name that
    // is what's typed, then one starting with it, then one with a later
    // word starting with it, and then a row found only by what it holds of
    // its own (hiddenWords), each tier best match first. Such a word isn't
    // said beside the row, since it may be a name the machine knows a thing
    // by, such as tailscale0, that Joel never sees. Within a tier, one
    // picked before for what's typed comes first, as at the top (`picked`,
    // Picks.qml's weights), so the same letters land on the same row inside
    // a step too.
    function within(rows, letters, picked) {
        const typedWords = Fuzzy.words(letters);
        const need = Fuzzy.letters(typedWords);
        const found = [];
        for (const row of rows) {
            const prepared = row.prepared ?? Fuzzy.prepare(row.name);
            const byName = (prepared.mask & need) === need
                ? Fuzzy.match(prepared, typedWords, false) : { score: -1 };
            if (byName.score >= 0) {
                found.push({
                    item: prepared,
                    score: byName.score,
                    grade: byName.grade,
                    tier: Rank.tier(row.name, letters, false),
                    choice: row
                });
                continue;
            }
            const known = hiddenWords(row);
            if (!known || (known.mask & need) !== need)
                continue;
            const byWord = Fuzzy.match(known, typedWords, false);
            if (byWord.score >= 0)
                found.push({
                    item: prepared,
                    score: byWord.score,
                    grade: byWord.grade,
                    tier: Rank.wordTier(known.text, letters),
                    choice: row
                });
        }
        return Rank.order(found, { picked: picked ?? {} }).map(f => f.choice);
    }

    // A row's other words, prepared to match: its keywords, which say only
    // what it holds of its own, such as a network card's name, since the
    // vocabulary gives a thing one word, its name. Made once for each row
    // and kept on it, since a step's rows are searched at every key; a
    // WeakMap keeping them crashed the shell once its rows were collected.
    function hiddenWords(row) {
        if (row.hiddenMade !== undefined)
            return row.hiddenMade;
        const words = row.keywords ? row.keywords.text ?? "" : "";
        const made = words === "" ? null : Fuzzy.prepare(words);
        row.hiddenMade = made;
        return made;
    }

    // What's typed in a category: what it holds, and what's held in the
    // things it holds as one row (Programs' programs), found by their
    // words, then each of its collections' best match, as at the top. A
    // row named what's typed, whole, comes first wherever it was found, so
    // "workspaces" in Shortcuts is its Workspaces group, before any chord
    // or act that only uses the word.
    function inCategory(letters) {
        const pool = offered.reduce((all, row) => all.concat([row], row.group ? row.verbs() : []),
            []);
        const inside = offered.filter(row => row.from).map(row => row.from);
        const here = held[held.length - 1].row.key ?? "";
        const picked = Rank.joined(picks.weights(letters),
            picks.weights(Rank.stepped(here, letters)));
        return namedFirst(within(pool, letters, picked).concat(gathering(letters, inside)),
            letters);
    }

    // Rows named what's typed, whole, before the rest, each in its order.
    function namedFirst(rows, letters) {
        const said = Fuzzy.plainly(letters);
        const named = row => Fuzzy.plainly(row.name) === said;
        return rows.filter(named).concat(rows.filter(row => !named(row)));
    }

    // What a category's own answers make of what's typed in it: in
    // computer, "battery".
    function answeringIn(category, letters) {
        if (letters.trim() === "")
            return null;
        // A part another thing holds, as Power holds Processor's Kept From
        // Resting, doesn't make the place answer all its provider's words.
        const names = holdings(category).filter(id => Words.of(id).in === undefined)
            .map(id => Words.of(id).provider ?? id);
        for (const source of sources.filter(one => names.includes(one.settings.name))) {
            const said = source.answer(letters);
            if (said)
                return heard(said, source, letters);
        }
        return null;
    }

    // Whether the category's word and what's typed after it ask something
    // whole ("time in tokyo", "time left", "weather tokyo"): a question,
    // not a search inside it, so the field gives it back whole. An answer
    // there only when nothing else is, as Ask's for any two plain words,
    // asks nothing whole, or "network a" would leave Network on its
    // second letter.
    function askedWhole(letters) {
        if (held.length !== 1 || Fuzzy.words(letters).length === 0)
            return "";
        const whole = `${held[0].typed} ${letters}`;
        return sources.some(source => {
            const said = source.answer(whole);
            return !!said && !said.fallback;
        }) ? whole : "";
    }

    // The places a category sits in, outermost first, as Machine holds
    // Power; none for one at home.
    function placesAbove(category) {
        const above = Words.categories.find(one => (one.holds ?? []).includes(category.id));
        return above ? placesAbove(above).concat([above]) : [];
    }

    // Steps into a category from a word and a space, or from the top,
    // through the places it sits in, so the path says where it lives and
    // - climbs out through them; what was typed is put back on
    // the way out.
    function intoCategory(category, word) {
        const chain = placesAbove(category).concat([category]);
        const steps = [];
        // Where each step was stepped in from: the field for the first,
        // then the place before it, its row for the next one selected.
        let from = Object.assign(here(), { typed: word });
        chain.forEach((place, at) => {
            const rows = members(place);
            steps.push(Object.assign(from, {
                row: categories.find(one => one.category === place),
                rows: rows
            }));
            const inner = rows.findIndex(one => one.category === chain[at + 1]);
            from = { typed: "", at: Math.max(0, inner), key: inner < 0 ? ""
                : steadyId(rows[inner]), scroll: 0 };
        });
        held = steps;
        restart("");
    }

    // Puts a command's start in the field, out at the top, where it's
    // finished.
    function fill(start) {
        held = [];
        restart(start);
    }

    // What a collection says at its right, { side, sideItems }, its items
    // made once: said as items, its side is them joined, as Now's first
    // thing waiting is, else what it says whole (homeSays), null for
    // nothing of its own.
    function sidesOf(source) {
        const items = source.homeItems();
        return items.length > 0
            ? { side: items.map(item => item.said).join(" \u00b7 "), sideItems: items }
            : { side: source.homeSays(), sideItems: [] };
    }

    // A collection as a row the search finds: as it is at home, but naming
    // and counting none of what it holds, which would read all of it at
    // every change. One that says how it stands without reading it, as
    // Agents says how its sessions stand (homeSays), says that at its
    // right, its answer by name, read as the launcher opens, as home is.
    function named(source) {
        const collection = source.collection;
        const says = sidesOf(source);
        const row = Object.assign(whole(source, false, false), { weight: source.weight },
            says.side !== null ? says : {});
        row.prepared = Fuzzy.prepare(row.name);
        row.keywords = Fuzzy.prepare([row.name, collection.noun].join(" "));
        // Its words are what a person calls it, so they count as its name.
        row.named = true;
        return row;
    }

    // A row for each collection that has matches: its best, with what's been
    // picked for this query before put first, and a count of the rest. Its
    // verbs are the matches, which Return steps into with what was
    // typed kept. One letter finds too much of a collection to be worth a
    // row.
    property var gathered: []

    function gathering(letters, among) {
        // What each collection took, in milliseconds, for a key that's slow
        // to say which (Trials.qml). Made empty first, so a key too short to
        // search them doesn't say what the key before it took.
        const took = {};
        listsTook = took;
        if (letters.replace(/\s+/g, "").length < 2)
            return [];
        const rows = [];
        // And how many texts each made ready to match, which it should have
        // had made while closed (textsMadeClosed).
        const made = {};
        for (const source of among ?? collections) {
            // One opened only by its own chord adds no row to a search, but
            // does inside a category that holds it.
            // One of its parts may still be gathered, as Capture's Recent
            // is (`gathers`), stepped into on Return with what's typed kept.
            const part = source.collection.searched === false && !among
                ? source.gathers?.(letters) ?? null : null;
            if (source.collection.searched === false && !among && part === null)
                continue;
            const asked = Date.now();
            const madeBefore = Fuzzy.keptSoFar();
            const said = part ?? source.gather(letters);
            took[source.settings.name] = Date.now() - asked;
            if (Fuzzy.keptSoFar() > madeBefore)
                made[source.settings.name] = Fuzzy.keptSoFar() - madeBefore;
            // Inside a category, its best is one named what's typed before
            // one picked for it.
            const learned = learnedFirst(said.rows, letters);
            const found = among ? namedFirst(learned, letters) : learned;
            if (found.length === 0)
                continue;
            const best = found[0];
            const noun = source.collection.noun;
            // What it belongs to leads, as a session ranked flat says whom
            // it's under, the depth that showed it gone (Agents.qml), and its
            // noun only where no kind says it (Rank.gatheredSide).
            const gathered = Object.assign({}, best, {
                side: Rank.gatheredSide(best.belongs ?? "", found.length, noun,
                    kindOf(best) !== ""),
                chip: noun,
                title: source.collection.title,
                ask: source.collection.ask,
                lead: source.collection.glyph,
                leadHue: "voice",
                from: source,
                // Found by a word of its name, after the main search's own
                // (Rank.js's placed).
                tier: 1,
                verbs: () => found
            });
            // One with no act of its own, only more inside (an
            // orchestrator, a device), lands inside it on Return, its
            // collection's crumb before its own, as the approved proposal's
            // one feel has it, and the right arrow lands there too, so
            // Return never goes further in than it (Joel, 2026-10-26).
            if (part !== null)
                Object.assign(gathered, {
                    stays: true,
                    lands: true,
                    run: () => root.searchIn(source, part.into, letters)
                });
            else if (!best.run && (best.verbs || best.targets))
                Object.assign(gathered, {
                    stays: true,
                    lands: true,
                    run: () => root.landInside(gathered, best)
                });
            // Ctrl+] on any found thing that holds more goes straight to
            // where it lives, whatever its Return does (VKCF3Z, 2026-10-29:
            // "the crumbs are where it lives").
            else if (best.verbs || best.targets)
                gathered.landsIn = best;
            rows.push(gathered);
        }
        if (Object.keys(made).length > 0)
            Report.say(`texts made ready on a key ${JSON.stringify(made)}`);
        return rows;
    }

    // Steps into a collection's part, through the collection, and searches
    // it for what was typed, as Return on Recent's matches gathered at the
    // top does: - climbs back out to the search.
    function searchIn(source, part, letters) {
        into(source.settings.name);
        const at = matches.findIndex(one => steadyId(one) === steadyId(part));
        if (at >= 0)
            current = at;
        if (enter(at >= 0 ? matches[at] : part))
            restart(letters);
    }

    // Steps into a collection's matches, and then into the one of them that
    // was found, which stepping in keeps selected.
    function landInside(gathered, best) {
        if (enter(gathered) && selected && steadyId(selected) === steadyId(best)
            && enter(selected))
            // - comes back past the collection to the search.
            held[held.length - 1].through = true;
    }

    // How tall the list is while the launcher's open: always its rows, and
    // the gap of a page that opens with one.
    // Set as it opens, from the rows the screen gives.
    property int tall: 8 * rowTall

    // What kind of thing a row is, as it says at its right edge, by what
    // its provider calls its things: a closed set of nouns, so a kind is
    // learned once. An act or a set says what it is by its words, and an
    // answer's rows are the answer's, so they have none.
    // They're jitsusama.quickshell.launcher.kinds, so a machine can say them
    // its own way.
    readonly property var kinds: Providers.launcher.kinds

    function kindOf(row) {
        if (row && row.answered)
            return row.missed ? "" : "answer";
        if (!row || row.opens || row.plain)
            return "";
        return kinds[row.what ?? ""] ?? "";
    }

    // The one word the foot gives Return: the row's own, or else an act's
    // first word ("forget", "disconnect"), which its name starts with, and
    // which way for a turn ("turn off"), or "open" for a thing.
    function verbOf(row) {
        if (!row)
            return "";
        if (row.returns)
            return verbOf(row.returns);
        if ((row.verb ?? "") !== "")
            return oneWord(row.verb);
        return kindOf(row) !== "" ? "open" : oneWord(Rank.verbIn(row.name ?? "") || "open");
    }

    // What the keys do for a row, as the foot says them, a key and its
    // word each: Return's verb, Shift+Return where it can, where the arrows
    // step. Tab is said in the field, at its right edge while there's a
    // completion to take (Prompt.qml), not here: it comes and goes with
    // each letter, so in the foot it moved the keys or took room from how
    // things stand. The left arrow is said only where it goes back: that it
    // takes away what's typed is what any field does, and saying so took
    // room from how things stand as soon as Joel typed. Shift+Return and
    // the right arrow are said wherever they work, a step in too (Joel,
    // 2026-10-26: "the foot shows every key that works on the selection"):
    // the right arrow where `enter` finds somewhere to go, by what it would
    // ask, never by asking it, since the foot is said for every row.
    // Whether Return does something and keeps the launcher, as a setting
    // does, rather than going somewhere inside it, as a spot, a row with
    // more inside, one holding rows (its verbs and targets both, as a
    // meeting's Show in Calendar) or a collection's match does.
    function keepsOpen(choice) {
        return !!choice.stays && !!choice.run && !choice.spot && !choice.from
            && verbOf(choice) !== "more" && !(choice.verbs && choice.targets)
            && !copiesOnly(choice);
    }

    // Whether a row's only act is copying what it shows while it holds
    // more: a figure, as a disk's Space and Power's Battery are; an answer
    // to what was typed, as Memory is, wherever it's asked; or a row that
    // copies its name for want of a window to go to, as a job or a council
    // with no session does (`figure: false`). Return steps in, and y
    // copies, with the copy listed inside after what can be done
    // (Catalogue.js's leading). Return is the one way in now that l steps
    // nowhere, so a copy on it hid the parts (Joel, 2026-11-06: "i type
    // memory in the launcher, hit enter and it copies and does not go into
    // more details"). A sum holds nothing, so its Return still copies, and
    // a thing Return pastes, as an emoji or a copy kept, keeps its Return.
    function copiesOnly(choice) {
        return !!choice && (choice.figure !== undefined || choice.answered === true)
            && verbOf(choice) === "copy" && holdsMore(choice);
    }

    // Whether a row holds a thing's insides (its verbs, Return's act first,
    // then its parts) rather than a verb's targets: wherever things are
    // listed, at the top or in a collection, and anywhere for a row with
    // verbs and no targets, so a row's insides don't depend on the level
    // it's shown at (Joel, 2026-10-26: one thing, one set of insides). Inside
    // a program, its Shortcuts and its windows held nothing.
    function asThing(row, among) {
        return among || (!row.targets && !!row.verbs);
    }

    // Whether a row holds more to step into, as `enter` would find: a thing's
    // insides, a verb's targets in a step, a place found from the top, or an
    // ask.
    function holdsMore(choice) {
        if (held.length > 0 && choice.given)
            return true;
        // One that keeps the launcher holds its own act, which closes it.
        // A copy never keeps it, and asking would ask copiesOnly, which
        // asks this.
        if (verbOf(choice) !== "copy" && keepsOpen(choice)
            && (asThing(choice, held.length === 0 || collecting)
            || !choice.targets))
            return true;
        if (held.length === 0 && (choice.category || choice.inside))
            return true;
        return asThing(choice, held.length === 0 || collecting) ? !!choice.verbs
            : !!choice.targets;
    }

    // Whether a row with nothing inside it goes somewhere else in the
    // launcher on Return: a spot to where it is, a command into the field
    // to be finished, one that opens a collection into it.
    function leadsElsewhere(choice) {
        return !!choice && !choice.gone && !holdsMore(choice)
            && (choice.trail !== undefined || choice.start !== undefined || !!choice.opens);
    }

    // Whether Return steps into a row, as `run` decides: a place or a
    // holder, a verb wanting a target, one that can't be taken back, or one
    // with nothing to do of its own. Such a row says open, so stepping in
    // has one word wherever it's done (Joel, 2026-10-26), where it was
    // choose, go, see or the act asked about.
    function stepsIn(choice) {
        if (leadsElsewhere(choice) || copiesOnly(choice))
            return true;
        // One whose Return does one of its acts does it, never stepping in.
        if (choice.returns)
            return false;
        if (choice.start !== undefined || choice.given || !holdsMore(choice))
            return false;
        if (choice.category || choice.group || choice.lands || !choice.run)
            return true;
        if (held.length > 0 && choice.targets && choice.verb === "more")
            return true;
        return !!choice.confirm && (held.length === 0 || collecting);
    }

    // Whether Return on a row changes how things stand (Catalogue.js's
    // changesState), as the key language flags the command the row is
    // named by, so Return lands where d asks.
    function changesState(choice) {
        const act = returnOf(choice);
        return !!act && Catalogue.changesState(act, commandNamed(act));
    }

    // What Return does on a row: the act it names to (`returns`), as a
    // session's Return does what its state makes harmless, Talk for one
    // listening and Resume for one paused (VKCF3Z, 2026-11-06), while its
    // step in still holds all its acts; or else the row's own. Every way
    // Return is read (run, lands, endsSomething, the foot's word and the
    // report's verb) reads this, so they never disagree.
    function returnOf(row) {
        return row?.returns ?? row;
    }

    // An act's own words for its ask, where it has them (Confirm.question),
    // as Reboot's "Reboot the Machine, Closing 12 Windows", read as it's
    // asked, since what it closes changes.
    function askedOf(act) {
        const asked = typeof act?.asked === "function" ? act.asked() : act?.asked;
        return typeof asked === "string" ? asked : "";
    }

    // What an act asked from its own row is an act of, for its ask
    // (Confirm.actOf): the thing holding it, never a place.
    function askedOfHolder(holder, act) {
        return Confirm.actOf(holder, act, holder ? Catalogue.kindOf(holder) : "");
    }

    // The command a row is named by (Catalogue.js's named), from its words
    // or its verb, as the command line names its act, and the key
    // language's flags for it.
    function commandNamed(row) {
        return Catalogue.named(commandWords(row).command, Commands.commandOf(row),
            Options.commands);
    }

    // Whether Return steps into a thing whose own act changes something, as
    // a network or a device is, from the find and from normal alike, where
    // its act is only done from inside it (the vi-paradigm spec's Return):
    // a name typed and Return never joins, disconnects or forgets. Only the
    // thing's own row, found from the top or in its collection: a part
    // inside one, as an output's Muted, is a switch that Return switches.
    function stepsThing(choice) {
        const kind = kindOf(choice);
        return !!choice && prompt.modal && changesState(choice)
            && (kind === "network" || kind === "device") && (held.length === 0 || collecting)
            && holdsMore(choice) && !stepsIn(choice);
    }

    // Whether Return from the find lands on a row rather than doing it: one
    // that changes something, which leaves the line for normal on it, saying
    // what a second Return does (Joel, 2026-11-04: "it is quite dangerous
    // how in insert mode i can type in a name of a wifi network and then
    // hitting enter disconnects from it"). Normal's Return does it.
    // One that asks for words first asks, as Join by Code does, and one
    // asking Joel to be sure is done, since stepping in was the second press.
    // A meeting's row joins, as its row says (Catalogue.js's startsItself).
    function lands(choice) {
        const act = returnOf(choice);
        return !!choice && prompt.modal && prompt.mode === "find" && changesState(choice)
            && !stepsIn(choice) && !stepsThing(choice) && !act.given && !act.sure
            && !sureOf() && !Catalogue.startsItself(act, commandNamed(act))
            && !endsSomething(choice);
    }

    // Whether the step Joel is in asks him to be sure, as Power Off's does.
    function sureOf() {
        return held.length > 0 && !!held[held.length - 1].row?.confirm;
    }

    // Whether a row's act ends, takes away or reaches someone, which no
    // Return runs from any mode (Catalogue.js's endsSomething): the key
    // language's ends flag on its command, or :set's no form for an option
    // whose off ends what it holds (its `ends`, Provider.qml), as Wi-Fi
    // Off's :set nowifi drops the network, where :set nomic ends nothing.
    function endsSomething(choice) {
        return !!choice && prompt.modal && actEnds(choice);
    }

    // Whether a row's act ends something, wherever it's asked from: a
    // terminal's `launcher do` asks it with no line in normal mode.
    function actEnds(choice) {
        const act = returnOf(choice);
        const named = commandNamed(act);
        const set = named.command === "set"
            ? SetWords.setOne(String(commandWords(act).argument ?? ""), setOptions) : null;
        const off = set?.how === "off" && setOptions[set.option]?.ends === true;
        return Catalogue.endsSomething(act, named, off);
    }

    // The line Return writes for a row that ends something, as the menu
    // would write its act chosen: "disconnect", "set nowifi". A step that
    // asked Joel to be sure, as Log Out Now, was the asking, so its line is
    // banged, "logout!", and runs past its confirm rather than asking again.
    function endsLine(choice) {
        const thing = held.length > 0 ? held[held.length - 1].row : choice;
        const line = Commands.actLine(returnOf(choice), true, { called: commandWords,
            table: Options.commands, thing });
        const asked = (!!choice.sure || sureOf()) && line.bang;
        return Commands.written(asked ? Object.assign({}, line, { banged: true }) : line, "");
    }

    // What an ending row's side names for the key that carries it: d with
    // its verb where the row is d's own ending act (Operators.endingOf), else
    // the line Return writes, ":disconnect" (VKCF3Z, 2026-11-05). A row
    // whose side says how it stands rather than what it does, as Sleep's
    // does, says `carries: false`, and the foot alone names its line
    // (VKCF3Z, 2026-11-08).
    function carriedBy(choice) {
        if (!endsSomething(choice) || choice.carries === false)
            return "";
        if (Operators.endingOf(choice, []) === choice) {
            const key = Object.keys(Operators.operators)
                .find(one => Operators.operators[one] === "ending") ?? "";
            return `${key} ${verbOf(choice)}`;
        }
        return `:${endsLine(choice)}`;
    }

    // When Return last wrote an ending row's line, so a Return doubled
    // with it, a key bounced, doesn't run the line it opened.
    property real linedAt: 0

    // Return on a row that ends something: its command written on the :
    // line, the menu naming what it acts on, for a fresh Return to run.
    function lineFor(choice) {
        commandLine({ typed: endsLine(choice) });
        linedAt = Date.now();
    }

    // What the last Return did ("ran", "landed", "entered", "gave", "lined", or ""
    // for one doubled), and when it was pressed, heard and done, in
    // milliseconds of the monotonic clock, so a test sees that a name and
    // one Return opens with no wait between.
    property var returned: ({})

    // The row Return last landed on, by its steady id, and the keys the
    // line had taken then, so what it says lasts only until the next key.
    property string landedKey: ""
    property int landedKeys: -1
    property real landedAt: 0

    function land(choice) {
        prompt.toNormal();
        landedKey = steadyId(choice);
        landedKeys = prompt.keysTaken;
        landedAt = Date.now();
    }

    // Whether a row is the one landed on, until the next key.
    function landedOn(choice) {
        return !!choice && landedKey !== "" && prompt.keysTaken === landedKeys
            && steadyId(choice) === landedKey;
    }

    // Whether Return comes so soon after a landing that it's the same
    // Return doubled, a key bounced or pressed twice for one, which does
    // nothing, as a confirm's second Return waits the same while.
    function landedJustNow() {
        return landedKey !== "" && Date.now() - landedAt < Options.keyTiming.wait;
    }

    // The keys the foot names while the drawer holds them, where the row's
    // Return and arrows do nothing: the way back to the rows and the key
    // list where it's bound there, each as the table first names it
    // (VKCF3Z, 2026-11-05: a lone Return in the drawer named a key that did
    // nothing). Choosing names no way back, since only Escape leaves it,
    // and nothing names Escape, which only ever leaves a mode; VIS and the
    // count already say where you are (VKCF3Z, 2026-11-05).
    function drawerKeys(mode) {
        const words = Words.field.drawer ?? {};
        const named = act => QueryKeys.table.find(one => one.act === act
            && QueryKeys.inMode(one, mode));
        const back = QueryKeys.chosen(mode) ? null : named("rows");
        const list = named("keys");
        return [back ? [ListedKeys.sequenceFace(back.keys[0]), words.back ?? ""] : null,
            list ? [ListedKeys.sequenceFace(list.keys[0]), words.keys ?? ""] : null]
            .filter(key => key !== null);
    }

    function keysFor(choice) {
        if (QueryKeys.paged(prompt.mode))
            return drawerKeys(prompt.mode);
        // A row whose thing has gone does nothing, so it says no key.
        if (!choice || choice.gone)
            return [];
        // In visual, Return opens the selection's rows it can, by count,
        // and the rest is as normal's foot: :, d by the cursor row's ending,
        // y and Space ?, so the way to every key stays (VKCF3Z, 2026-11-05).
        // The colon is drawn furthest from Return, so it gives way first
        // where room is short: Space ? lists it, and d and y are what
        // visual is for.
        if (visual) {
            const hints = Words.field.hints ?? {};
            const opens = visualRows().filter(row => Catalogue.opens(row, commandNamed(row)));
            return [["\u21b5", Operators.opensSaid(opens.length, Words.field.visual ?? {})],
                [":", hints.commands ?? ""]]
                .concat(operatorKeys(choice), [[hintKey("keys", "visual"), hints.keys ?? ""]]);
        }
        // The row landed on says the act a second Return does, in the
        // accent, since it's the one key that changes something now; the
        // rest is normal's foot, as a landing is normal, so its shape never
        // changes (VKCF3Z, 2026-11-05).
        if (choice === selected && landedOn(choice)) {
            const hints = Words.field.hints ?? {};
            return [["\u21b5", verbOf(choice), Theme.accent, Theme.accent],
                [":", hints.commands ?? ""], [hintKey("keys"), hints.keys ?? ""]];
        }
        // A row that ends something names the line Return writes at the
        // right end, where Return always is, and before it the key that
        // carries it at once (VKCF3Z, 2026-11-05).
        if (endsSomething(choice))
            return endingKeys(choice);
        // One Return lands on from the find says the act Return leads to,
        // though its first press only lands, since a glyph with no word
        // tells Joel nothing where every other key says what it does
        // (VKCF3Z, 2026-11-07; design.md's Return table), and no arrow,
        // since the find's arrows move the caret.
        if (lands(choice))
            return [["\u21b5", verbOf(choice)]];
        // Going elsewhere keeps Return's own word, where it takes him.
        // Return going in says open, on the find and in normal alike, as
        // oil's Return opens a folder, it being the one way in: stepping in
        // has one word wherever it's done (1JQ9TO, 2026-11-05).
        const more = leadsElsewhere(choice) ? verbOf(choice)
            : ((Words.field.hints ?? {}).open ?? "");
        const stepping = stepsIn(choice) || stepsThing(choice);
        // Normal names its keys by letter and lowercase verb, from the
        // table that does them (the vi-paradigm spec's foot): - up, :
        // commands and Space ?, the way to every key, always.
        if (prompt.modal && prompt.mode === "normal")
            return normalKeys(choice, stepping, more);
        // No arrow is named: where a line is typed they move its caret, and
        // nothing else steps by them (Joel, 2026-11-05). One with nothing to do yet (`still`), as the file chooser's Open
        // the Marked before anything is marked, says no Return at all
        // rather than a word Return wouldn't do (VKCF3Z, 2026-11-03).
        const keys = choice.still ? [] : [["\u21b5", stepping ? more : verbOf(choice)]];
        keys.push(...findTaught(choice));
        // Where the line holds a prompt's words, it says how to get to
        // normal's keys and to the list of them, just before Return
        // (VKCF3Z, 2026-11-05).
        if (holdsWords) {
            const said = Words.field.footKeys ?? {};
            if (prompt.mode === "insert")
                keys.push(["Esc", said.normal ?? ""]);
            keys.push(["Space ?", said.keys ?? ""]);
        }
        return keys;
    }

    // What the find teaches in the foot, by key and lowercase word, since
    // its line holds only the find and how far it narrows (VKCF3Z,
    // 2026-11-05): the key that takes the completion shown; less to type
    // where what's typed is longer than it need be, worked out once, since
    // each start of what's typed asks every pick; and a thing's chord,
    // since its row says its kind where a chord would be. Only the
    // selection's, as the foot says, so the rows' widths never ask it.
    function findTaught(choice) {
        if (!prompt.modal || prompt.mode !== "find" || choice !== selected)
            return [];
        const hints = Words.field.hints ?? {};
        const taught = [];
        // A row Return opens that holds parts, as a program its commands
        // and windows, says Tab goes into them, first, furthest from
        // Return, as normal's foot does, so nothing slides as it comes and
        // goes with the row (Joel, 2026-11-05: "in programs i don't
        // understand how to go to sub commands on a program"). It's the
        // most needed, though, so the others give way before it
        // (footYielding).
        const stepping = stepsIn(choice) || stepsThing(choice);
        if (!stepping && !choice.still && (holdsParts(choice) || !!choice.landsIn))
            taught.push([hintKey("into", "find"), hints.in ?? ""]);
        if (typed !== "" && prompt.ghost !== "")
            taught.push([prompt.completeKey, hints.complete ?? ""]);
        const shorter = enough(choice);
        if (shorter !== "")
            taught.push([shorter, hints.enough ?? ""]);
        else if (taughtChord(choice).length > 0)
            taught.push([chordText(taughtChord(choice)), hints.nextTime ?? ""]);
        return taught;
    }

    // The chord the foot teaches for a row its row doesn't draw: a thing's,
    // whose row says its kind where a chord would be, and a collection's,
    // which opens it straight (whole), at home or not.
    function taughtChord(choice) {
        return kindOf(choice) !== "" ? choice.chord ?? [] : choice.nextTime ?? [];
    }

    // The find's keys that give way first, short of room, in order of
    // need rather than as drawn: the completion, whose ghost the line
    // already shows, then less to type or next time, so Tab in and
    // Return never go (VKCF3Z, 2026-11-06: at "k", "Tab in \u00b7
    // Super+\u21b5 next time \u00b7 \u21b5 start", Tab being Ctrl+]'s
    // named twin since 2026-11-07). Normal's Tab in goes first, as it's
    // drawn furthest, then the operators' d and y, which Space ? lists,
    // so - up and : outlast them: in a step such as a filter of For You,
    // - is the one way back (VKCF3Z, 2026-11-08, on a look's foot at 2
    // saying only "Space ? keys \u00b7 \u21b5 open"). Other modes give way
    // furthest from Return first.
    readonly property var footYielding: {
        const hints = Words.field.hints ?? {};
        if (prompt.mode === "find")
            return [hints.complete, hints.enough, hints.nextTime].filter(word => !!word);
        if (prompt.modal && prompt.mode === "normal" && !visual)
            return [hints.in, ...(selected ? operatorKeys(selected) : []).map(key => key[1])]
                .filter(word => !!word);
        return [];
    }

    // An ending row's keys: Return and the line it writes, "\u21b5 :disconnect";
    // on the find, how to get to normal's keys; in normal, - up where a step
    // is held, d with its verb where d carries it, else the command line,
    // and the key list.
    function endingKeys(choice) {
        const hints = Words.field.hints ?? {};
        const keys = [["\u21b5", `:${endsLine(choice)}`]];
        if (prompt.mode !== "normal")
            return keys.concat(findTaught(choice));
        if (held.length > 0)
            keys.push([hintKey("out"), hints.up ?? ""]);
        const carried = carriedBy(choice);
        keys.push(carried.startsWith(":") ? [":", hints.commands ?? ""]
            : [carried.slice(0, carried.indexOf(" ")), carried.slice(carried.indexOf(" ") + 1)]);
        keys.push([hintKey("keys"), hints.keys ?? ""]);
        return keys;
    }

    // Normal's keys for a row: Return's act, which goes in where it steps
    // in, - where there's a place to go up to, then : and Space ?.
    function normalKeys(choice, stepping, more) {
        const hints = Words.field.hints ?? {};
        const keys = choice.still ? [] : [["\u21b5", stepping ? more : verbOf(choice)]];
        // A row that opens on Return but holds parts says Tab goes into
        // them, drawn first, furthest from Return, so it is the first to give
        // way, and Space ? stays just before Return (VKCF3Z, 2026-11-07:
        // "Tab in \u00b7 Space ? keys \u00b7 \u21b5 go to").
        if (!stepping && !choice.still && (holdsParts(choice) || !!choice.landsIn))
            keys.push([hintKey("into"), hints.in ?? ""]);
        if (canGoUp)
            keys.push([hintKey("out"), hints.up ?? ""]);
        keys.push(...operatorKeys(choice));
        // The colon opens the command line from rest (QueryKeys.read).
        keys.push([":", hints.commands ?? ""]);
        keys.push([hintKey("keys"), hints.keys ?? ""]);
        // A place's chord, drawn first, furthest from Return, so it gives
        // way first (VKCF3Z, 2026-11-06).
        if ((choice.nextTime ?? []).length > 0)
            keys.splice(1, 0, [chordText(choice.nextTime), hints.nextTime ?? ""]);
        return keys;
    }

    // A key as the foot names it: the first the table gives an act in a
    // mode, normal unless said, drawn as the key list draws it, so a hint
    // names the key that does what it says.
    // Found once an act and mode, since the table never changes and the
    // foot asks it for every listed row as the rows change (2026-11-10).
    function hintKey(act, mode) {
        const asked = `${act}\u241f${mode ?? "normal"}`;
        const known = hintKeys[asked];
        if (known !== undefined)
            return known;
        const row = QueryKeys.table.find(one => one.act === act
            && QueryKeys.inMode(one, mode ?? "normal"));
        const key = row ? ListedKeys.sequenceFace(row.keys[0]) : "";
        hintKeys[asked] = key;
        return key;
    }
    readonly property var hintKeys: ({})

    // d and y where the row has an ending or a copy that its Return isn't,
    // d by the ending's own verb, as docs/design.md's foot says "d
    // disconnect", each by the operator's key (Operators.operators). Only
    // the one row's actions are read, as pendingSaid reads them, and only
    // once it's selected (selectedActions), never in the foot's binding.
    function operatorKeys(choice) {
        return choice === footActions.row ? footActions.keys : [];
    }

    // d and y for a row, from its actions, as operatorKeys says them.
    function operatorKeysOf(choice, actions) {
        const ending = Operators.endingOf(choice, actions);
        const copy = Operators.copyOf(choice, actions,
            one => verbOf(one) === "copy" && !!one.run);
        const keyOf = act => Object.keys(Operators.operators)
            .find(key => Operators.operators[key] === act) ?? "";
        const keys = [];
        // d says what it ends by, as the pending operator's room does
        // (pendingSaid): a window's close, a notice's read or forget, never
        // the verb a row's own action may lack.
        if (ending !== null && ending !== choice)
            keys.push([keyOf("ending"), ending.ends || verbOf(ending)]);
        // A row whose own act is the copy says y only where Return goes in
        // instead, as an answer holding parts does, since there y is the one
        // way to copy it.
        if (copy && (copy !== choice || copiesOnly(choice)))
            keys.push([keyOf("copy"), (Words.field.hints ?? {}).copy ?? ""]);
        return keys;
    }

    // A word the foot says for a key, in place of the keys, until the next
    // key: q's “q does nothing · :q closes”, or “not here” for
    // a key that means nothing in normal, so none is silent.
    property var saying: []
    property int sayingKeys: -1
    readonly property bool sayingNow: saying.length > 0 && prompt.keysTaken === sayingKeys
    // Words that aren't keys, as what an act didn't do and why, say nothing
    // in Return's place, so its last stands at its own width: held there,
    // the why stood a Return's room away from what it was the why of.
    property bool sayingLoose: false
    function say(keys, loose) {
        saying = keys;
        sayingLoose = loose === true;
        sayingKeys = prompt.keysTaken;
    }

    // What an act done once the launcher closed came to, when it wasn't
    // what its row said it would be, as a folder gone between the listing
    // and Return: said once the next time it opens, until the first key,
    // as `say` says a key's word, never as a notice (VKCF3Z, 2026-11-07
    // [4a00987e]). Each is [mark, word, colour], as the foot's keys are;
    // said again before it opens, the newer takes its place.
    property var toldOnOpen: []
    property bool toldLoose: false
    function tellOnOpen(told, loose) {
        if (open) {
            say(told, loose);
        } else {
            toldOnOpen = told;
            toldLoose = loose === true;
        }
    }

    // What q says, on the rows and in the drawer alike: with :q at the
    // right end, where Return stands, so it reads q then :q.
    function sayQuit() {
        const hints = Words.field.hints ?? {};
        say([[":q", hints.quits ?? ""], ["q", hints.quit ?? ""]]);
    }

    // What h or l says on a row, which has no line to move along: that it
    // means nothing here, and Vim's way in or out beside it, at the right
    // end where Return stands, so it reads "l not here \u00b7 \u21b5 open" and
    // "h not here \u00b7 - up" (the vi-paradigm spec, Normal). Each key is
    // the table's, so neither names one that does nothing; Backspace is h.
    function sayTheWay(act) {
        const hints = Words.field.hints ?? {};
        const way = act === "intoSaid" ? [hintKey("run"), hints.open ?? ""]
            : [hintKey("out"), hints.up ?? ""];
        say([way, [hintKey(act), hints.notHere ?? ""]]);
    }

    // A key as the foot says it: its mark, then its word where it has one.
    function saidKey(key) {
        return key[1] === "" ? key[0] : `${key[0]} ${key[1]}`;
    }

    // How far in the foot's keys may reach and how wide Return's place is,
    // so they hold still. Said for the selected row alone, the keys moved
    // as the arrows went from a row that opens to one with more behind it,
    // and squeezed how things stand beside them from words to glyphs and
    // back (Joel, 2026-10-25: "jumpy ... at home arrowing around"). So
    // Return, last at the right end, keeps a place as wide as the widest
    // it's been said since the launcher opened, and how things stand keeps
    // clear of where the widest set of keys has reached. The rest sit just
    // before Return, each as wide as it is: a place kept for every key any
    // row had left holes between them as soon as Joel typed ("the
    // whitespacing ... is all fucked up"), and took room from how things
    // stand that no row used. Both only grow while the
    // launcher's open, and only as the rows change, so nothing gives way
    // under the selection. Return's place opens as wide as the widest
    // Return word, so the first letter typed doesn't move it: grown only
    // as rows came, Return moved in as soon as Joel typed, since home's
    // rows say less than what a letter finds. The keys before it hold no
    // room until a row says them.
    // Chosen as each opening's room is set (footReset), since a binding
    // would weigh the verbs once, maybe before the foot's face had loaded
    // and measured them all alike (Y2YBTS, 2026-11-05). Weighed again only
    // once the words file or the face has changed (Pacing.verbRoom): every
    // opening walking 1,803 things' verbs was its slowest part after the
    // first, 8 to 11 ms of the 30 to 39 before it could draw (2026-11-10).
    property string footVerbRoom: "connect"
    // The words and the face the room was last weighed in, with how many
    // things that opening weighed, said with its time (openingsTimed).
    property var footVerbsWeighed: null
    property int openVerbsWeighed: 0

    // The widest verb the words file names, in any of its states, as the
    // foot draws it after Return (VKCF3Z, 2026-11-05: "Hold room at the
    // far end for the widest Return word the words file names"). A room
    // kept for "connect" alone let "\u21b5 brighten" move Return on the
    // first letter; "connect", which a network's source says, stays the
    // least. A source's own wider verb may still move it once.
    function widestVerb(things) {
        let widest = "connect";
        for (const entry of things) {
            const states = Object.values(entry.states ?? {});
            for (const verb of [entry.verb].concat(states.map(state => state.verb))) {
                if (typeof verb === "string"
                    && statusLine.keys.advance("\u21b5", verb)
                        > statusLine.keys.advance("\u21b5", widest))
                    widest = verb;
            }
        }
        return widest;
    }
    // Return and its widest word on the find, where the launcher opens,
    // and the completion key the first letter brings, since room is held
    // only for what a mode draws (VKCF3Z, 2026-11-05): held for
    // Shift+Return's "below" and the right arrow too, which no foot on the
    // find draws, it cut the date at 1.5 beside an empty stretch; held for
    // Return alone, "Ctrl+Y complete" on the first letter cut it then and
    // slid every fact after it. Tab in is drawn on the find beside a
    // row holding parts, so it's held too: left out, the first letter
    // picking a program slid the keys in by its width. A row's own keys
    // grow it as they come (noteKeys).
    readonly property var footKeysRoom: [["\u21b5", footVerbRoom],
        [hintKey("into", "find"), (Words.field.hints ?? {}).in ?? ""],
        [prompt.completeKey, (Words.field.hints ?? {}).complete ?? ""]]
    // What normal says beside Return at home: the commands and the keys,
    // kept while normal is taken rather than from the opening. Kept from
    // the start, it cut the battery's charge from the glance on the find,
    // which never gives way; - up is said only once a step is held, and
    // Tab in only on a row holding parts, whose rows grow the room as
    // any row does.
    readonly property var footNormalRoom: [["\u21b5", ""], [":", "commands"],
        ["Space ?", "keys"]]
    // Return's place, as wide as the widest Return said since the launcher
    // opened, its marks and its word whole: kept by its word alone, a
    // Return said with the right arrow beside it, as a collection's "see
    // all", was wider than the place and moved its marks along from a
    // category's (Joel, 2026-10-25: "jumpy ... at home arrowing around").
    property real footReturnWidest: 0
    property real footKeysWidest: 0
    // The step and what's typed that the foot's room was last weighed for.
    property string footWeighedFor: ""
    // Each row's keys as the foot weighed them for that and the mode, for
    // it and the last places shown, newest first (Pacing.weighingAmong),
    // with how many rows were weighed afresh in all, for the check that an
    // act changing one row weighs none but it.
    property var footRowsWeighed: []
    property int rowsWeighed: 0
    // How many times the foot weighed any row afresh, so an opening says
    // how often it weighed its rows before its frame (openDrawn).
    property int keysWeighings: 0
    // How many figures the chosen count holds room for: as many as the
    // place has rows, three while the drawer is out, and only ever more
    // within a place, so rows arriving with no key pressed, as a scan's,
    // or the drawer going back in never narrow the facts (Y2YBTS,
    // 2026-11-06: Joel's "nothing moves on its own").
    readonly property int footFiguresNow: drawerCard.out ? 3
        : String(Math.max(1, matches.length)).length
    property int footFiguresWidest: 1
    onFootFiguresNowChanged: footFiguresWidest = Math.max(footFiguresWidest, footFiguresNow)

    // Return's place starts from nothing and grows from the rows it may
    // name, as design.md's foot says: on the find those what's typed lists,
    // weighed afresh as each keystroke changes it (noteKeys); in normal and visual
    // the place's own rows, weighed again as the place changes, since room
    // for a verb no row there can say ranks nothing, and over two windows
    // in visual it cost the foot its y (VKCF3Z, 2026-11-06). Held as wide
    // as the widest verb words.json names, it left a gap before "\u21b5
    // start" on "chrome" (VKCF3Z, 2026-11-07). The keys' room keeps that
    // widest verb all the same (footKeysRoom), so the facts never slide
    // as Return grows; only the keys move, toward it.
    function footReset() {
        footReturnWidest = 0;
        footKeysWidest = keysWide(footKeysRoom);
        footFiguresWidest = footFiguresNow;
    }

    // How wide a row's keys are, Return in its place, or loose (say) at
    // its own width.
    function keysWide(told, loose) {
        if (told.length === 0)
            return 0;
        return told.slice(1).reduce((sum, key) => sum + statusLine.keys.spacing
            + statusLine.keys.advance(key[0], key[1]), loose === true
            ? statusLine.keys.advance(told[0][0], told[0][1]) : returnPlace(told[0]));
    }

    // How wide Return's place is beside what it says now.
    function returnPlace(key) {
        return Math.max(statusLine.keys.advance(key[0], key[1]), footReturnWidest);
    }

    function noteKeys() {
        // A keystroke that changes what's typed changes every row, and a row
        // no longer listed can't be named, so it holds no room: the room is
        // weighed afresh for the rows the text now lists, and grows only as
        // more of them come or the cursor moves among them. Held from a
        // letter passed, Sleep's ":suspend" on "s" left a gap before
        // "\u21b5 more" once "swayimg" was typed (VKCF3Z, 2026-11-07).
        const weighing = `${stepsTaken}|${typed}`;
        if (weighing !== footWeighedFor) {
            footWeighedFor = weighing;
            footReset();
        }
        // A row weighed before for these is the same keys at the same
        // widths, so only a row new to them is weighed. The selection says
        // more than its keys elsewhere, and visual's keys move with its
        // rows, so those are weighed each time (keysFor).
        // Held for the last few places shown, so one shown again, as a
        // chord's kept place is, weighs only its rows new since then.
        const held = Pacing.weighingAmong(footRowsWeighed, `${weighing}|${prompt.mode}`,
            { words: Words.all, face: statusLine.keys.measured }, typed === "");
        footRowsWeighed = held;
        const kept = held[0];
        const before = kept.weighed;
        const weigh = row => weighedKeys(keysFor(row));
        const afresh = row => {
            Pacing.keptRow(kept, row);
            return weigh(row);
        };
        // A run of rows alike in their keys, as Emoji's, is weighed once.
        const alike = Pacing.toWeigh(matches, selected);
        const listed = asking ? alike.concat([asking]) : alike;
        const said = listed.map(row => row === selected || visual ? afresh(row)
            : Pacing.weighedRow(kept, row, weigh)).filter(one => one.keys.length > 0);
        rowsWeighed += kept.weighed - before;
        if (kept.weighed > before)
            keysWeighings += 1;
        // Room for the drawer's own only once the keys have gone in, so
        // the rows' foot never gives up room to keys they don't show.
        // Choosing, the drawer names no key, so it takes no room.
        const drawn = QueryKeys.paged(prompt.mode) ? drawerKeys(prompt.mode) : [];
        if (drawn.length > 0)
            said.push(weighedKeys(drawn));
        if (prompt.mode === "normal")
            said.push(weighedKeys(footNormalRoom));
        const place = Math.max(footReturnWidest, ...said.map(one => one.returnWide));
        // Said as it grows, so a test sees which row's Return took the
        // room, though the foot shows only the selection's.
        if (place > footReturnWidest) {
            const widest = said.find(one => one.returnWide === place);
            Report.say(`foot room for ${widest.keys[0].join(" ")}`);
            footReturnWidest = place;
        }
        const wide = Math.max(footKeysWidest,
            ...said.map(one => one.restWide + Math.max(one.returnWide, footReturnWidest)));
        if (wide > footKeysWidest)
            footKeysWidest = wide;
    }

    // Keys with how wide Return is, its marks and its word, and how wide
    // the rest are after it, as keysWide adds them.
    function weighedKeys(keys) {
        if (keys.length === 0)
            return { keys, returnWide: 0, restWide: 0 };
        return { keys, returnWide: statusLine.keys.advance(keys[0][0], keys[0][1]),
            restWide: keys.slice(1).reduce((sum, key) => sum + statusLine.keys.spacing
                + statusLine.keys.advance(key[0], key[1]), 0) };
    }

    // The widest that graphed rows have said at their right in this step,
    // in characters, so the graphs' column only ever widens until a key
    // changes the step: worked out afresh from the rows shown, it moved
    // every graph and cut every name somewhere else each time a figure
    // grew or shrank by a digit.
    property int figuresWidest: 0
    property string figuresStep: ""
    // The same for what every row says at its right, so where a name is
    // cut holds still as "working" becomes "needs you" and back: worked out
    // afresh, each title's cut crept left and right as its side changed
    // (the launcher audit's finding 31, Joel's yes on 2026-10-26).
    property int sidesWidest: 0

    function noteFigures() {
        const step = `${held.map(each => steadyId(each.row)).join("/")}|${typed}`;
        // A side that counts on by itself holds room for the widest it
        // reads until it grows a figure (its `sideWidest`), since it's
        // measured only when its row is made.
        const said = row => ((held.length === 0 ? kindOf(row) : "")
            || ((row.chord ?? []).length > 0 ? ""
                : typeof row.counts === "function" ? row.sideWidest ?? row.side ?? ""
                : row.side ?? "")).length;
        const now = Math.max(0, ...matches.filter(row => !!row.graph).map(said));
        const sides = Math.max(0, ...matches.map(said));
        if (step !== figuresStep) {
            figuresStep = step;
            figuresWidest = now;
            sidesWidest = sides;
            return;
        }
        if (now > figuresWidest)
            figuresWidest = now;
        if (sides > sidesWidest)
            sidesWidest = sides;
    }

    // The shortest way the foot's facts have had to read since the launcher
    // opened (Standing.js's fittedFrom), which they never read longer than
    // until it closes.
    property int footStage: 0
    // The room each of the foot's facts has needed since the launcher
    // opened, by its label and the way it reads, which it keeps: the
    // battery going from 100% to 99%, or the agents from "10 working" to
    // "9 working", slid every fact after it (the launcher audit's finding
    // 32, Joel's yes on 2026-10-26). Kept for each way, so a fact shrunk
    // to its glyph doesn't keep the room its words took.
    property var factsWidest: ({})

    // A step of its own has notes of its own, so the field's right end
    // takes the room they need afresh (Prompt.qml's steady). Known by the
    // path, since a refresh makes the same steps again.
    readonly property string stepsTaken: held.map(step => steadyId(step.row)).join("/")
    onStepsTakenChanged: prompt.forget()

    // How many times the rows have been made, and how many rows drawn
    // afresh, for the check that a refresh redraws only what changed.
    property int rowsMade: 0
    property int rowsDrawn: 0
    // And how many of those were built from nothing rather than taken
    // from the rows let go, for the check that a step takes its rows from
    // those it let go.
    property int rowsBuilt: 0

    // Where the rows drawn stand, the glyph and hue an act of its own
    // without them wears and whether they're home's, told as their rows
    // come rather than as held moves (RowsPlace.js), once the list keeps
    // them (list.keep), so the rows of a place left never redraw for the
    // place they give way to, and a refresh in the place they stay in
    // redraws none. A move of held that brings no
    // new rows tells them once it's done.
    property string rowsGlyph: ""
    property string rowsHue: ""
    property bool rowsAtHome: true
    function rowsPlaced() {
        // Told while the rows drawn are still the last ones, it redraws
        // rows about to go, as home's did for each kept opening, about
        // 3 ms (ZINAWZ, 2026-11-11); counted for the check that it isn't.
        if (list.rows !== matches)
            rowsToldEarly += 1;
        RowsPlace.tell(root, held);
    }
    property int rowsToldEarly: 0

    onMatchesChanged: {
        // The selection's start keeps to the row it began on, as the
        // cursor does (compose's Rank.pinned), so rows arriving above it
        // never move what d would take from what's tinted.
        if (visualFrom >= 0)
            visualFrom = Operators.heldFrom(matches, visualId, visualFrom, Rank.identity);
        rowsMade += 1;
        noteKeys();
        noteFigures();
        noteDigits();
    }
    onAskingChanged: noteKeys()
    Connections {
        target: prompt
        function onModeChanged() {
            root.readSelectedActions();
            // The foot's room starts again for each mode, which changes
            // what it draws, so the glance never gives way to keys a mode
            // no longer shows, as normal's once back on the find (QIFT5D);
            // and its facts' forms with it, which within one mode only
            // shorten, or a form normal forced would outlive it (Y2YBTS).
            root.footReset();
            root.footStage = 0;
            root.factsWidest = {};
            root.noteKeys();
        }
    }

    // A verb as the foot says it, one word (Joel, 2026-10-24: "Switch to
    // it" was too many): a turn by the way it goes ("off"), a verb that's
    // one with the way it goes whole ("log out", "put back", whose first
    // word alone says nothing, Go To's "go to", so the foot names the act
    // by its name, as VKCF3Z ruled on 2026-11-07, and "start afresh", which
    // says it won't reopen what was), a verb with how many it takes whole
    // (the file picker's "open 2", which said "open" while it would answer
    // two gathered files), anything else by its first word.
    readonly property var phrasal: ["in", "out", "back", "to", "afresh", "again"]

    function oneWord(verb) {
        const said = verb.split(" ");
        if (said[0] === "turn" && said.length > 1)
            return said[said.length - 1];
        return said.length === 2 && (phrasal.includes(said[1]) || /^\d+$/.test(said[1]))
            ? verb : said[0];
    }

    // A chord as words, for the foot: its keys joined by +, those pressed
    // one after another a space apart, as Chord draws them.
    function chordText(keys) {
        return (keys ?? []).map((key, index) => index === 0 ? key.text
            : `${key.after === true ? " " : "+"}${key.text}`).join("");
    }

    // The word that found a row by what else it's called ("sleep" for
    // Suspend the machine), shown after its name, so a row is never named
    // by a word it doesn't say and Joel can see why it's there: the run of
    // its other words that each word typed starts, or the first one.
    function saidAs(row) {
        const typedWords = Fuzzy.words(typed);
        // Its keywords hold its name too, and a hyphened word of it closed
        // up, so a search can say some words of each; only the others are
        // shown.
        const named = new Set(Fuzzy.words(row ? row.name : "")
            .reduce((all, word) => all.concat([word, word.replace(/-/g, "")]), []));
        const own = row && row.keywords
            ? Fuzzy.words(row.keywords.text ?? "").filter(word => !named.has(word)) : [];
        const inName = word => Array.from(named).some(one => one.startsWith(word));
        const typedOwn = typedWords.filter(word => !inName(word));
        if (typedOwn.length === 0 || own.length === 0)
            return "";
        for (let at = 0; at + typedOwn.length <= own.length; at++) {
            if (typedOwn.every((word, i) => own[at + i].startsWith(word)))
                return own.slice(at, at + typedOwn.length).join(" ");
        }
        return own.find(word => word.startsWith(typedOwn[0])) ?? "";
    }

    // The fewest letters that reach a row from what's typed: the shortest
    // start of it that's learned to mean this row. Said in the foot when
    // it's shorter than what was typed, so the habit gets quicker by itself.
    function enough(row) {
        const letters = Fuzzy.plainly(typed);
        if (!row || letters.includes(" "))
            return "";
        // What's found at a shorter start isn't known, so any row picked
        // there counts against this one, and it's said only when sure.
        const there = Rank.maybeThere(foundHere);
        for (let length = 1; length < letters.length; length++) {
            const start = letters.slice(0, length);
            if (picks.latched(start, there, namedAs[start] ?? []) === row.key)
                return start;
        }
        return "";
    }

    // What's latched for what's typed, worked out once a search, and the
    // rows it found, by key, each with the fewest letters it's learned at
    // (Rank.js's leastOf), so a row that's gone counts for nothing.
    property string latch: ""
    property var foundHere: ({})

    // The keys of the rows named each name, so a name typed whole is first
    // over another row latched there (Rank.js's latched).
    readonly property var namedAs: {
        const out = {};
        for (const choice of choices) {
            const name = Rank.plainly(choice.name);
            (out[name] ?? (out[name] = [])).push(choice.key);
        }
        return out;
    }

    // Every row there is now, by key, each with the fewest letters it's
    // learned at, for forgetting the picks of those that are gone: what the
    // main search finds from, and what each collection holds. Made only
    // when something's picked, from every collection, wanted or not, so a
    // collection that can name its rows' keys without making its rows says
    // them in keysKnown(), as People does.
    function knowing() {
        const out = {};
        const add = row => {
            if (row && row.key)
                out[row.key] = Rank.leastOf(row);
        };
        choices.forEach(add);
        for (const source of collections)
            (source.keysKnown?.() ?? source.everything()).forEach(add);
        return out;
    }

    // The names a row had before the glossary renamed it (words.json's
    // `once`, or the row's own for one whose key isn't a word's, as a
    // connection's): they still find it, and are never shown, so a rename
    // never loses a row (Joel, 2026-10-26: "i didn't know its name had
    // changed, i was used to calling it caffeinate").
    function oldNames(row) {
        const id = String(row.key ?? "").replace(/^(?:category|collection|answer|place):/, "");
        return row.once ?? row.category?.once ?? Words.of(id).once ?? [];
    }

    // "Once called Caffeinate." below a row found by a name it had and not
    // by its own, so the new name is learned as it's used.
    function onceCalled(row) {
        const typed = Fuzzy.plainly(root.typed);
        if (typed === "" || startsAWord(row.name ?? "", typed))
            return "";
        const old = oldNames(row).find(name => {
            const plain = Fuzzy.plainly(name);
            return plain.startsWith(typed) || typed.startsWith(plain);
        });
        return old ? `Once called ${old}.` : "";
    }

    // A row's own words, with the names it had and those a machine adds
    // for it (jitsusama.quickshell.launcher.synonyms), so a habit of Joel's
    // finds it. Made once for each row, and never twice for the same one.
    function withSynonyms(row) {
        const more = (Providers.launcher.synonyms[row.key] ?? []).concat(oldNames(row));
        if (more.length === 0 || row.synonymised)
            return row;
        row.keywords = Fuzzy.prepare([row.keywords ? row.keywords.text : ""].concat(more)
            .join(" ").trim());
        row.synonymised = true;
        return row;
    }

    // How often each thing's been picked, taken as the launcher opens, so
    // what it orders holds still while it's open.
    property var often: ({})

    // With nothing typed, home is the same rows every time, each in its
    // one place, as the approved proposal sets them
    // (attachments/launcher-proposal.md under the launcher's quest): Now
    // first, saying what's on or waiting this moment, then each place, a
    // context of its own, saying what it holds, so one glance reminds Joel
    // what there is. Each thing has one home among them, and nothing else
    // is here. What each row says is read as the launcher opens and held
    // while it's open, so nothing moves under him.
    // How many characters a row's name and what it holds have room for.
    function rowRoom() {
        return Math.floor((list.width - 2 * card.inset - glyphColumn
            - Theme.space.rung.groups - Theme.space.rung.apart)
            / Math.max(1, rowFace.advanceWidth("0")));
    }

    function home() {
        // Now is home's first row, not a group of its own, so the rows sit
        // a row apart from the head's hairline to the foot's (VKCF3Z,
        // 2026-11-05).
        return Words.home.map(id => homeRow(id)).filter(row => row !== null);
    }

    // One of home's rows: the row its place is everywhere else, made by
    // the one function that makes it there (VKCF3Z, 2a429f67, on Joel's
    // "i don't like how the title and values on the home screen differ
    // from every other sub-menu"). A place that's one answer, as Disk is,
    // is there by name with nothing after it until the machine can say,
    // as before its first reading is back, so home never gains a row
    // while it's open, and Return leaves it open rather than closing on
    // nothing.
    function homeRow(id) {
        const row = resolved(id);
        if (row)
            return row;
        const entry = Words.of(id);
        if (!entry.name)
            return null;
        return {
            key: `place:${id}`,
            name: entry.name,
            glyph: Words.glyph(id),
            hue: "voice",
            plain: true,
            chord: [],
            side: "",
            count: "",
            verb: "open",
            stays: true,
            run: () => {}
        };
    }

    // A collection as one row, at home and in a place such as Desktop:
    // Return or the right arrow opens all of it. Read whole, it says how it
    // stands at its right, or that it holds none, or how many it holds, and
    // names what it holds in its drawer (PlaceRow.js); a row the search
    // finds isn't read, since naming it would read all of it at every
    // change (`named`).
    function whole(source, gap, read = true) {
        const collection = source.collection;
        const name = source.settings.name;
        const opener = Shortcuts.actions.find(action => action.opens === name);
        const title = collection.title ?? collection.noun;
        // What it holds is read once, for its names and for its count. A
        // place that says how it stands itself (saysAtHome) isn't read for
        // its rows at all, so they wait to be wanted (Wanted.js), and it
        // names what it holds without making them, as Agents does.
        const rowsRead = read && !source.saysAtHome;
        const held = rowsRead ? source.everything() : [];
        const members = !read ? [] : rowsRead ? source.memberNames(held)
            : source.memberNames();
        const none = members.length === 0 ? collection.none ?? "" : "";
        const says = read ? sidesOf(source) : { side: "", sideItems: [] };
        const side = read ? PlaceRow.side(says.side,
            rowsRead ? source.counted(held) : "", none) : "";
        const row = {
            key: `collection:${name}`,
            verb: "open",
            name: title,
            plain: true,
            note: source.noted(),
            glyph: collection.glyph,
            hue: "voice",
            // No row draws the chord that opens it straight: the foot
            // teaches it as the next time's way while it's the cursor row,
            // and the row's far end is its say's. Drawn on Now, Super+O
            // stood alone at the row's end and read as a stray "+ 0"
            // (VKCF3Z, 2026-11-06), and Desktop's collections now read as
            // home's do (2a429f67).
            chord: [],
            nextTime: opener ? opener.chord : [],
            // How it stands, or how many it holds, at its right, where a
            // row's figure stands (VKCF3Z, 2026-11-05); holding none, it
            // says so there as a run, never the sentence an empty place
            // says inside (VKCF3Z, 2026-11-08, dae7fb67).
            side: side,
            // The same as items, where it gives way by them, as Now's first
            // thing waiting does (Now.qml's homeItems).
            sideItems: says.sideItems,
            // Nothing after its name, as no other row says there; what it
            // holds is named in its drawer (VKCF3Z, dae7fb67).
            count: "",
            drawer: PlaceRow.drawer(title, members, placeSaid, collection.holding ?? ""),
            gap: gap,
            opens: name,
            from: source,
            chip: collection.noun,
            title: collection.title,
            ask: collection.ask,
            lead: collection.glyph,
            leadHue: "voice",
            verbs: () => source.everything()
        };
        // A collection with a plate of its own, as Now's moment is
        // (Now.qml), shows it beside its row: by `placePlate` alone, since
        // Home Network's and WARP's `plate` make one of their rows' plates
        // and, asked with nothing, threw on every opening that listed them
        // (the net, 2026-11-09).
        return typeof source.placePlate !== "function" ? row
            : Object.assign(row, { plated: true, drawer: () => source.placePlate() });
    }

    readonly property bool atHome: typed === "" && held.length === 0

    // Whether a source's rows are wanted now (launcher/Wanted.js), which
    // each source's own wanted asks.
    function wants(source) {
        return Wanted.wanted({ cataloguing: cataloguing, open: open, held: held,
            becoming: Object.values(becoming), typed: typed, entering: entering }, source);
    }
    // The place into() opens, from before the launcher opens until its
    // step is held, so no source home names is wanted in between.
    property var entering: null

    // A collection's matches with those picked before for what's typed
    // first, most picked first, the rest in their order.
    function learnedFirst(found, letters) {
        const learned = picks.weights(letters);
        const before = found.filter(row => learned[row.key]);
        if (before.length === 0)
            return found;
        return Order.sorted(before, (a, b) => learned[b.key].weight - learned[a.key].weight)
            .concat(found.filter(row => !learned[row.key]));
    }

    // Shift+Return, as a chord beside a verb that does what it does.
    readonly property var belowChord: [
        { text: "\u21e7", wide: false },
        { text: "\u21b5", wide: false }
    ]

    // What's been stepped into, each with the rows it offers and where the
    // step before it was left: what was typed, the row selected and how far
    // the list was scrolled, so stepping back out finds it as it was. A
    // thing and its verbs, then a verb and its targets.
    property var held: []

    // The place used last, as Vim's alternate file: what was held there and
    // where it was left, which Tab goes back to, the place it leaves taking
    // its turn, so Tab again comes back. A place is what's held, so a step
    // in, out or beside makes the one left the last; what a held step
    // says of itself changing, as its rows are found, never does. It's
    // let go as the launcher opens, since the places before were left.
    property var lastPlace: null
    property var placeHeld: []
    function placeOf(steps) {
        return steps.map(step => `${step.row?.key ?? ""}\u0000${step.row?.name ?? ""}`)
            .join("\u0001");
    }
    // Called as held changes, before the field is set for the new place,
    // so where the launcher is still says where the place left was left.
    onHeldChanged: {
        Qt.callLater(rowsPlaced);
        const left = placeHeld;
        placeHeld = held;
        // Every place's gutter starts as wide as home's, its numbers as
        // few as its own rows need.
        gutterDigits = leastDigits;
        numberDigits = 1;
        if (open && placeOf(left) !== placeOf(held)) {
            // A place opening redraws every row, so its foot's room is
            // weighed afresh then, as its rows come.
            footReset();
            lastPlace = { held: left, here: here() };
            // A place stepped to afresh goes on the way back, and leaves
            // nothing ahead, as Vim's jump list does.
            if (!jumping) {
                jumpsBack = jumpsBack.concat([{ held: left, here: here() }]);
                jumpsAhead = [];
            }
        }
    }

    // The places stepped into, as Vim's jump list: those left, which Ctrl+O
    // goes back through, and those gone back from, which Ctrl+I and Tab go
    // forward through again. Let go as the launcher opens, with the place
    // used last.
    property var jumpsBack: []
    property var jumpsAhead: []
    property bool jumping: false
    function jump(by) {
        const from = by < 0 ? jumpsBack : jumpsAhead;
        if (from.length === 0)
            return;
        const to = from[from.length - 1];
        const left = { held: held, here: here() };
        if (by < 0) {
            jumpsBack = jumpsBack.slice(0, -1);
            jumpsAhead = jumpsAhead.concat([left]);
        } else {
            jumpsAhead = jumpsAhead.slice(0, -1);
            jumpsBack = jumpsBack.concat([left]);
        }
        traces.began("jump", { from: held.length, to: to.held.length });
        jumping = true;
        held = to.held;
        jumping = false;
        restart(to.here.typed);
        reselect(to.here);
    }
    // Goes somewhere by several steps as one jump, as Vim's :cd or :bnext
    // is one: the place left goes on the jump list and is the place used
    // last, never a step on the way, so Ctrl+O goes back to where Joel
    // was; one that ends where it began, as a place made again in place
    // does, is no jump at all. A walk that opens a collection afresh
    // (into) lets go what was on the way back, which is kept here.
    function oneJump(walk) {
        const left = { held: held, here: here() };
        const was = { back: jumpsBack, ahead: jumpsAhead, tags: tagsBack, last: lastPlace };
        jumping = true;
        try {
            walk();
        } finally {
            jumping = false;
        }
        const moved = placeOf(left.held) !== placeOf(held);
        jumpsBack = moved ? was.back.concat([left]) : was.back;
        jumpsAhead = moved ? [] : was.ahead;
        tagsBack = was.tags;
        lastPlace = moved ? left : was.last;
    }
    // Whether Ctrl+] goes into a row's own parts. A notice's acts are
    // commands under its card's words, not parts (VKCF3Z, 2026-11-05), so
    // Ctrl+] leaves it to where it lives instead.
    function holdsParts(row) {
        return holdsMore(row) && kindOf(row) !== "notice";
    }
    // Whether Tab goes into a row: one holding parts, as a program's page,
    // or a found act, into its program, as the foot's "Tab in" says.
    function goesIn(row) {
        return !!row && !row.gone && !row.still && (holdsParts(row) || !!row.landsIn);
    }
    // Tab: into the selected row where it goes in, as Ctrl+] would, saying
    // whether it did. Where it doesn't, normal says so, as l does, and the
    // find leaves it to be its next row. On the find it goes in only where
    // the foot says "Tab in", so never on a row Return already steps into,
    // as an answer holding parts, where it stays the next row.
    function tabbedIn(mode) {
        const row = matches[current] ?? null;
        const stepping = !!row && (stepsIn(row) || stepsThing(row));
        if (goesIn(row) && !(mode === "find" && stepping)) {
            follow(row);
            return true;
        }
        if (mode === "find")
            return false;
        say([[hintKey("into"), (Words.field.hints ?? {}).notHere ?? ""]]);
        return true;
    }
    // Where Ctrl+] takes a row, as the old step in by l did, so no walk is
    // lost (Joel's rule): a found act into its program, a row into its
    // parts, a found place to where it sits, and a spot or a collection
    // that opens to where it leads. A command, which only fills the line,
    // has no place, nor a notice, whose acts are commands.
    function followed(row) {
        if (steppedAsLed(row))
            return true;
        if (row.lands) {
            row.run();
            return true;
        }
        if (row.landsIn) {
            landInside(row, row.landsIn);
            return true;
        }
        if (kindOf(row) === "notice")
            return false;
        if (row.inside && held.length === 0) {
            goTo(row.inside);
            return true;
        }
        if (leadsElsewhere(row) && row.start === undefined) {
            run(row, false);
            return true;
        }
        return enter(row);
    }
    function toLastPlace() {
        if (lastPlace === null)
            return;
        const to = lastPlace;
        traces.began("lastPlace", { from: held.length, to: to.held.length });
        held = to.held;
        restart(to.here.typed);
        reselect(to.here);
    }

    // The places Ctrl+] left, as Vim's tag stack, which Ctrl+T comes back
    // through. Let go as the launcher opens, with the jumps.
    property var tagsBack: []

    function forgetFollows() {
        tagsBack = [];
    }

    // The rows each source has made looking for where a row lives, by its
    // name, for the check that tells them from what a key made: only a
    // Ctrl+] press looks (livesOf), so with none pressed it's none. A
    // plain object, so adding to it notifies nothing.
    readonly property var livesMade: ({})
    // How many rows one look may list before it gives up, saying the row
    // isn't found here: the look is the key's own work, so it's bounded
    // rather than left to hold the key (Y2YBTS, 2026-11-09). Joel's home
    // places listed 344 rows between them, People's 117 the most (the
    // `names` call, 2026-11-09), and a count rather than a time keeps the
    // VM's slow drawing from cutting it short.
    readonly property int livesMost: 3000

    // Where the row `id` lives, looked for on the press that asks
    // (Catalogue.js's livesOf) rather than kept for every row: kept, it was
    // walked again on most openings and after each provider's change, and
    // made 18,693 of People's 19,461 rows and 5,876 of Agents' 9,865 at
    // home in Joel's morning, so keys landed behind it (2026-11-09). The
    // look goes through places first, what home's categories and
    // collections list, where a notice's program or Do Not Disturb lives,
    // and only then on into the things it set aside, each row listed and
    // counted against the budget once. It never goes into a place
    // that says itself at home, as Agents does, whose rows wait to be
    // wanted (Wanted.js), nor a person's insides, which ask every source
    // (People.qml's `parted`). It says `path`, by steady ids from home or
    // null, the rows it listed and made and its ms, and whether it gave up.
    function livesOf(id) {
        const looking = Date.now();
        const before = tallies();
        const said = Catalogue.livesOf(home(), id, {
            idOf: one => steadyId(one),
            holds: (one, above) => catalogueHolds(one,
                above === null || collects({ row: above })),
            enters: one => one.from?.saysAtHome !== true,
            first: one => Catalogue.isPlace(one),
            spent: listed => listed > livesMost
        });
        const after = tallies();
        let made = 0;
        for (const name of Object.keys(after)) {
            const more = after[name] - (before[name] ?? 0);
            livesMade[name] = (livesMade[name] ?? 0) + more;
            made += more;
        }
        return { path: said.path, listed: said.listed, made: made, took: Date.now() - looking,
            spent: said.spent };
    }

    // A row whose side runs on, as a running stopwatch's count, says it as
    // `counts(now)`, which its row draws afresh from this clock each tenth
    // of a second while one is listed: only that side changes, so its
    // provider makes no rows again and the list isn't put together anew
    // (Y2YBTS, 2026-11-08). Its `side` stays what it said when made, for
    // what the tests read, and its `sideWidest` gives the column its room.
    property real tenth: Date.now()
    readonly property bool counting: open
        && matches.some(row => typeof row?.counts === "function")

    Timer {
        interval: 100
        repeat: true
        running: root.counting
        onTriggered: root.tenth = Date.now()
    }

    // Ctrl+] goes to the row's own place, as Vim follows a tag (VKCF3Z,
    // 2026-11-05): first wherever followed() takes it, as the old step in
    // by l did, so a row that holds more goes into its parts, as the
    // foot's Tab in says. A row with no parts living in another place,
    // as a Now row, one found from home or a notice, whose acts are
    // commands, not parts (VKCF3Z, 13:07), goes to that place, onto it
    // there. A row Return takes elsewhere, a spot or a program's act found
    // from home, keeps where it lives; a notice lives as its sender, its
    // program's row; any other is looked for from home, on the key
    // (livesOf), saying it isn't here if the look gives up. The place left
    // goes on the tags' way back, as Vim's tag stack keeps it, and goTo
    // puts it once on the jump list, as a tag is one jump in Vim, not one
    // a step (oneJump). A row with none says so.
    function follow(row) {
        const live = row && !row.gone ? row : null;
        const came = { held: held, here: here() };
        if (live && followed(live)) {
            traces.began("follow", { from: came.held.length, into: true });
            tagsBack = tagsBack.concat([came]);
            return;
        }
        // The look goes in the trace, found or not, since it runs on the
        // key: its ms, the rows it listed and made, and whether it gave up.
        const to = live ? followedTo(live) : { spot: null, look: null };
        const spot = to.spot;
        if (live)
            traces.began("follow", Object.assign({ from: held.length,
                to: spot ? spot.trail.length : null }, to.look === null ? {} : {
                took: to.look.took, listed: to.look.listed, made: to.look.made,
                spent: to.look.spent }));
        if (spot === null) {
            say([[hintKey("follow"), (Words.field.hints ?? {}).notHere ?? ""]]);
            return;
        }
        goTo(spot);
        tagsBack = tagsBack.concat([came]);
    }

    // Where Ctrl+] goes from a row (`spot`), by steady ids, or null where it
    // lives nowhere but here or the look gave up, and the look made for it
    // (`look`, as livesOf says), or null where the row says where it lives.
    function followedTo(row) {
        const own = row.trail !== undefined ? { trail: row.trail, spot: row.spot } : row.inside;
        const look = own ? null : livesOf(row.follows || steadyId(row));
        const path = own ? own.trail.concat([own.spot]) : look.path;
        return { spot: Catalogue.followed(path, held.map(step => steadyId(step.row))), look };
    }

    // Ctrl+T: back to the place Ctrl+] left last, as it was left, as Vim's
    // pops its tag stack; with none, Vim's E73, the rows staying put.
    function tagBack() {
        if (tagsBack.length === 0) {
            // Where the command line is, as E492 and E21 are.
            commandSay((Words.field.hints ?? {}).E73 ?? "", true);
            return;
        }
        const to = tagsBack[tagsBack.length - 1];
        tagsBack = tagsBack.slice(0, -1);
        traces.began("tagBack", { from: held.length, to: to.held.length });
        held = to.held;
        restart(to.here.typed);
        reselect(to.here);
    }

    // Every key here, as normal's Space ? lists them in the drawer, in
    // place of what the selected row has, until Space ? again or Escape.
    // Pressed in the drawer, it lists the drawer's own keys there, the
    // keys staying in it, until Space ? again or the way back to the rows
    // (VKCF3Z), which puts that list away as :pclose puts a window away.
    property bool listing: false
    property bool listedInDrawer: false
    // Whether the gutter counts from the cursor row or numbers each row
    // as it is, which Space u l switches, as his u l does.
    property bool numbersRelative: true
    // Whether the gutter numbers the rows: wherever a list shows, in every
    // mode (VKCF3Z, 2026-11-05), counting from the cursor in normal and
    // visual and as each row is on the find and the command line, as his
    // numbertoggle does (Joel, 2026-11-05: "it needs to show in find as
    // well as normal mode, showing real numbers in find mode").
    // On the : line, its menu's lines are the rows numbered.
    readonly property bool numbered: prompt.modal && over === null
        && (commanding !== null ? commandRows.count > 0 : matches.length > 0)
    readonly property bool numbersCounted: numbersRelative && commanding === null
        && (prompt.mode === "normal" || prompt.mode === "visual")
    // How many digits the gutter's room holds: three in every place, and
    // more only once a list numbers past 999, so the rows' glyphs and names
    // stand in one place on every page and in every mode. Home's find
    // reaches hundreds; a place that held only its own rows' room stood
    // its glyphs and names a digit left of home's, and stepping in moved
    // every column (Joel, 2026-11-06: "padding is different when on the
    // home launcher screen and a secondary screen").
    readonly property int leastDigits: 3
    property int gutterDigits: leastDigits
    // How many digits its numbers take, as Vim's number column takes as
    // many as its buffer's lines need: the rows a place lists, and while a
    // find is typed, the most it has listed, so its narrowing never
    // shrinks them under the cursor. They start on the inset whatever
    // their count, and the room they don't take lies after them, before
    // the glyphs (VKCF3Z, 2026-11-06, for Joel's "why leave constant room
    // for three when we often have only 1?").
    property int numberDigits: 1

    function noteDigits() {
        const digits = String(Math.max(1, matches.length)).length;
        if (digits > gutterDigits)
            gutterDigits = digits;
        numberDigits = typed === "" ? digits : Math.max(numberDigits, digits);
    }
    onNumberedChanged: placeNumbers()
    // The : line's lines numbered while it's open, and the rows again after.
    onCommandingChanged: Qt.callLater(placeNumbers)
    onRowsDrawnChanged: Qt.callLater(placeNumbers)

    // Tells the gutter where each row in view has its name's baseline, from
    // the rows the list has drawn, so its numbers sit on the rows' lines.
    function placeNumbers() {
        if (!numbered) {
            if (rowsGutter.rows.length > 0)
                rowsGutter.rows = [];
            return;
        }
        const shown = commanding !== null ? commandRows : list;
        const from = shown.contentY;
        const placed = [];
        for (let at = Math.max(0, shown.indexAt(shown.width / 2, from));
            at < shown.count && placed.length < rowsGutter.slots; at += 1) {
            const entry = shown.itemAtIndex(at);
            if (!entry || entry.y - from >= shown.height)
                break;
            placed.push({ at, base: shown.y + entry.y - from + entry.nameBaseline });
        }
        rowsGutter.rows = placed;
    }
    readonly property var listedPlate: {
        const words = Words.field.listed ?? {};
        const only = listedInDrawer ? ["page", "pageVisual"] : null;
        return { eyebrow: words.eyebrow ?? "",
            title: (listedInDrawer ? words.drawerTitle : words.title) ?? "", keys: true,
            facts: ListedKeys.keyList(QueryKeys.table, words.lines ?? {}, words.modes ?? {},
                only) };
    }
    onListingChanged: drawerSettled()
    // The whole list, listed from the rows before going in, gives way to
    // the drawer's rather than going.
    function listInDrawer() {
        const shown = listing && listedInDrawer;
        listedInDrawer = !shown;
        listing = !shown;
        drawerCard.anew();
    }
    Connections {
        target: prompt
        function onModeChanged() {
            if (!QueryKeys.paged(prompt.mode) && root.listedInDrawer) {
                root.listing = false;
                root.listedInDrawer = false;
            }
        }
    }

    // The command line, the : line (CommandLine.qml), while it's open:
    // { from, range, things, go, menu, options }, the mode : was pressed
    // in, which Escape gives back, the range typed before it, the rows its
    // acts act on, bound as : was pressed so a row arriving can't shift
    // them, what :{n} does, its menu (Commands.js) and :set's options as
    // they stood (standingOptions). Null while it's shut, so nothing of it
    // is made until : is typed.
    property var commanding: null
    // The lines Return ran, as Vim's history keeps them for :his and @:,
    // the newest last (Commands.remembered).
    property var commandEarlier: []
    // What Return on the line said, standing in the line's place for the
    // key that ran it alone: what went wrong, as Vim's command line says
    // E492, or what it asked, as :set volume? says volume=40.
    property string commandSaid: ""
    property bool commandFailed: false
    property int commandSaidFor: -1
    readonly property bool commandSays: commandSaid !== ""
        && commandSaidFor === prompt.keysTaken

    // What the cursor row's acts are, as a step into it lists them,
    // Return's first (stepRows), or the row alone where it's an act
    // itself, as one inside a thing is. Only acts are listed, never the
    // places and parts a step in also holds.
    function actsOf(row) {
        if (!row || row.gone)
            return [];
        const things = asThing(row, held.length === 0 || collecting);
        if (things && row.verbs) {
            return (stepRows(row, true) ?? [])
                .filter(one => !one.gone && Catalogue.kindOf(one) === "action");
        }
        return Catalogue.kindOf(row) === "action" ? [row] : [];
    }

    // Opens the command line over the rows, as : in normal does, or V4's
    // visual with its selection: { range, things, surface, go }, things
    // the rows the acts act on, the cursor row where none are given;
    // surface a surface's own commands (Commands.listed), as a viewer's
    // :rotate; go what :{n} and :$ do, where it isn't the rows'; and goes
    // what its line says that is, as "That picture, by its number".
    function commandLine(opening) {
        const row = selected;
        const things = opening?.things ?? (row ? [row] : []);
        const one = things.length === 1 ? things[0] : null;
        // A selection's acts are each of its things', listed once.
        const acts = one ? actsOf(one) : [].concat(...things.map(thing => actsOf(thing)));
        // An act on the cursor row acts on the thing it's inside.
        const on = one && acts.length === 1 && acts[0] === one && held.length > 0
            ? [held[held.length - 1].row] : things;
        const words = Words.field.command ?? {};
        commanding = {
            from: prompt.mode,
            range: opening?.range ?? "",
            // What it opens with written in, as Return on a row that ends
            // something writes its command.
            typed: opening?.typed ?? "",
            things: on,
            // The rows as they stood, which a range binds to things.
            rows: matches.slice(),
            cursor: current,
            marks: things.length > 1 ? { "<": matches.indexOf(things[0]),
                ">": matches.indexOf(things[things.length - 1]) } : {},
            go: opening?.go ?? null,
            // Given no surface, the place stood in lists its own, as the
            // file picker's folders give :sort and :cd (V3's interface).
            menu: Commands.listed({ acts, surface: opening?.surface
                ?? held[held.length - 1]?.row.commands?.() ?? [],
                table: Options.commands, row: current + 1,
                said: opening?.goes ? Object.assign({}, words, { row: opening.goes }) : words,
                called: commandWords, thing: one,
                earlier: opening?.earlier ? commandEarlier : [] }),
            options: standingOptions(setOptions),
            local: standingOptions(localOptions(on)[0])
        };
        // Its lines are numbered as the rows are, so the gutter holds
        // their digits too.
        gutterDigits = Math.max(gutterDigits, String(commanding.menu.length).length);
    }

    // The command line's opening while the launcher is open over a viewer,
    // so : there opens it again after something went wrong. No rows are
    // drawn behind it, since nothing in the launcher is what its commands
    // act on, and leaving it closes the launcher, back to the viewer. Null
    // while the launcher is over its own rows.
    property var over: null

    // The command line opened over a viewer, as : in swayimg or mpv opens
    // it (launcher-say command-line <viewer> <where> <name>): the viewer's own
    // commands (Options.commandLines, from its module), each carried to it
    // by its program, given where the viewer is, its process or its
    // socket, the command's whole name and what was typed after it, a !
    // after the name where the command says it takes one, as zathura's
    // :write! does; :{n} and a time are its goto. It opens alone, aimed at what the viewer
    // shows, by its name, and :help opens the launcher's Shortcuts at the
    // viewer's own keys, as F1 does. A viewer with none opens nothing.
    function viewerCommandLine(viewer, where, name) {
        const line = Options.commandLines[viewer];
        if (!line)
            return;
        const carry = (name, arg) => Niri.spawn(line.run.concat([where, name, arg]));
        const surface = line.commands.map(own => ({
            command: own.command,
            short: Options.commands[own.command]?.short ?? own.command,
            title: own.title,
            key: own.key ?? "",
            bang: own.bang === true,
            run: call => {
                open = false;
                carry(call.bang ? `${call.name}!` : call.name, call.arg);
            }
        })).concat([{
            command: "help",
            short: Options.commands.help?.short ?? "h",
            title: (Words.field.command ?? {}).help ?? "",
            key: "F1",
            bang: false,
            run: () => into("shortcuts")
        }]);
        // Its commands act on what the viewer shows, never on a row: a
        // thing named for the drawer that holds the viewer's commands
        // rather than being an act of its own (Catalogue.kindOf).
        over = {
            things: name ? [{ name, nested: true }] : [],
            surface,
            goes: line.goes ?? "",
            go: to => {
                open = false;
                carry("goto", String(to));
            }
        };
        open = true;
        held = [];
        restart("");
        commandLine(over);
    }

    // The line let go, the keys back to the find line in the mode it was
    // in, or in normal; over a viewer, the launcher closed, but for what
    // went wrong ("said"), which stays to be read.
    function commandLeft(to) {
        commanding = null;
        if (over !== null && to !== "said") {
            open = false;
            return;
        }
        prompt.resume(to === "said" ? "normal" : to);
    }

    // Whether a line of the menu is chosen, which then wears the selection
    // as the cursor row does, so the foot says Return rather than Tab.
    readonly property bool commandChosen: (commandLoader.item?.chosenLine ?? null) !== null

    // The command line's own keys, as the foot says them while it's open:
    // Return and the command it runs, once one is typed or chosen, since
    // Return on an empty line runs nothing, and Tab while nothing is chosen.
    // Space ? isn't among them, since there Space types a space.
    function commandKeys() {
        const line = commandLoader.item;
        if (!line)
            return [];
        const words = Words.field.command ?? {};
        const keys = [];
        if (line.text.trim() !== "") {
            const read = Commands.ran(line.text, commanding?.menu ?? []);
            keys.push(["\u21b5", read.entry?.command ?? words.run ?? ""]);
        }
        if (!commandChosen && line.shown.length > 0)
            keys.push(["Tab", words.choose ?? ""]);
        return keys;
    }

    // What the drawer says while a line of the menu is chosen: what it
    // acts on, "on Home-5G", or for the several a range or :g binds as
    // typed, how many of the rows, "on 3 of 12".
    function commandPlate(line) {
        if (!commanding)
            return null;
        const aimed = commanding.things.map(thing => thing.name ?? "").filter(name => name);
        // Over a viewer, with nothing chosen, it names what it's aimed at.
        if (!line && over !== null && aimed.length > 0)
            return { title: aimed.join(", "), body: "" };
        if (!line || (line.kind !== "act" && line.kind !== "surface"))
            return null;
        const words = Words.field.command ?? {};
        const read = Commands.ran(commandLoader.item?.text ?? "", commanding.menu);
        const bound = read.error === undefined && read.go === undefined
            ? commandBound(read, commanding) : { things: commanding.things };
        const things = bound.things ?? commanding.things;
        if (things.length > 1) {
            return { title: line.title, body: String(words.onSome ?? "")
                .replace("{some}", things.length).replace("{all}", commanding.rows.length) };
        }
        const names = things.map(thing => thing.name ?? "").filter(name => name);
        if (names.length === 0)
            return null;
        return { title: line.title, body: String(words.on ?? "").replace("{things}", names[0]) };
    }

    function commandSay(words, failed) {
        if (words === "")
            return;
        commandSaid = words;
        commandFailed = failed;
        commandSaidFor = prompt.keysTaken;
    }

    // The options :set takes, by name, as the providers give them (each
    // Provider's setOptions), and the theme's background, which only a
    // switch of the system sets.
    readonly property var setOptions: Object.assign({
        background: { kind: "choice", values: ["light", "dark"], fixed: true,
            now: () => Theme.appearance }
    }, ...sources.map(source => source.setOptions ?? {}))

    // An act's words for the line, by its key and the state its words are
    // in (CommandBinding.wordsFor).
    function commandWords(act) {
        return CommandBinding.wordsFor(act, (id, state) => Words.of(id, state));
    }

    // The options as they stand as : is pressed, each read once, so the menu
    // says where each stands without reading it again on every key: a
    // display's brightness is read from a file, which moves the reader it
    // shares, and read while the menu was worked out, that looped its
    // binding. :set's are the machine's; :setl's those of what it's on.
    function standingOptions(options) {
        const standing = {};
        for (const [name, option] of Object.entries(options)) {
            const now = option.now();
            standing[name] = Object.assign({}, option, { now: () => now });
        }
        return standing;
    }

    // The options :setl sets on each thing it's bound to, those of the
    // things it's inside where it's on one (CommandBinding.localOptions).
    function localOptions(things) {
        return CommandBinding.localOptions(things, held.map(step => step.row));
    }

    // A named command's arguments, as a space after it lists them: :set's
    // options, :setl's of what it's on, a surface command's own, or null
    // for one that has none.
    function commandArguments(entry, arg) {
        if (entry.kind === "set")
            return SetWords.setArguments(arg, commanding?.options ?? {});
        if (entry.kind === "setlocal")
            return SetWords.setArguments(arg, commanding?.local ?? {});
        return entry.kind === "surface" ? Commands.ownArguments(entry, arg) : null;
    }

    // :set's words, each set as its option's row would, or said as Vim's
    // :set volume? says it; :set all says every one, in name order, and one
    // that can't be read says so. Each set is given the change too, so a
    // level only stepped, as a board's lights, steps by it. The first word
    // it can't take says Vim's error and none of them changes. `sets` are
    // the options it sets, the machine's unless :setl gives each thing's.
    function commandSet(arg, sets) {
        const unread = (Words.field.command ?? {}).unread ?? "";
        const planned = [];
        for (const options of sets ?? [setOptions]) {
            const plan = commandSetPlan(arg, options);
            if (plan.error !== undefined) {
                commandSay(plan.error, true);
                return;
            }
            planned.push(plan);
        }
        const said = [];
        for (const { options, steps } of planned)
            said.push(...commandSetSteps(options, steps, unread));
        commandSay(said.filter(words => words !== "").join("  "), false);
    }

    // What :set's words do to a set of options, each change to where it
    // goes from where it stands: { options, steps }, or { error } in Vim's
    // words for the first it can't take.
    function commandSetPlan(arg, options) {
        const read = SetWords.setting(arg, options);
        if (read.error !== undefined)
            return read;
        const steps = read.changes.map(change => change.how === "show" ? { change }
            : { change, to: SetWords.toward(change, options[change.option].now(),
                options[change.option]) });
        const refused = steps.find(step => step.to?.error !== undefined);
        return refused ? refused.to : { options, steps };
    }

    // Sets or says each of a plan's steps, giving what each says.
    function commandSetSteps(options, steps, unread) {
        const said = [];
        for (const { change, to } of steps) {
            if (to !== undefined) {
                said.push(options[change.option].set(to.to, change));
                continue;
            }
            const names = change.option === "all" ? Object.keys(options).sort()
                : [change.option];
            for (const name of names)
                said.push(SetWords.shown(name, options[name].now(), options[name])
                    ?? unread.replace("{option}", name));
        }
        return said;
    }

    // :setl's words, set on each thing the line's bound to, or on what the
    // cursor row is inside, as :set sets the machine's; one with no such
    // option says Vim's E518 and none changes.
    function commandSetLocal(read, opened) {
        const bound = commandBound(read, opened);
        if (bound.error !== undefined) {
            commandSay(bound.error, true);
            return;
        }
        commandSet(read.arg, localOptions(bound.things));
    }

    // Return on the line: exactly what's typed, as Commands.ran read it.
    // From the drawer, :q closes its window, the keys back on the rows, as
    // his Neovim's :q closes a split, and :{n} stays in it.
    function commandRan(read) {
        // A Return doubled with the one that wrote an ending row's line is
        // that one press, so it runs nothing; only a fresh Return does.
        if (commanding?.typed && Date.now() - linedAt < Options.keyTiming.wait)
            return;
        const opened = commanding;
        commandEarlier = Commands.remembered(commandEarlier, read.typed ?? "");
        // What went wrong is said where the line was, over a viewer too,
        // and from the drawer the keys stay in it, as an error leaves his
        // Neovim in the window it was typed in.
        const drawn = QueryKeys.paged(opened.from);
        if (drawn)
            commandLeft(read.error || read.go !== undefined ? "page" : "normal");
        else
            commandLeft(read.error ? "said" : "normal");
        if (drawn && read.error === undefined && read.go === undefined
            && read.entry.kind === "quit")
            return;
        if (read.error !== undefined) {
            commandSay(read.error, true);
            return;
        }
        if (read.go !== undefined && opened.go) {
            opened.go(read.go);
            return;
        }
        // A time is a player's to go to; the rows have none.
        if (typeof read.go === "string" && read.go !== "$") {
            commandSaid = `E492: Not an editor command: ${read.go}`;
            commandSaidFor = prompt.keysTaken;
            return;
        }
        if (read.go !== undefined) {
            const last = matches.length - 1;
            arrowed(read.go === "$" ? last : Math.max(0, Math.min(read.go - 1, last)), ":");
            return;
        }
        const entry = read.entry;
        switch (entry.kind) {
        case "quit":
            open = false;
            return;
        case "help":
            listing = true;
            return;
        case "set":
            commandSet(read.arg);
            return;
        case "setlocal":
            commandSetLocal(read, opened);
            return;
        case "history":
            commandLine({ earlier: true });
            return;
        }
        const bound = commandBound(read, opened);
        if (bound.error !== undefined) {
            commandSay(bound.error, true);
            return;
        }
        if (entry.kind === "surface") {
            entry.own.run({ name: entry.command, bang: read.bang, arg: read.arg,
                things: bound.things, count: bound.things.length });
            return;
        }
        if (bound.things.length !== 1 || bound.things[0] !== opened.things[0]) {
            commandAcross(entry, read, bound.things);
            return;
        }
        // An act that asks for a target or for words is stepped into, as
        // Return on it would, from inside the thing it belongs to.
        const act = entry.act;
        const thing = opened.things[0];
        if ((act.targets || act.given) && act !== thing && enter(thing)) {
            const step = held[held.length - 1];
            run(step.rows.find(one => one.key === act.key) ?? act, false);
            return;
        }
        commandAsked(entry, [{ act, thing }], 1, read.bang);
    }

    // Runs a line's acts on what it's bound to, `found` each act with its
    // thing, asking once first where CommandBinding.asking says, by d's
    // rule, through the one ask (askOnce), in d's words for one thing or
    // many (Confirm.question), the rows it covers tinted: y runs them past
    // their own confirm, as a ! does; n, Escape and Ctrl+C leave them be.
    function commandAsked(entry, found, all, bang) {
        const words = Words.field.command ?? {};
        const acts = found.map(one => one.act);
        const asked = CommandBinding.asking(entry, acts, { all, bang }, Options.commands ?? {});
        const fill = template => {
            const said = String(template ?? "").replace(/\{command\}/g, entry.command)
                .replace("{some}", acts.length).replace("{all}", all);
            return said.charAt(0).toUpperCase() + said.slice(1);
        };
        if (asked.none) {
            commandSay(fill(words.none), true);
            return;
        }
        const sure = () => commandActs(acts.map(act => act.confirm
            ? Object.assign({}, act, { confirm: false }) : act));
        if (asked.ask !== "") {
            // The ask says the act's own verb, never the line's command,
            // and for an act the cursor is on, what it's an act of.
            const of = one => one.act === one.thing && held.length > 0
                ? askedOfHolder(held[held.length - 1].row, one.thing) : one.thing;
            const question = asked.ask === "some" ? fill(words.some)
                : Confirm.questionParts(found.map(one => ({ ends: one.act.ends || verbOf(one.act),
                    asked: askedOf(one.act),
                    name: one.act.askedAs || (of(one)?.name ?? ""), kind: kindOf(of(one)),
                    leaves: one.act.leaves ?? "" })),
                    Words.field.operators ?? {});
            askOnce(question, sure, () => {}, found.map(one => matches.indexOf(one.thing))
                .filter(at => at >= 0));
            return;
        }
        if (bang) {
            sure();
            return;
        }
        commandActs(acts);
    }

    // A line's acts run: one u can take back done where it stands, as d
    // does it, the launcher kept open so u can, as Vim's :g/x/d leaves the
    // buffer in view; the rest as Return does them. Where every one can be
    // taken back, the foot says so, as after d. One done as d does it is
    // a change made where the row stands rather than a thing chosen, so
    // it teaches the find nothing and starts no act's trace, as d from
    // the rows doesn't; Return's acts from the line still do both.
    function commandActs(acts) {
        for (const act of acts) {
            if (typeof act.undo === "function")
                Operators.doing(act)();
            else
                run(act, false);
        }
        if (changedBy(acts) === acts.length)
            operatorTells(Operators.doneSaid(acts.length, Words.field.operators ?? {}));
    }

    // @:, the last line Return ran run again on the cursor row as it is
    // now, or Vim's E30 where none has.
    function commandAgain() {
        const last = commandEarlier[commandEarlier.length - 1];
        if (last === undefined) {
            commandSay("E30: No previous command line", true);
            return;
        }
        commandLine(null);
        commandRan(Object.assign(Commands.ran(last, commanding.menu), { typed: last }));
    }

    // The things a line acts on: those bound as : was pressed, or where a
    // range or :g is typed, the rows it names as they stood then
    // (CommandBinding.bound), so a row arriving after can't shift them.
    function commandBound(read, opened) {
        if ((read.range ?? null) === null && !read.global)
            return { things: opened.things };
        const bound = CommandBinding.bound(read, { names: opened.rows.map(row => String(row.name ?? "")),
            cursor: opened.cursor, marks: opened.marks });
        return bound.error !== undefined ? bound
            : { things: bound.rows.map(at => opened.rows[at]) };
    }

    // A line run on several things, or on one a range or :g found: each
    // thing's own act (CommandBinding.actFor), asked first where it asks.
    function commandAcross(entry, read, things) {
        const found = things.map(thing => ({ act: CommandBinding.actFor(entry, actsOf(thing),
            commandWords), thing }))
            .filter(one => one.act !== undefined);
        commandAsked(entry, found, things.length, read.bang);
    }

    // Where the launcher is now, for a step into something to keep.
    function here() {
        const row = selected;
        return {
            typed: typed,
            at: current,
            key: row ? steadyId(row) : "",
            // How far below the list's top its row stood, so coming back
            // stands it there again, wherever the rows made again begin.
            above: current >= 0 ? list.rowY(current) - list.contentY : 0
        };
    }

    // Selects what was selected before, by what it is when it's still
    // there, so of two windows titled alike it's the same one, since what
    // was searched again may have moved it, or else by where it was, and
    // scrolls the list back to where it stood.
    function reselect(left) {
        const byKey = left.key === "" ? -1
            : matches.findIndex(row => steadyId(row) === left.key);
        const lowest = answer || lineHeads ? -1 : 0;
        const at = left.at === -1 && (answer || lineHeads) ? -1
            : byKey >= 0 ? byKey : Math.max(lowest, Math.min(left.at, matches.length - 1));
        current = at;
        if (current < 0)
            return;
        list.positionViewAtIndex(current, ListView.Contain);
        const above = Math.max(0, Math.min(left.above ?? 0, list.height - list.rowHeight(current)));
        list.contentY = list.clamped(list.rowY(current) - above);
    }
    readonly property var offered: held.length > 0 ? held[held.length - 1].rows : []

    // A row on offer now by its key, as it reads now: in the step held, or
    // at the top. Undefined where it's gone.
    function onOffer(key) {
        return held.length > 0 ? offered.find(row => row.key === key && !row.gone) : keyed[key];
    }

    // The rows shown, said again as they are now (Steady.js), each in its
    // place. One still on offer keeps it as it reads now, though what it
    // says no longer matches what's typed: a Play that Return made a Pause
    // shows Pause, rather than going.
    // The row Return last changed something with, kept among what's found
    // until Joel types or steps again, so Wi-Fi Off that became Wi-Fi On
    // still shows it, though "wifi off" no longer finds it.
    property var acted: null
    function stillActed(found) {
        const row = acted ? onOffer(acted.key) : undefined;
        return row && !found.some(one => one.key === row.key) ? [row] : [];
    }

    function steadied(found) {
        // What ends a search comes after it all and is made again with it,
        // so it's no row to hold in place: held, it stood dimmed above the
        // one made again.
        const shown = matches.filter(row => !row.endedBy);
        const renamed = shown.filter(row => !row.gone && onOffer(row.key)
            && !found.some(one => one.key === row.key)).map(row => onOffer(row.key));
        if (!holdingRows) {
            waitingRows = 0;
            return Steady.kept(shown, found.concat(renamed), pairedId);
        }
        const still = Steady.held(shown, found.concat(renamed), pairedId);
        waitingRows = still.waiting;
        return still.rows;
    }

    // While visual, a pending operator or the command line acts on the
    // rows, they hold still: what arrives waits, counted in the foot as
    // "2 new", and comes in at the end once it ends, as a refresh adds it
    // (VKCF3Z's spec, the foot's fact by VKCF3Z, 2026-11-05).
    readonly property bool holdingRows: open && (visual || pendingSaid !== ""
        || commanding !== null)
    property int waitingRows: 0
    readonly property string waitingSaid: waitingRows === 0 ? ""
        : (Words.field.waiting ?? "").replace("{count}", String(waitingRows))
    onHoldingRowsChanged: {
        if (!holdingRows && waitingRows > 0)
            reshow(held.length);
    }

    // Whether the step held last is a collection, whose members are things
    // with verbs of their own, as at the top; or a row that says its verbs
    // are things too (nested), as a report's rows are, so stepping goes on
    // down as far as it holds anything.
    readonly property bool collecting: held.length > 0 && collects(held[held.length - 1])

    function collects(step) {
        return !!step.row.from || !!step.row.nested;
    }

    // What a row holds, led by Return's own act where it was left out
    // (Catalogue.js), that act's name prepared to be found as a
    // provider's rows are.
    function closingLed(row, made) {
        const own = Catalogue.closingFirst(row, made, Theme.glyph.copy);
        // Its act leads, prepared for searching as the rows beside it are.
        if (!own[0].prepared)
            own[0].prepared = Fuzzy.prepare(own[0].name);
        return own;
    }

    function led(row, made) {
        const own = Catalogue.leading(row, made, Theme.glyph.copy);
        // Return's act leads, or a copy comes after the acts.
        const added = own === made ? undefined
            : own.find(one => String(one.key).startsWith("return:") && !one.prepared);
        if (added)
            added.prepared = Fuzzy.prepare(added.name);
        return own;
    }

    // What a step into a row lists of its own: a thing's verbs, led by
    // Return's act, or a verb's targets; null where it holds none. Made the
    // same way when a step is made again, so a refresh never finds the
    // act it led with gone.
    function stepRows(row, things) {
        // One that keeps the launcher holds its act, which closes it, so
        // the right arrow then Return on it does it and closes.
        if (keepsOpen(row) && (things || !row.targets))
            return closingLed(row, things && row.verbs ? row.verbs() : []);
        const made = things ? (row.verbs ? row.verbs() : null) : row.targets ? row.targets() : null;
        // A row that would only copy gains its copy inside wherever it's
        // stepped into, so the copy is never lost to Return stepping in.
        return made && (things || copiesOnly(row)) ? led(row, made) : made;
    }

    // What a step into a row holds: its own rows, then a thing's parts
    // after what can be done to it. Inside a network or a device whose own
    // act changes something, which Return steps into rather than doing,
    // the first is never one that changes anything, so a Return that steps
    // in and a second after it only ever show (Catalogue.js's quietFirst).
    // Anywhere else its own order stands: a setting's act still leads its
    // step, a sound output's use, and an answer's levels its copy. Null
    // where it holds nothing.
    function inside(row, things) {
        const own = stepRows(row, things);
        const parts = things ? partsIn(row) : [];
        const into = parts.length > 0 ? (own ?? []).concat(parts) : own;
        return into && things && stepsThing(row)
            ? Catalogue.quietFirst(into, one => root.commandNamed(one)) : into;
    }

    // Steps into a row: a thing's verbs, or a verb's targets. Says whether
    // there was anywhere to go.
    function enter(row) {
        if (!row || row.gone)
            return false;
        traces.began("step", { into: row.key ?? "", depth: held.length });
        // A step is timed to its frame as a key is, named by the arrow and
        // the row, so a slow place shows in the journal and the trials
        // (Joel, 2026-10-28: "programs from the main launcher screen takes
        // for fucking ever to load").
        const began = Date.now();
        const heard = KeyTimes.now();
        const pressed = KeyTimes.pressed();
        if (held.length > 0 && row.given) {
            ask(row);
            return true;
        }
        // A place found from the top that sits in another is stepped into
        // through it, so the path says where it lives.
        if (held.length === 0 && row.category && placesAbove(row.category).length > 0) {
            intoCategory(row.category, typed);
            return true;
        }
        if (steppedAsLed(row))
            return true;
        const things = asThing(row, held.length === 0 || collecting);
        const into = inside(row, things);
        // A collection with nothing in it still holds the places its
        // vocabulary names, which a step into it shows: Agents with no
        // agent running still leads to its Quests and Runs. A row
        // saying what it holds when empty, as Waiting on You does, is a
        // place stepped into however little it holds.
        const places = things && row.from ? partsOf(row.from).length : 0;
        if ((!into || into.length === 0) && places === 0 && !row.empty)
            return false;
        const made = Date.now();
        // Going back lands on the row stepped into, as oil's - lands on the
        // folder it came from, though it was stepped into without being
        // selected, as the file picker holds each folder on the way to
        // where it opens; Return steps into the selection, so it's the same.
        held = held.concat([Object.assign(here(), { key: steadyId(row), row: row,
            rows: into ?? [] })]);
        const pushed = Date.now();
        if (row.from)
            row.from.opened();
        // A row that marks what it holds once Joel's looked inside, as
        // For You's notices count as seen, says so only on a step in,
        // never when `:` lists its acts (Now.qml's noticesRow).
        if (typeof row.stepped === "function")
            row.stepped();
        const opened = Date.now();
        // What was typed found the row, so it goes: inside, everything the row
        // holds shows, and the left arrow puts it back on the way out. Stepped
        // into from a collection's match, the match stays selected among all
        // it holds (Joel, 2026-10-26: stepping in keeps the match). A step
        // whose line holds its words, as a save's name, puts them back.
        searchParts = null;
        restart(row.words?.() ?? "");
        placeCaret(row);
        if (row.from) {
            const matched = matches.findIndex(one => steadyId(one) === steadyId(row));
            if (matched >= 0) {
                current = matched;
                list.positionViewAtIndex(matched, ListView.Contain);
            }
        }
        // Said as a key's are: making the step's lists, what each part of
        // stepping took, and the search that drew its rows, whose parts
        // restart() left.
        searchParts = trials.stepped(searchParts, { began, made, pushed, opened,
            ended: Date.now() });
        keyTimed(`\u2192 ${row.name}`, began, heard, pressed);
        return true;
    }

    // What the field is asking for words for, such as a password: the step
    // held last, when its row hands what's typed to `given`.
    readonly property var asking: held.length > 0 && held[held.length - 1].row.given
        ? held[held.length - 1].row : null

    // Asks for words in the field, with nothing to choose from below it.
    function ask(row) {
        held = held.concat([Object.assign(here(), { row: row, rows: [] })]);
        restart("");
        // Words are what it asks for, so the line is typed in at once.
        prompt.typing();
    }

    // Hands over what was typed, and empties the field for another try
    // while the footer says how it went.
    function give() {
        const words = prompt.text;
        if (words === "")
            return;
        asking.given(words);
        prompt.text = "";
        searchNow();
    }

    // A collection whose things changed while it's open is searched again,
    // and so is anything stepped into below it, each step's rows asked for
    // again, since they can say how a thing stands, as a Wi-Fi network's say
    // its access points; and a category holding it has its members made
    // again, so Agents in "ai" says how they stand. What's shown refreshes
    // in place (Steady.js): the rows keep the order they're shown in, new
    // ones come at the end, the selection stays on its row and the list
    // where it was scrolled, so nothing moves under the cursor. Only what
    // Joel types orders by rank.
    // Taken once the change has settled, since a provider may say it
    // changed from within its own things changing (Session.qml does),
    // before the rows read from them (choices, partsOfRows) have heard;
    // each source heard is made again once, in the order heard.
    property var refreshing: []
    // A change told from a daemon's line is timed from it, on the
    // monotonic clock, to the frame niri presents with it (Hearing.js).
    function updatedFrom(source) {
        const line = Hearing.heardAt();
        if (open && line > 0)
            updatesWaiting = Presented.heard(updatesWaiting, source.settings.name,
                line + KeyTimes.now() - Date.now());
    }
    function refreshed(source) {
        updatedFrom(source);
        // A collection's note is read as the corpus is joined, so a source
        // saying it changed marks it stale even when its things are the
        // same.
        choicesStale = true;
        // Only a key that acted owns a change beside it (Pacing.keyed),
        // timed from when its work was done (Pacing.sinceKey).
        const ago = Pacing.sinceKey(KeyTimes.now(), keyHeardAt, keyDoneAt);
        const byKey = open && Pacing.keyed(ago, prompt.keyMoved, keyedWithin);
        // One just past it is said, so a key's change left to the pace is
        // seen for what it was.
        if (open && !byKey && !prompt.keyMoved && ago < 1000)
            Report.say(`heard ${source.settings.name} ${Math.round(ago)} ms after a key's work`);
        if (byKey && !keyedActs.some(one => one.source === source && one.heard === keyHeardAt)) {
            const act = { source: source, heard: keyHeardAt, done: Infinity };
            keyedActs = keyedActs.filter(one => one.heard === keyHeardAt).concat([act]);
            // When the key's own work is done, and the change could be shown.
            Qt.callLater(() => act.done = KeyTimes.now());
        }
        // What a key changed is shown at once, whatever else waits on the
        // pacing below, so it's in the key's own frame or the next; and so
        // is a daemon's word on what an act asked it, whenever it comes.
        if (byKey || Object.values(becoming).some(one => one.from === source)) {
            if (byKey)
                refreshesKeyed += 1;
            if (!refreshing.includes(source))
                refreshing = refreshing.concat([source]);
            Qt.callLater(refreshDue);
            return;
        }
        if (refreshing.includes(source))
            return;
        refreshing = refreshing.concat([source]);
        // The first change is taken at once, and those after it within a
        // quarter second together (launcher/Pacing.js).
        if (!open) {
            Qt.callLater(refreshDue);
            return;
        }
        const heard = Pacing.heard(pace, Date.now(), refreshPace);
        keepPace(heard.pace);
        if (heard.now)
            Qt.callLater(refreshDue);
    }
    readonly property int refreshPace: 250
    // How many refreshes the open launcher has made, for the check that
    // it makes at most four a second however fast its sources change, and
    // how many changes were taken at once as a key's own, for the check
    // that an arrow takes none.
    property int refreshesDone: 0
    property int refreshesKeyed: 0
    property var pace: Pacing.rested()
    // The timer rings when the pace says a refresh is due, and is stopped
    // once none waits.
    function keepPace(now) {
        pace = now;
        if (now.due === null) {
            refreshPacing.stop();
        } else if (!refreshPacing.running) {
            refreshPacing.interval = Math.max(0, now.due - Date.now());
            refreshPacing.start();
        }
    }
    Timer {
        id: refreshPacing
        onTriggered: {
            root.keepPace(Pacing.rang(root.pace));
            root.refreshDue();
        }
    }

    // What a key changed, timed from the key to the frame that shows it
    // (Joel, 2026-11-07: "when i modify something in the launcher, like
    // marking read, snoozing, ignore, whatever, it should show right
    // away"). A source saying it changed within a frame or two of a key
    // changed by what the key did, as Read or Not Now does it; each is
    // kept with when that key was heard and when its work was done, until
    // the refresh that shows it is drawn.
    readonly property int keyedWithin: 50
    property real keyHeardAt: -Infinity
    property real keyDoneAt: -Infinity
    property var keyedActs: []
    property var actsDrawing: Pacing.unframed()

    Connections {
        target: prompt

        // A letter typed into the find changes nothing, and a source
        // changing beside one is left to the pacing, as Slack's answers
        // arriving while Joel types are.
        function onKeysTakenChanged() {
            if (prompt.typingNow && prompt.keyTaken.length === 1)
                return;
            const heard = KeyTimes.now();
            root.keyHeardAt = heard;
            // Its work is done once its handler has returned.
            Qt.callLater(() => {
                if (root.keyHeardAt === heard)
                    root.keyDoneAt = KeyTimes.now();
            });
        }
    }

    // The acts whose refresh has run, begun at `begun`, to be timed by the
    // frame after it.
    function actsRefreshed(due, begun) {
        const shown = keyedActs.filter(one => due.includes(one.source));
        if (shown.length === 0)
            return;
        keyedActs = keyedActs.filter(one => !due.includes(one.source));
        for (const one of shown)
            one.begun = begun;
        actsDrawing = Pacing.refreshedActs(actsDrawing, shown);
    }

    // Each act a frame showed, `shown`, timed to it, said for the test and,
    // slow enough to feel, in the journal, as a key's is (Trials.qml). Only
    // the last key's: one an earlier key changed, whose refresh drew nothing
    // new, had no frame of its own, and the one now drawn is another key's.
    function actsDrawn(shown) {
        const drawn = KeyTimes.now();
        for (const one of shown.filter(each => each.heard === keyHeardAt)) {
            const took = Math.round(drawn - one.heard);
            const waited = Math.round(one.begun - one.done);
            const name = one.source.settings.name;
            Report.say(`act shown in ${name} after ${took} ms, `
                + `refreshed ${waited} ms after the key's work`);
            if (took >= 50)
                console.info(
                    `launcher: what a key changed in ${name} took ${took} ms to its frame`);
        }
    }
    // A condition a shown place names has moved, as when the charger goes
    // in: its rows are made again, so what it held stays dimmed and says
    // why, rather than freezing as it was (see `conditions`).
    function conditionMoved() {
        if (!open || asking || !held.some(step => !!step.row.category?.while))
            return;
        const steps = held.map(step => step.row.category?.while ? Object.assign({}, step, {
            rows: Steady.kept(step.rows, members(step.row.category), pairedId)
                .map(row => heldBack(step.row.category, row))
        }) : step);
        if (!remade(steps, held.length))
            return;
        held = steps;
        reshow(steps.length);
    }

    // Each place held said again from its members, as a provider whose
    // rows it holds first hears its daemon: a row it couldn't make before
    // its records came, as Heat's, comes in place, and a place's own row
    // says how it stands now, as Power's charge in Machine. Before, each
    // read its source as it was asked, so the place was whole at once.
    // The place's own order wins over Steady's, which puts a row new to
    // it last: Heat belongs between Memory and Disk, and the place is
    // still being made, a moment after it was stepped into.
    function placesRead() {
        if (!open || asking || !held.some(step => !!step.row.category))
            return;
        const steps = held.map(step => step.row.category ? Object.assign({}, step, {
            rows: members(step.row.category).map(row => heldBack(step.row.category, row))
        }) : step);
        if (!remade(steps, held.length))
            return;
        held = steps;
        reshow(steps.length, true);
    }

    // A row a condition holds back now, dimmed, saying why in the place's
    // words; any other as it is. It's judged by the row's id alone, so a
    // row said again from a new reading is held back the same as one
    // Steady kept.
    function heldBack(category, row) {
        const asked = "answer:";
        const id = (row.is ?? "").startsWith(asked) ? row.is.slice(asked.length) : "";
        const named = (category.while ?? {})[id];
        if (named === undefined || holdsNow(category, id))
            return row;
        return Object.assign({}, row, { gone: true, note: (category.said ?? {})[named] ?? "" });
    }

    Connections {
        target: UPower
        function onOnBatteryChanged() {
            root.conditionMoved();
        }
    }

    function refreshDue() {
        if (refreshing.length === 0)
            return;
        keepPace(Pacing.took(Date.now()));
        const refreshedAt = pace.at;
        const begun = KeyTimes.now();
        if (open) {
            refreshesDone += 1;
            freshen();
        }
        const due = refreshing;
        // When each began, by the shell's clock, which the pacing goes by:
        // a report said as one ends is moved by its own work.
        if (open)
            for (const source of due)
                Report.say(`refresh of ${source.settings.name} began ${refreshedAt}`);
        refreshing = [];
        // The list is shown again once for every source that changed it,
        // not once each: at home, sessions and people changing together
        // composed it twice a refresh (keeps-up, 48 times in 46 refreshes,
        // 2026-11-09).
        const done = due.map(source => {
            const startedAt = Date.now();
            const started = KeyTimes.now();
            const shown = refreshNow(source);
            return { source, startedAt, shown, tookMs: KeyTimes.now() - started };
        });
        if (done.some(one => one.shown))
            reshow(held.length);
        for (const one of done)
            updateDrawn(one.source.settings.name, one.shown);
        // Each a span of the opening (Traces.qml), saying which source
        // changed, how long its own work took by the shell's clock, and how
        // many rows the list held after (Y2YBTS, 2026-11-07).
        for (const one of done) {
            if (one.shown)
                Report.say(`refreshed ${one.source.settings.name}`);
            traces.did("refresh", one.startedAt, { source: one.source.settings.name,
                tookMs: one.tookMs, rows: matches.length, depth: held.length });
        }
        actsRefreshed(due, begun);
        heldAgreed(due);
    }

    // What an act's rows will be once the daemon it asks has done it, by
    // each row's key, drawn now and held until its source agrees
    // (launcher/Becoming.js): Wi-Fi's Disconnect reads disconnected on the
    // key's own frame, not once wifid has said so.
    property var becoming: ({})
    readonly property bool anyBecoming: Object.keys(becoming).length > 0
    // How long a daemon has to agree before its row is put back: wifid
    // leaves a network, bluez a device and systemd starts a service well
    // within it.
    readonly property int becomingWithin: 5000

    // A row of `from`'s, by its key, drawn as `row` from now until its source
    // makes it so, or left out where `row` is null. What's said if it never
    // comes names the row as `name` and what was asked by `act`'s verb. One
    // whose outcome can't be foretold says when it's `done`, and one slower
    // than a daemon how long it's waited for, `within` ms (Becoming.js).
    function becomes(from, key, row, said) {
        becomesAll(from, [{ key: key, row: row }], said);
    }

    // Several of `from`'s rows, each { key, row }, held as one act's.
    function becomesAll(from, holds, said) {
        if (holds.length === 0)
            return;
        const now = Object.assign({}, becoming);
        const at = Date.now();
        const how = Object.assign(typeof said.done === "function" ? { done: said.done } : {},
            said.within > 0 ? { within: said.within } : {});
        for (const one of holds) {
            now[one.key] = Object.assign({ key: one.key, from: from, row: one.row,
                name: said.name, act: said.act, at: at }, how);
            Report.say(`becoming ${one.key}`);
        }
        becoming = now;
        timeBecoming();
        refreshed(from);
    }

    // Its source wouldn't do what the row was to become, saying why in its
    // own words, or "" for no word yet.
    function becameNot(key, why) {
        const one = becoming[key];
        if (one)
            takenBack(one, why);
    }

    function takenBack(one, why) {
        letGo([one.key]);
        toldNot(one.name, one.act, why);
        Report.say(`not become ${one.key}${why === "" ? "" : `: ${why}`}`);
        refreshed(one.from);
    }

    // That an act on `name` wasn't done, `why` in the daemon's words, or
    // with none, that it hasn't been yet: said now while the launcher is
    // open, else as it next opens, as an act that shut it is told.
    function toldNot(name, act, why) {
        const words = Words.field.becoming ?? {};
        const what = (why === "" ? words.late : words.refused) ?? "";
        tellOnOpen(Becoming.told({
            mark: Theme.glyph.failed,
            what: what.replace("{object}", name)
                .replace("{did}", (words.did ?? {})[act] ?? act),
            why: why === "" ? words.noAnswer ?? "" : why
        }, { what: Theme.alert, why: Theme.muted }), true);
    }

    function letGo(keys) {
        becoming = Becoming.without(becoming, keys);
        timeBecoming();
    }

    // The rows to show, each held row as it will be.
    function live(rows) {
        if (!anyBecoming)
            return rows;
        const laid = Becoming.laid(rows, becoming, reads);
        if (laid.agreed.length > 0)
            agreed(laid.agreed);
        return laid.rows;
    }

    function reads(one, row) {
        return one.from.says([row], [], true);
    }

    function agreed(keys) {
        for (const key of keys)
            Report.say(`became ${key}`);
        letGo(keys);
    }

    // The row a hold's source makes now, wherever the launcher reads it: its
    // things, its collection's members, or a step shown; null where none
    // is, and undefined where it can't be told, as a step's act once the
    // step is left.
    function nowRow(one) {
        const key = one.key;
        const own = one.from.rowNow(key);
        if (own !== undefined)
            return own;
        const made = one.from.things.concat(one.from.everything())
            .concat(...held.map(step => step.rows ?? []));
        const found = made.find(row => row.key === key && !row.gone);
        if (found)
            return found;
        return String(key).startsWith("verb:") && !open ? undefined : null;
    }

    // Each hold whose source has just been heard from, let go once its row
    // reads as it was to become, or is gone as it was to go.
    function heldAgreed(sources) {
        if (!anyBecoming)
            return;
        const done = Object.values(becoming).filter(one => sources.includes(one.from))
            .filter(one => {
                const now = nowRow(one);
                return now !== undefined && Becoming.agrees(one, now, reads);
            });
        if (done.length > 0)
            agreed(done.map(one => one.key));
    }

    // A hold no source agreed to in time is put back, saying so, unless its
    // row agrees now, or can't be read to tell.
    function becameLate() {
        for (const key of Becoming.late(becoming, Date.now(), becomingWithin)) {
            const one = becoming[key];
            const now = nowRow(one);
            if (now === undefined || Becoming.agrees(one, now, reads))
                agreed([key]);
            else
                takenBack(one, "");
        }
    }

    function timeBecoming() {
        const due = Becoming.nextDue(becoming, Date.now(), becomingWithin);
        if (due < 0) {
            becomingDue.stop();
            return;
        }
        becomingDue.interval = Math.max(1, due);
        becomingDue.restart();
    }

    Timer {
        id: becomingDue
        onTriggered: root.becameLate()
    }
    // Whether what's shown is to be said again for the source, which
    // refreshDue does once for all of them; the steps held are made again
    // here, each source's on the last's.
    function refreshNow(source) {
        if (!open || asking)
            return false;
        // Out at the top, a row the search found from it is said again in
        // place, so a setting Return changed shows how it stands now
        // (Joel, 2026-10-26: an act that changes something stays open and
        // shows the new state on its row at once).
        if (held.length === 0) {
            // Home, with nothing typed, names each collection with what it
            // holds, so a collection that changed says it again in place:
            // Agents makes its rows only once the launcher is open, after
            // home was first drawn from none, and a session can start while
            // home is shown.
            // The collection found by name is said again too, since it can
            // say how what it holds stands (homeSays), as Agents' row does.
            // And what's typed is searched again when the source now holds
            // a thing it finds, though none was shown: a document Drive
            // sent once its daemon answered, after the words were typed,
            // never showed until they changed (2026-11-07, the picker's
            // Drive save in vm-desktop-opens-files, core #2148).
            const home = typed.trim() === "" && collections.includes(source);
            return home || typed.trim() !== "" && (matches.some(row => row.endedBy === source
                || row.key === `collection:${source.settings.name}`
                || source.things.some(own => own.key === row.key)) || reaches(source));
        }
        const from = held.findIndex(step => step.row.from === source);
        const ofSource = step => !!step.row.category && holdsFrom(step.row.category, source);
        // A thing of its own stepped into from the top, as a program is, is
        // made again too, with everything it holds, so its Media and Sound
        // say how its player and its streams stand now. What the top found
        // is one of the rows its source's search looks through
        // (looksThrough), which can be more than its things: People lists
        // none and finds anyone, so its things alone missed a person found
        // by name, whose Write in Slack then never lit where it stood. Those
        // are read only when its things don't hold the row, and only for a
        // step stepped into from the top.
        const fromTop = from < 0 && held.length > 0 && !held[0].row.from && !held[0].row.category;
        const found = row => row.key === held[0].row.key;
        // One a search ended with is asked of its source again, for what
        // was typed then, and kept knowing who gave it.
        const endedFor = held.length > 0 ? held[0].row.endedFor : "";
        const ended = fromTop && held[0].row.endedBy === source ? source.last(endedFor) : null;
        const own = !fromTop ? [] : ended
            ? [Object.assign({}, ended, { endedBy: source, endedFor: endedFor })]
            : source.things.some(found) ? source.things : source.looksThrough();
        const top = fromTop && own.some(found);
        if (from < 0 && !top && !held.some(ofSource))
            return false;
        const steps = held.map(step => ofSource(step) ? Object.assign({}, step, {
            rows: Steady.kept(step.rows, members(step.row.category), pairedId)
                .map(row => heldBack(step.row.category, row))
        }) : step);
        const above = top ? own : from < 0 ? [] : source.everything();
        if (!remade(steps, top ? 0 : from < 0 ? held.length : from + 1, above))
            return false;
        held = steps;
        return true;
    }

    // Each step from `from` on made again from the rows the one above it
    // offers now (`above`, or that step's own rows), as enter made it:
    // a thing at the top, a collection's things or a nested row's, asked
    // for their verbs and then their parts, anything else for its targets.
    // A step entered from what was typed is made again from its own row,
    // since what was typed found it, not the rows above, as a place the
    // picker holds at home is found from any folder. Says whether every
    // step was still there.
    function remade(steps, from, above) {
        let rows = above ?? (from > 0 && from <= steps.length ? steps[from - 1].rows : []);
        for (let at = from; at < steps.length; at++) {
            const step = steps[at];
            const thing = rows.find(row => sameRow(row, step.row))
                ?? ((step.typed ?? "") !== "" ? step.row : undefined);
            const things = !!thing && asThing(thing, at === 0 || collects(steps[at - 1]));
            const into = !thing ? null : inside(thing, things);
            if (!into)
                return false;
            steps[at] = Object.assign({}, step, {
                row: thing,
                rows: Steady.kept(step.rows, into, pairedId)
            });
            rows = into;
        }
        return true;
    }

    // What's shown searched again under what's held now, `count` steps,
    // and kept in place: the rows in the order they're shown, the
    // selection on its row and the list where it was scrolled.
    // The search is told it's only saying what's there now (steadying), so
    // it keeps the rows in their order itself and leaves the selection and
    // the list alone, rather than starting them afresh to be put back: the
    // rows are made once a refresh, not twice.
    // A source's search found more once asked, as People's does when the
    // people daemon answers, and only then, never on its other changes:
    // what's typed is searched again, ranked as the key's own search when
    // the answer came fresh, so soon that its first frame hardly showed
    // without it, and otherwise in place, holding still what's shown.
    function foundMore(fresh) {
        if (!open || asking || typed.trim() === "")
            return;
        // Rows held still take what was found in their order, waiting.
        if (fresh && !holdingRows)
            search(typed);
        else
            reshow(held.length);
    }

    // Given `ordered`, a place's rows take its own order rather than the
    // order they were shown in, as a place still being made does
    // (placesRead); the selection stays on its row either way.
    function reshow(count, ordered) {
        const was = current >= 0 && selected ? pairedId(selected) : "";
        const at = current;
        // A provider that throws mid-search mustn't leave every later search
        // keeping its rows as a refresh does.
        steadying = true;
        ordering = ordered === true;
        list.afresh = false;
        try {
            search(typed);
        } finally {
            steadying = false;
            ordering = false;
        }
        // What's typed asked something whole, out at the top, now it can.
        if (held.length !== count)
            return;
        const lowest = (answer && !parted(answer)) || lineHeads ? -1 : 0;
        const now = at === -1 && (answer || lineHeads) ? -1 : Steady.selection(matches, {
            row: was,
            at: at,
            lowest: lowest
        }, pairedId);
        // The list stays where it stands: Qt keeps the rows in view still as
        // rows above them come or go. Put back to where it had been scrolled,
        // it was put to a place the rows had left, above the first once rows
        // above it went, and stopped a glide short (Joel, 2026-11-08).
        if (current !== now)
            current = now;
        // Rows listed afresh stand from the list's start, so the selection
        // is put back in view wherever it stands, at once: left there, a row
        // kept in its place sat below the card unseen.
        if (list.afresh && current >= 0)
            list.positionViewAtIndex(current, ListView.Contain);
        else
            root.showMoved(at);
    }

    // Whether the search under way only says again what's there, as a
    // refresh does (reshow).
    property bool steadying: false
    // Whether a place said again takes its own order (reshow).
    property bool ordering: false

    // What a row is, to hold it in place as it changes: what it says it
    // is, or for a part with no key, as an answer's figures have none, its
    // name, so the Cores stay the Cores as their loads move.
    function steadyId(row) {
        return Rank.identity(row) || `named:${row.name ?? ""}`;
    }

    // What a row is to the row it stands in for as the rows change, and
    // only there: Steady's pairing, the selection held and the rows kept
    // drawn. A row taking the place a dimmed one held while it was asked
    // for says so with `steadyAs`, the one it takes over from, as Search
    // Slack's first message takes its stand-in's light. Everything else,
    // a trail, a spot or what follows a row, knows it by what it is, so
    // that message is the one For You lists.
    function pairedId(row) {
        return row.steadyAs ?? steadyId(row);
    }

    // Whether a row offered now is the one held: by its key, or else by
    // what it says it is, or a part with neither by its name.
    function sameRow(row, was) {
        if ((was.key ?? "") !== "")
            return row.key === was.key;
        return (was.is ?? "") !== "" ? row.is === was.is : row.name === was.name;
    }

    // Whether a category holds something of a source's, by the provider
    // each thing it holds comes from, so its members are made again only
    // when one of theirs changed.
    // A place holds a source's row by its word's provider, by the source's
    // own name, or as one of its things: Desktop's Do Not Disturb is the
    // session's, and no word says so, so its act never showed while
    // Desktop was held (2026-11-07).
    function holdsFrom(category, source) {
        const name = source.settings.name;
        return holdings(category).some(id => (Words.of(id).provider ?? id) === name
            || source.things.some(own => own.key === id));
    }

    // The place beside the one held deepest, the next (1) or the one before
    // (-1), where that place says what's beside it (`beside`), as the file
    // picker's folders do; anywhere else ]b and [b do nothing.
    function beside(by) {
        held[held.length - 1]?.row?.beside?.(by);
    }

    // Whether - has a place to go up to: anywhere but home, and but a
    // collection with nothing above it, as a dialog's (`outermost`), which
    // - would leave for a home it never showed, its ask still waiting.
    readonly property bool canGoUp: held.length > 1
        || (held.length === 1 && held[0].row.from?.collection?.outermost !== true)

    function back() {
        if (held.length === 0)
            return;
        // A thing landed in from a search goes back to the search, past the
        // collection it was found in.
        const steps = held[held.length - 1].through && held.length > 1 ? 2 : 1;
        traces.began("back", { from: held[held.length - 1].key ?? "", depth: held.length });
        const left = held[held.length - steps];
        held = held.slice(0, -steps);
        // A step back onto one whose line holds its words puts them back.
        const onto = held.length > 0 ? held[held.length - 1].row : null;
        restart(onto?.words?.() ?? left.typed);
        placeCaret(onto);
        reselect(left);
    }

    // A step whose line holds its words, as a save's name, says where the
    // caret starts in them: before a name's extension, so typing renames
    // it (VKCF3Z's vi-paradigm spec, The File Picker).
    function placeCaret(row) {
        if (typeof row?.words === "function")
            prompt.caretTo(row.caretIn?.(prompt.text) ?? prompt.text.length);
    }

    // Whether the step held holds its words in the line rather than a
    // find: then normal edits them, and its rows aren't narrowed by them.
    // ZZ: the place stood in writes what it has to write, by its own :x, as
    // a save dialog saves; with none, the launcher closes.
    function xit() {
        const own = held.length > 0 ? held[held.length - 1].row.commands?.() ?? [] : [];
        const writes = own.find(one => one.command === "xit");
        if (!writes) {
            open = false;
            return;
        }
        writes.run({ name: "xit", bang: false, arg: "", things: [], count: 0 });
    }

    // How the place stood in says it stands, in the mode word's room in
    // normal, as the file picker's "2 gathered": read again as its rows
    // are, since a place says it changed by asking for them again.
    readonly property string placeSays: {
        void matches;
        return held.length > 0 ? held[held.length - 1].row.standing?.() ?? "" : "";
    }

    readonly property bool holdsWords: held.length > 0
        && typeof held[held.length - 1].row.words === "function"

    // Searches afresh for what the field is set to, since what was found
    // before belongs to another step.
    function restart(text) {
        steadying = false;
        typed = "";
        candidates = [];
        if (prompt.text === text)
            search(text);
        else
            prompt.text = text;
        // Whoever set it reads what it finds next.
        searchNow();
    }

    property string typed: ""
    property var matches: []
    property int current: 0
    property var pending: null

    // The words typed, and every choice they found before the finest were
    // kept: a longer query can only find fewer, so it searches these.
    property var words: []
    property var candidates: []
    // Which joining of the choices the candidates were taken from: a
    // search said again in place looks through them all once they were
    // joined again since, so a thing that came after the words were
    // typed is found, and comes in at the end (Steady.js).
    property int candidatesJoined: 0

    // Whether what's typed at the top finds one of a source's things, by
    // its name or its words, as the search does. Asked only as a source
    // refreshes, never on a key, so typing keeps within its budget.
    function reaches(source) {
        const need = Fuzzy.letters(words);
        const finds = item => (item.mask & need) === need
            && Fuzzy.match(item, words, false).score >= 0;
        return source.things.some(thing => {
            const choice = keyed[thing.key];
            return !!choice && (finds(choice.prepared)
                || !!choice.keywords && finds(choice.keywords));
        });
    }

    // What was picked for what was typed, which the ranking leans on.
    Picks {
        id: picks
        learns: Providers.learning.enable
        halfLife: Providers.learning.halfLife
        remember: Providers.learning.remember
    }

    // What each opening took, to tell the homes Joel tries apart.
    Trials {
        id: trials
    }

    Traces {
        id: traces
    }

    // The list has settled: the spans waiting on it end, with how it's laid.
    function settledNow() {
        traces.settled(laidNow());
    }

    // Where the list lays its rows now, for the spans waiting on it to
    // settle: its first row's place below its top (0 at rest, less once
    // scrolled, null when not drawn), where it's scrolled to and how tall
    // what it holds is.
    function laidNow() {
        const first = list.itemAtIndex(0);
        return { firstAt: first ? Math.round(first.y - list.contentY) : null,
            contentY: Math.round(list.contentY), contentHeight: Math.round(list.contentHeight),
            rows: matches.length, depth: held.length };
    }

    // Every choice the words find (Fuzzy.js says how), in Rank.js's order:
    // what's latched for what's typed, then by tier, then what was picked
    // for it, what's picked most and the better match. One found only with
    // its keywords, beside its name or alone, is in the last tier, but for a
    // collection's own words, which are what it's called.
    // Rows gone since they were shown leave every step once a key searches
    // afresh, so a refresh after it doesn't keep them on as gone again. A
    // row its source draws dimmed stays where it stands: stepping into
    // someone searches afresh, and Write in Slack's dimmed row, there until
    // Slack answers, would otherwise leave and come back at the end.
    function forgetGone() {
        if (held.some(step => step.rows.some(row => row.went)))
            held = held.map(step => Object.assign({}, step, {
                rows: Steady.forgotten(step.rows)
            }));
    }

    function search(letters) {
        const searchBegan = Date.now();
        // Only what's typed is a search of its own; a list said again in
        // place is a refresh. Its letters are counted, never kept.
        if (!steadying) {
            searched += 1;
            forgetGone();
            traces.began("search", { letters: letters.length, depth: held.length });
        }
        const narrowing = typed !== "" && letters.startsWith(typed);
        // Inside a step, the row selected as what's typed is taken away,
        // to stay on.
        const cleared = letters === "" && typed !== "" && held.length > 0 && selected
            ? steadyId(selected) : "";
        typed = letters;
        if (acted && (letters !== acted.typed || held.length !== acted.depth))
            acted = null;
        words = Fuzzy.words(letters);
        // A step whose line holds its words is told each change to them.
        if (holdsWords)
            held[held.length - 1].row.worded?.(letters);
        if (asking) {
            matches = [];
            answer = null;
            current = 0;
            return;
        }
        // A category's word and a space step into it, unless a name goes on
        // from that word, as Power off does from Power: then it's searched
        // for here, and nothing moves under what's being typed.
        const chipped = held.length === 0 ? /^(\S+) $/.exec(letters) : null;
        const named = chipped && !choices.some(row => !row.category
            && String(row.name ?? "").toLowerCase().startsWith(letters))
            ? categoryFor(chipped[1]) : null;
        if (named) {
            intoCategory(named, chipped[1]);
            return;
        }
        const whole = category !== null ? askedWhole(letters) : "";
        if (whole !== "") {
            held = [];
            restart(whole);
            return;
        }
        if (category !== null) {
            // What's typed searches only inside it, its answers first.
            gathered = [];
            root.found = words.length === 0 ? offered.slice() : inCategory(letters);
            compose(!steadying);
            if (!steadying)
                list.positionViewAtBeginning();
            return;
        }
        if (held.length > 0) {
            const from = held[held.length - 1].row.from;
            // A collection searches itself, all of it until something's
            // typed, and then its parts: Agents its agents, then their
            // tasks, helpers and spend. A collection whose vocabulary says
            // its parts lead ("lead": "parts") lists them first, so the
            // Keyboard's keyboards aren't below a hundred keys; it's said,
            // not counted, so no list changes order as it grows. A part
            // named leads what's found.
            let found = [];
            if (from && words.length === 0) {
                const members = from.everything();
                const parts = partsOf(from);
                found = Words.of(from.settings.name).lead === "parts" ? parts.concat(members)
                    : members.concat(parts);
            } else if (from)
                found = within(partsOf(from), letters)
                        .concat(learnedFirst(from.search(letters).rows, letters));
            // A step's rows, all of them in their order until something's
            // typed, then those it finds, best first, and with them the
            // choices of a part that offers them (a display's Scale), so
            // what's typed reaches them without a step in: "scale 150".
            // Found by name or by what it holds, in the top level's tiers.
            // A step that makes rows of what's typed (`typed`), as Move the
            // Meeting makes a time, lists them first. One that finds what's
            // typed past the rows it shows (`finds`), as Capture's Recent
            // finds among everything taken, lists what it finds instead.
            else if (words.length === 0 && !holdsWords)
                found = offered.slice();
            else if (held[held.length - 1].row.finds)
                found = held[held.length - 1].row.finds(letters);
            else
                found = (held[held.length - 1].row.typed?.(letters) ?? [])
                    .concat(within(offered.reduce((all, row) => all.concat([row],
                        row.offers && row.verbs ? row.verbs() : []), []), letters,
                        picks.weights(Rank.stepped(held[held.length - 1].row.key ?? "",
                            letters))));
            found = found.concat(stillActed(found));
            answer = null;
            lined = holdsWords ? held[held.length - 1].row.line?.() ?? null : null;
            // Searched again only to say what's there now, it keeps the
            // order it's shown in, and the selection where it is (reshow).
            if (steadying) {
                matches = live(ordering ? found : steadied(found));
                if (current === -1 && !lined)
                    current = 0;
                return;
            }
            const foundAt = Date.now();
            const lived = live(found);
            const livedAt = Date.now();
            list.keptLast = null;
            matches = lived;
            // What's typed taken away whole leaves the selection on the row
            // it was on, now among them all.
            const kept = cleared ? matches.findIndex(row => steadyId(row) === cleared) : -1;
            current = Math.max(lined ? -1 : 0, kept);
            if (kept >= 0)
                list.positionViewAtIndex(kept, ListView.Contain);
            else
                list.positionViewAtBeginning();
            // Said as the top level's are: finding the place's rows, then
            // showing them, with how many were new to the list and how long
            // laying them out took, so a letter or a step inside says where
            // its time went. Without the laying, Capture's step said only
            // "showing 138" (Joel, 2026-11-11).
            searchParts = { before: 0, lists: foundAt - searchBegan, rows: 0,
                answers: Date.now() - foundAt, each: {}, within: null,
                composing: { asking: 0, placing: livedAt - foundAt,
                    showing: Date.now() - livedAt, kept: list.keptLast } };
            return;
        }
        // What the search spends its time on, said with a key that's slow
        // (Trials.qml): every collection searched, every row matched, then
        // the answers and the list made of them.
        const began = Date.now();
        gathered = gathering(letters);
        const listed = Date.now();
        latch = "";
        foundHere = {};
        // What the rows' time went on, for a first letter that's slow
        // (Joel's own openings put 215 to 271 ms there): matching every
        // choice, ordering what was found, or making the rows shown.
        let costs = null;
        if (words.length === 0) {
            candidates = [];
            root.found = home();
        } else {
            const need = Fuzzy.letters(words);
            const found = [];
            // What the next letter narrows from: everything found, and a
            // collection held back only for being too short yet.
            const early = [];
            // Said again in place, what was found is each row as it is now,
            // or, the choices joined again since it was found, every choice,
            // so what came since is found too.
            const widened = steadying && candidatesJoined !== choicesJoined;
            const looked = !narrowing || widened ? choices : !steadying ? candidates
                : candidates.map(choice => onOffer(choice.key)).filter(choice => !!choice);
            for (const choice of looked) {
                // A part whose whole is all that's named is found in it,
                // and kept for a word past the whole's name.
                if (choice.of && partHidden(choice, words)) {
                    early.push(choice);
                    continue;
                }
                const byName = (choice.prepared.mask & need) === need
                    ? Fuzzy.match(choice.prepared, words, false) : { score: -1 };
                if (byName.score >= 0) {
                    found.push({
                        item: choice.prepared,
                        score: byName.score * choice.weight,
                        grade: byName.grade,
                        tier: Rank.tier(choice.name, letters, false),
                        choice: choice
                    });
                    continue;
                }
                if (!choice.keywords || (choice.keywords.mask & need) !== need)
                    continue;
                const byWord = Fuzzy.match(choice.keywords, words, false);
                // A collection's own words are found by their start from
                // three letters, and below that only whole, so "pi" is pi's
                // sessions and not the Clipboard's pictures.
                const tooShort = choice.named && Fuzzy.plainly(letters).length < 3
                    && !Rank.says(choice.keywords.text ?? "", letters);
                if (byWord.score >= 0 && tooShort)
                    early.push(choice);
                else if (byWord.score >= 0)
                    found.push({
                        item: choice.prepared,
                        score: byWord.score * choice.weight / (choice.named ? 1 : 4),
                        grade: byWord.grade,
                        tier: choice.named ? 1 : Rank.wordTier(choice.keywords.text ?? "", letters),
                        choice: choice,
                        byWord: true
                    });
            }
            candidates = found.map(f => f.choice).concat(early);
            if (looked === choices)
                candidatesJoined = choicesJoined;
            const matchedAll = Date.now();
            const learned = picks.weights(letters);
            const here = {};
            for (const f of found)
                here[f.choice.key] = Rank.leastOf(f.choice);
            for (const row of gathered)
                for (const member of row.verbs())
                    here[member.key] = Rank.leastOf(member);
            foundHere = here;
            latch = picks.latched(letters, Rank.foundNow(here),
                namedAs[Rank.plainly(letters)] ?? []);
            const ordered = Rank.order(found, {
                latched: latch,
                picked: learned,
                often: often
            });
            const orderedAll = Date.now();
            const made = ordered.map(f => Object.assign({
                byWord: f.byWord === true,
                times: (learned[f.choice.key] ?? { times: 0 }).times,
                grade: f.grade,
                tier: f.tier,
                kept: !!learned[f.choice.key]
            }, f.choice));
            const madeAll = Date.now();
            const spots = spotted(letters);
            root.found = made.concat(spots, meantBelow(letters, made.concat(spots, gathered)));
            costs = { matching: matchedAll - listed, ordering: orderedAll - matchedAll,
                making: madeAll - orderedAll, spots: Date.now() - madeAll, looked: looked.length,
                found: found.length };
        }
        const matched = Date.now();
        // Said again in place, as each refresh at home is, the selection
        // stays on its row and the list where it stands. Composed afresh,
        // each refresh put the selection on the first row, sent the list to
        // the top and put both back after, under a glide already running,
        // a hundred times an opening: the highlight went its own way and
        // the list stopped short of its first row (Joel, 2026-11-08).
        compose(!steadying);
        if (!steadying)
            list.positionViewAtBeginning();
        searchParts = { before: began - searchBegan, lists: listed - began, rows: matched - listed,
            answers: Date.now() - matched, each: listsTook, within: costs,
            composing: composeParts };
    }

    // What the last search of what's typed spent its time on, at the top
    // or inside a step, or null when the search said nothing of it.
    property var searchParts: null
    // What each collection took in the last search of them.
    property var listsTook: ({})

    // Times each key from its search to the first frame begun after it,
    // the one its new rows are in: the window says when a frame begins and
    // when it's been swapped onto the screen, and it's listened to only
    // while a key waits for one. A key waits as an act does
    // (launcher/Pacing.js's frameSwapped), so a swap of a frame already
    // under way when its handler ran isn't taken for it: a step from
    // normal, just after Escape, was credited with Escape's frame and said
    // 33 to 51 ms, where its own frame came 117 to 158 ms after the key
    // (keeps-pace under load, 2026-11-10). Each key keeps what it is, when
    // it began, its search and the frame's start (`begun`).
    property var keysDrawing: Pacing.unframed()

    // A key is heard when its handler began, read then on the clock its
    // press is stamped by, rather than worked back from the wall clock,
    // whose whole milliseconds could put it before the press it handles.
    // `pressed` is KeyTimes.pressed() read as the key was heard, since a key
    // searched later, with others typed at once, is timed after KeyTimes has
    // let its press go.
    function keyTimed(typed, began, heard, pressed) {
        keysDrawing = Pacing.refreshedActs(keysDrawing, [{ text: typed, began,
            searched: Date.now() - began, parts: searchParts, begun: null,
            rendered: null, pressed: pressed > 0 ? pressed : heard, heard,
            done: KeyTimes.now() }]);
        keysPressed.push(pressed > 0 ? pressed : null);
        keysHeard.push(heard);
        // Its handler is done once it's searched or moved, all a key's own
        // work, which design.md holds within 4 ms.
        keysDone.push(KeyTimes.now());
    }

    // A frame begins, so it draws every key that waited, and each is told
    // when, on the monotonic clock.
    function keysFrameBegun() {
        const begun = KeyTimes.now();
        for (const key of keysDrawing.waiting) {
            key.begun = begun;
            presentedAwait(key.pressed, at => keyPresented(key.pressed, at, key));
        }
        keysDrawing = Pacing.frameBegun(keysDrawing);
    }

    // The frame those keys are in has been rendered and is being swapped
    // onto the screen, which a check times apart, since a software swap
    // in a test's machine took a median 30 ms a frame under load, where a
    // key took 58 to its frame (keeps-pace, 2026-11-10). The window says
    // so from the render thread, and it's heard here in order before the
    // swap is, so a busy thread can only make the swap seem shorter.
    function keysFrameRendered() {
        const rendered = KeyTimes.now();
        for (const key of keysDrawing.framing)
            key.rendered = rendered;
    }

    // A frame was swapped: the keys it drew are timed by the newest of them.
    function keysFrameSwapped() {
        const swapped = Pacing.frameSwapped(keysDrawing);
        if (swapped.shown.length === 0)
            return;
        keysDrawing = swapped.acts;
        keyDrawn(swapped.shown);
    }

    function keyDrawn(drawnKeys) {
        const drawn = Date.now();
        const shown = KeyTimes.now();
        const newest = drawnKeys[drawnKeys.length - 1];
        // Its time after its own work said apart, as a drawer's is: waiting
        // for the frame that drew it, then drawing it, so a key whose frame
        // came 80 ms after a 30 ms search says which held it (Joel's "ag",
        // 2026-11-11).
        trials.timed(newest.searched, drawn - newest.began, newest.parts, newest.text, {
            waiting: Math.round(newest.begun - newest.done),
            drawing: Math.round(shown - newest.begun)
        });
        // Every key the frame drew keeps it as its own, and the list holds
        // one time a key.
        for (const key of drawnKeys) {
            keysFramed.push(shown);
            keysBegun.push(key.begun);
            keysRendered.push(key.rendered);
        }
        keysTimed += drawnKeys.length;
        // The oldest go from all six at once, so each place still holds
        // one key though keys typed as this frame was drawn wait on.
        const over = keysFramed.length - keysKept;
        if (over > 0) {
            for (const kept of [keysFramed, keysBegun, keysRendered, keysHeard, keysDone,
                keysPressed])
                kept.splice(0, over);
        }
    }

    // What waits for the launcher's next frame, as a daemon's watch begun
    // by a step does (Daemon.qml's paced): run once a frame begun after it
    // is drawn, so a swap already under way isn't taken for it, or at once
    // while the launcher is closed, as no frame is coming. A frame is asked
    // for, so one comes though nothing else changed. Each waits in
    // afterFrames until a frame begins, then in afterFramesDrawing until
    // that frame is swapped, so one queued while a frame is drawn waits for
    // the next rather than running on a swap that began before it.
    property var afterFrames: []
    property var afterFramesDrawing: []
    function afterFrame(then) {
        if (!open && !opening) {
            then();
            return;
        }
        afterFrames = afterFrames.concat([then]);
        card.Window.window?.update();
    }
    function afterFramesDue() {
        const due = afterFramesDrawing.concat(afterFrames);
        afterFramesDrawing = [];
        afterFrames = [];
        for (const then of due)
            then();
    }

    // When each of the last keys reached its frame, oldest first, for the
    // check that times each key from the keyboard to the screen under load,
    // and for the machine's own load proof. Every key is kept, however
    // quick, since a time is only a time beside the key that made it, and
    // with how many have been timed in all, so a key is found by its count
    // however many the list has let go. Nothing binds to them, so they're
    // changed in place.
    readonly property int keysKept: 300
    property var keysFramed: []
    // And when the frame each was drawn in began, after its handler was
    // done, so a time is seen to be its own frame's, and when its render
    // ended and its swap began, null where that wasn't heard.
    property var keysBegun: []
    property var keysRendered: []
    // When each of those keys was pressed, as the compositor says, and
    // when the shell's own handler took it, all on the monotonic clock the
    // compositor's times are on, so the machine's load proof can time
    // Joel's own keys with no keyboard of its own: the press counts the
    // kernel's, niri's and a busy main thread's part, the handler only the
    // shell's. A key no press made, as text set when the launcher opens,
    // has no press. A key is kept in both once heard, so they run ahead of
    // the frames by the keys still waiting for one.
    property var keysPressed: []
    property var keysHeard: []
    property var keysDone: []
    property int keysTimed: 0

    // The last f or F, which ; and , go again (Finding.js).
    property var lastLetter: null

    // n and N: the find's next row or the one before, `forward` its way,
    // round past an end and saying so, as Vim's wrapscan does; with nothing
    // typed, or nothing found, Vim's E35 or E486, the rows staying put.
    function toMatch(forward, count, key) {
        const went = Finding.matchTo({ at: current, count, forward, last: matches.length - 1,
            typed });
        if (!went.none)
            arrowed(went.to, key);
        saySought(went, typed);
    }

    // What a search says in the foot until the next key, the rows' n and N
    // and the drawer's search alike: E35 or E486 where it went nowhere,
    // `sought` named in E486, or the end it went round, as Vim's do.
    function saySought(went, sought) {
        const hints = Words.field.hints ?? {};
        if (went.none)
            say([[(hints[went.none] ?? "").replace("{typed}", sought), ""]]);
        else if (went.wrapped !== "")
            say([[went.wrapped === "bottom" ? hints.wrappedBottom : hints.wrappedTop, ""]]);
    }

    // f and F to the next row and the one before whose name begins with
    // `letter`, and ; and , again, the same way and the other; with no such
    // row, or no f before ;, it says so and the rows stay put.
    function toLetter(act, count, letter) {
        const hints = Words.field.hints ?? {};
        const sought = Finding.again(lastLetter, act, letter);
        if (sought === null) {
            say([[hints.noAgain ?? "", ""]]);
            return;
        }
        if (act === "toLetter" || act === "toLetterBack")
            lastLetter = sought;
        const to = Finding.letterTo({ at: current, count, sought,
            names: matches.map(row => row?.name ?? "") });
        if (to < 0) {
            say([[(hints.noLetter ?? "").replace("{way}", sought.forward ? hints.below : hints.above)
                .replace("{letter}", sought.letter), ""]]);
            return;
        }
        arrowed(to, QueryKeys.keyOf(act, "normal"));
    }

    // Moves the selection as an arrow does, timing the key to its frame as
    // a typed one is. A key that moves nothing draws nothing, so it isn't
    // timed: its frame would be whatever came next.
    function arrowed(to, arrow) {
        trials.pressed("arrows");
        if (to === current)
            return;
        const began = Date.now();
        const heard = KeyTimes.now();
        const pressed = KeyTimes.pressed();
        searchParts = null;
        // What the glide it sets going is timed from, as the key is.
        movingFrom = pressed > 0 ? pressed : heard;
        current = to;
        movingFrom = 0;
        keyTimed(arrow, began, heard, pressed);
    }
    property real movingFrom: 0

    // Vim's operators over the rows (QueryKeys.js reads them, Operators.js
    // says what each does to a row): d does each row's ending action, as
    // dd deletes a line, and y copies each, as yy yanks one. What d does
    // that can't be put back it asks about once, as Vim's ask_yesno asks
    // (Confirm.js): the launcher's one ask, said whole on the line in the
    // find's place (Prompt.qml's asking), as the file picker's "Replace
    // notes.md? (y/n)" is, one place on every surface (VKCF3Z,
    // 2026-11-05). While it stands every key is its answer (Prompt.qml's
    // confirming): after its first 300 ms, y does it, n, Escape or Ctrl+C
    // lets it go, and any other key does nothing.
    property var asked: null
    // The rows it covers, by their places, which wear visual's tint while
    // it stands, so every one it would end shows as included.
    property var askedOver: []

    // Asks once, `yes` done on y and `no` on n, Escape or Ctrl+C, or as the
    // launcher closes, `covering` the places of the rows it covers, or none.
    function askOnce(question, yes, no, covering) {
        asked = Confirm.asked(question, yes, no ?? (() => {}), Date.now());
        askedOver = covering ?? [];
        askTaking = false;
        askReady.restart();
        Report.say(`asked ${asked.question}`);
    }

    // A key while the ask stands, as Prompt.qml hands it over.
    function answerAsk(key) {
        const ask = asked;
        if (ask === null)
            return;
        const said = Confirm.answer(ask, key, Date.now(), askReady.interval);
        if (said === "early" || said === "nothing")
            return;
        asked = null;
        Report.say(`answered ${said}`);
        if (said === "yes")
            ask.yes();
        else
            ask.no();
    }

    // When the ask can be answered, the same 300 ms as the key language's
    // wait for a second key, said for the tests, which wait on it.
    Timer {
        id: askReady
        interval: Options.keyTiming.wait
        onTriggered: {
            root.askTaking = root.asked !== null;
            if (root.asked !== null)
                Report.say("ask ready");
        }
    }
    // Whether the ask standing takes an answer yet, once askReady has run:
    // state rather than a binding on the clock, which no binding sees
    // pass, so what the tests are shown turns as the ask does.
    property bool askTaking: false
    onAskedChanged: {
        if (asked === null)
            askTaking = false;
    }
    // An operator's word for the key it was said for, as a search's is:
    // [mark, word, colour], for d over nothing to end or y over nothing to
    // copy.
    property var operatorSaid: null
    property int operatorSaidFor: -1
    // The rows y just copied, by their place, flashed for the key timing's
    // yank-flash as his Neovim's on_yank flashes a yank.
    property var flashing: ({})

    Timer {
        id: flash
        interval: Options.keyTiming["yank-flash"]
        onTriggered: root.flashing = {}
    }

    function operatorSays(word) {
        operatorSaid = [Theme.glyph.nothingFound, word ?? "", Theme.warning];
        operatorSaidFor = prompt.keysTaken;
    }

    // What's done, and how to take it back, in the success colour until the
    // next key, as the spec's "3 done \u00b7 u brings them back".
    function operatorTells(word) {
        operatorSaid = [Theme.glyph.done, word, Theme.success];
        operatorSaidFor = prompt.keysTaken;
    }

    // The changes d made that u can take back and Ctrl+R do again, each the
    // undoable actions of one d (Operators.undoable), for as long as the
    // launcher stays open, as the spec has every surface keep them; and
    // the last d, its count and motion, for . to do again.
    property var rowChanges: Changes.changes()
    property var lastChange: null

    // Keeps what u can take back of the actions just run, by d or by a
    // command line, which asks for none of them since u can; how many.
    function changedBy(actions) {
        const undoable = Operators.undoable(actions);
        if (undoable.length > 0)
            rowChanges = Changes.changed(rowChanges, undoable);
        changedRow();
        return undoable.length;
    }

    // The cursor row's actions read again once a change has run, since a
    // change can leave the row standing with other actions, as a notice
    // forgotten until the launcher closes has no d left; read only once
    // selected, its foot would go on naming the d that's done.
    function changedRow() {
        readSelectedActions();
        refreshShown();
    }

    // What an act changed, shown on the frame after its key: every source
    // what's shown is made from is made again at once, whether or not it
    // says it changed, since a source that never says so left d's change
    // unseen until another made the list again (Joel, 2026-11-07: "doing a
    // dd on ntofications does not update the screen wright away"). Each
    // place held and what holds it are read, as refreshNow reads them, and
    // out at the top what a search ended with; a source that changed
    // nothing draws nothing. Every other row out at the top is its
    // source's own, which says when it changes.
    function refreshShown() {
        const first = held.length > 0 ? held[0].row : null;
        const fromTop = first !== null && !first.from && !first.category;
        const shows = source => held.some(step => step.row.from === source
                || step.row.endedBy === source
                || !!step.row.category && holdsFrom(step.row.category, source))
            || fromTop && source.things.some(own => own.key === first.key)
            || first === null && matches.some(row => row.endedBy === source);
        for (const source of sources.filter(shows))
            refreshed(source);
    }

    // u: the last change taken back, or as many as counted, each action put
    // back by its own undo; with none, Vim's word for it.
    function undoChange(count) {
        const words = Words.field.operators ?? {};
        const back = Changes.undo(rowChanges, count);
        if (back.moved === null) {
            operatorSays(words.oldest);
            return;
        }
        rowChanges = back.history;
        const entries = [].concat(...back.moved);
        for (const entry of entries)
            entry.undo();
        changedRow();
        operatorTells((words.undone ?? "").replace("{count}", String(entries.length)));
        Report.say(`undid ${entries.length}`);
    }

    // Ctrl+R: what u took back done again, as many changes as counted.
    function redoChange(count) {
        const words = Words.field.operators ?? {};
        const again = Changes.redo(rowChanges, count);
        if (again.moved === null) {
            operatorSays(words.newest);
            return;
        }
        rowChanges = again.history;
        const entries = [].concat(...again.moved);
        for (const entry of entries)
            entry.redo();
        changedRow();
        operatorTells((words.redone ?? "").replace("{count}", String(entries.length)));
        Report.say(`redid ${entries.length}`);
    }

    // .: the last d again from the cursor, as Vim's . repeats a change, with
    // the count typed before it in its own's place; with none yet, nothing.
    function doAgain(count) {
        if (lastChange === null)
            return;
        const next = Operators.again(lastChange, count);
        operated("ending", next.count, next.motion);
    }

    // What a pending d or y will take, for the mode word's room, said at
    // once so d is never pressed blind: the keys typed, then the cursor
    // row's ending verb or copy and its name, or the count's rows ("d \u00b7
    // disconnect Home-5G", "3d \u00b7 close 3"); "" with none waiting. Only
    // the one row's actions are read, and only while an operator waits.
    readonly property string pendingSaid: {
        const hit = Operators.pending(prompt.standing);
        const row = current >= 0 ? matches[current] : headline;
        if (hit === null || !row)
            return "";
        const words = Words.field.operators ?? {};
        const act = Operators.operators[prompt.standing.operator.key];
        const actions = actionsOf(row);
        const copy = act === "copy"
            && Operators.copyOf(row, actions, one => verbOf(one) === "copy" && !!one.run);
        const ending = act === "ending" ? Operators.endingOf(row, actions) : null;
        const verb = copy ? words.copies ?? ""
            : ending !== null ? ending.ends || (words.ends ?? "") : null;
        return Operators.pendingSays(Object.assign(hit, { act, verb, name: row.name ?? "" }),
            words);
    }

    // A row's actions that d and y may take: its operands, where it says
    // them, plain fields a place of hundreds makes in well under the 4 ms
    // a key has; else its actions as the right arrow lists them, each a
    // row made searchable: nine such rows a row, as a window has, took
    // 18 ms over 400 rows when timed alone (2026-11-05).
    // The selected row's actions, read as it's selected rather than in the
    // foot's binding: a row's verbs() may ask its source, as Time's watch
    // chrony, and a binding that read them would loop on what they set.
    // Only in normal and visual, where the foot names d and y, and never a row whose
    // verbs are the things inside it (nested), since reading those asks
    // for them: Search Slack's asked Slack with each letter typed, which
    // Slack allows only a few times a minute.
    property var selectedActions: ({ row: null, actions: [] })
    function readSelectedActions() {
        const reads = selected && prompt.modal && ["normal", "visual"].includes(prompt.mode)
            && !selected.nested;
        selectedActions = { row: selected, actions: reads ? actionsOf(selected) : [] };
    }
    // d and y as they stand, worked out again as what they're read from
    // changes, as an answer's parts arriving give it y; and the foot's
    // own, told anew only where it would say something new
    // (Standing.toldAnew), since all it reads them for runs again each time
    // it's told. Else the row they're for changes unsaid: with no keys
    // before or now, no row's keys change.
    readonly property var selectedKeys: selectedActions.row === null ? []
        : operatorKeysOf(selectedActions.row, selectedActions.actions)
    property var footActions: ({ row: null, keys: [] })
    onSelectedKeysChanged: {
        if (Standing.toldAnew(footActions, selectedActions.row, selectedKeys))
            footActions = { row: selectedActions.row, keys: selectedKeys };
        else
            footActions.row = selectedActions.row;
    }

    function actionsOf(row) {
        if (typeof row.operands === "function")
            return row.operands();
        return typeof row.verbs === "function" ? row.verbs() ?? [] : [];
    }

    // d or y (Prompt.qml's operated), over the rows from the selection to
    // where the motion goes, or as many as the count says.
    // Visual line's selection: the row it began on, -1 outside it, and the
    // first and last rows it takes, as places in the rows. It begins on the
    // cursor row as V is pressed, and outlives the command line its colon
    // opens, which gives it back on Escape.
    property int visualFrom: -1
    readonly property bool visual: prompt.mode === "visual"
    readonly property var visualSpan: Operators.selection(visualFrom, current)
    readonly property int visualTop: visualSpan.from
    readonly property int visualBottom: visualSpan.to
    // What the row it began on is, to find it again as the rows change.
    property string visualId: ""
    onVisualFromChanged: visualId = visualFrom >= 0 && matches[visualFrom]
        ? Rank.identity(matches[visualFrom]) : ""

    // The last selection, where it began and where the cursor was, each
    // held by its row, for gv to bring back as Vim's '< and '> keep it;
    // null until there's been one, and again once the launcher closes.
    property var lastVisual: null
    onVisualChanged: {
        if (!visual && visualFrom >= 0)
            lastVisual = Operators.marked(matches, visualFrom, current, Rank.identity);
        visualFrom = !visual ? -1 : visualFrom >= 0 ? visualFrom : current;
    }

    // The run of rows one is in, a paragraph to ip, ap, { and }: rows of
    // one kind, or a notice's of one sender, no gap between (Operators.js).
    function runOf(at) {
        return Operators.runAt(matches, at, row => {
            const kind = kindOf(row);
            return kind === "notice" ? `notice:${row.sender ?? ""}` : kind;
        });
    }

    // Visual's own acts on the rows: o to the selection's other end; ip and
    // ap over the run the cursor's in, and count - 1 after it; and gv, the
    // last selection's rows again wherever they've moved, as Vim's gv
    // keeps its lines.
    function visualStep(act, count) {
        const last = matches.length - 1;
        if (act === "otherEnd") {
            const other = visualFrom;
            visualFrom = current;
            arrowed(other, "o");
        } else if (act === "reselect" && lastVisual !== null && last >= 0) {
            const back = Operators.reselected(matches, lastVisual, Rank.identity);
            visualFrom = back.from;
            arrowed(back.at, "gv");
        } else if (act === "inParagraph" || act === "aParagraph") {
            const run = Operators.spanned(0, { act, count }, {
                at: current, highest: prompt.highest, last, runOf: at => runOf(at)
            });
            visualFrom = run.from;
            arrowed(run.to, act === "inParagraph" ? "ip" : "ap");
        }
    }

    // The rows the selection takes.
    function visualRows() {
        return matches.slice(Math.max(0, visualTop), visualBottom + 1);
    }

    // Whether a row wears the selection's ground: visual's, or what a
    // pending operator will take.
    function tints(at) {
        if (asked !== null) {
            return askedOver.includes(at);
        }
        return visualFrom >= 0 ? at >= visualTop && at <= visualBottom
            : at === current && pendingSaid !== "";
    }

    // Return over the selection: the place stepped into takes the rows
    // itself where it says so (takes(rows) true, as the file picker's
    // marks them); else every row Return takes somewhere is opened, never
    // one that changes something, and what couldn't be is said once
    // ("2 of 3 open"), the cursor left on the first row taken.
    function ranOver(taken) {
        const rows = taken.map(one => one.row);
        const place = held.length > 0 ? held[held.length - 1].row : null;
        if (place && typeof place.takes === "function" && place.takes(rows) === true)
            return;
        const opening = rows.filter(row => Catalogue.opens(row, commandNamed(row)));
        const said = Operators.openedSaid(opening.length, rows.length,
            Words.field.visual ?? {});
        if (said !== "")
            operatorSays(said);
        Report.say(`opened ${opening.length} of ${rows.length}`);
        for (const row of opening)
            run(row, false);
    }

    function operated(act, count, motion) {
        searchNow();
        if (motion?.act === "visual") {
            const taken = Operators.taken(matches, visualSpan, headline);
            // Visual's d is done again by . over as many rows as it took,
            // from the cursor, as Vim's is.
            if (act === "ending")
                lastChange = { count: visualSpan.to - visualSpan.from + 1, motion: null };
            if (act === "copy")
                yanked(taken);
            else if (act === "ending")
                ended(taken);
            else
                ranOver(taken);
            return;
        }
        const view = list.inView();
        const span = Operators.spanned(count, motion, {
            at: current,
            highest: prompt.highest,
            last: matches.length - 1,
            top: view.top,
            bottom: view.bottom,
            screen: rows,
            runOf: at => runOf(at)
        });
        const taken = Operators.taken(matches, span, headline);
        if (act === "copy")
            yanked(taken);
        else {
            lastChange = { count, motion };
            ended(taken);
        }
    }

    // y: every copy over the rows joined a line each and put on the
    // clipboard at once, which takes text while the launcher holds the
    // keys, so the launcher stays as Vim's buffer does. A single copy that
    // says no text, as a picture, is done by its own act.
    function yanked(taken) {
        const words = Words.field.operators ?? {};
        const copies = Operators.copiesIn(taken, actionsOf,
            row => verbOf(row) === "copy" && !!row.run);
        const text = Operators.copied(copies.map(copy => copy.action));
        if (text !== "")
            Quickshell.clipboardText = text;
        else if (copies.length === 1)
            copies[0].action.run();
        else {
            operatorSays(words.noCopy);
            return;
        }
        const flashed = {};
        for (const copy of copies)
            flashed[copy.at] = true;
        flashing = flashed;
        flash.restart();
        Report.say(`yanked ${copies.length}`);
    }

    // d: every ending action over the rows, asked about once where any
    // can't be put back, the rows with none passed over.
    function ended(taken) {
        const words = Words.field.operators ?? {};
        const endings = Operators.endings(taken, actionsOf);
        if (endings.length === 0) {
            operatorSays(words.nothing);
            return;
        }
        const actions = endings.map(ending => ending.action);
        // What can be put back is kept for u; where all of it can, the
        // foot says so until the next key.
        const doing = () => {
            for (const action of actions)
                Operators.doing(action)();
            if (changedBy(actions) === actions.length)
                operatorTells(Operators.doneSaid(actions.length, words));
            Report.say(`ended ${actions.length}`);
        };
        if (!Operators.asks(actions, Options.commands ?? {})) {
            doing();
            return;
        }
        const rowAt = at => at === -1 ? headline : matches[at];
        const named = endings.map(ending => ({ ends: ending.action.ends,
            asked: askedOf(ending.action),
            name: ending.action.askedAs || (rowAt(ending.at)?.name ?? ""),
            kind: kindOf(rowAt(ending.at)),
            leaves: ending.action.leaves ?? "" }));
        askOnce(Confirm.questionParts(named, words), doing, null,
            endings.map(ending => ending.at));
    }

    // Text typed faster than it's searched, as a program types it, reaches
    // the field in one turn of the event loop: wtype's eight letters came in
    // one (2026-11-02). Searched once a letter, a burst's last letter waited
    // on every search before it (5FWIUS's finding [a0802572]). So each
    // change is kept with when it was heard and pressed. The first in a turn
    // is searched at once, as a key Joel presses always is, so it waits on
    // nothing more than before; any that follow it in the same turn are
    // searched once, for the newest text, as the turn ends or as soon as a
    // key reads what was found, whichever is sooner. Each key is still
    // timed on its own.
    property var keysDue: []
    property bool searchedThisTurn: false
    function searchSoon(text, began, heard, pressed) {
        keysDue = keysDue.concat([{ text, began, heard, pressed }]);
        if (searchedThisTurn)
            return;
        searchedThisTurn = true;
        Qt.callLater(turnEnded);
        searchNow();
    }
    function turnEnded() {
        searchedThisTurn = false;
        searchNow();
    }
    // What every key and setter that reads what was found runs first, so it
    // acts on what's typed now rather than on the rows of a letter before.
    function searchNow() {
        if (keysDue.length === 0)
            return;
        const due = keysDue;
        keysDue = [];
        searchParts = null;
        search(prompt.text);
        if (!asking)
            due.forEach(key => keyTimed(key.text, key.began, key.heard, key.pressed));
    }
    // How many searches of what's typed have run, refreshes aside, for the
    // check that a burst is searched once.
    property int searched: 0

    onCurrentChanged: {
        list.pick(list.settled);
        if (current >= 0)
            list.reveal(current);
    }

    // The rest of the selected name from where what's typed starts a word in
    // it, so the command line can finish the thought.
    readonly property string completion: {
        // An answer that does something says what, rather than equalling it.
        if (current === -1 && answer) {
            const rest = answer.question ? answer.question.slice(Fuzzy.plainly(typed).length) : "";
            return rest + (answer.run ? ` \u2192 ${answer.headline}` : ` = ${answer.headline}`);
        }
        const choice = matches[current];
        if (!choice || typed === "")
            return "";
        // An answer's row says the answer; the field finishes its question.
        if (choice.answered)
            return answer && answer.question
                ? answer.question.slice(Fuzzy.plainly(typed).length) : "";
        const name = choice.name;
        const at = name.toLowerCase().indexOf(typed.toLowerCase());
        if (at < 0 || (at > 0 && name[at - 1] !== " "))
            return "";
        return name.slice(at + typed.length);
    }

    // Which of a name's letters what's typed matched, in order: none for a
    // row found by another word, or for an answer, named by what it says
    // rather than by what was typed. A collection, a category or a part
    // said plainly is still found by its name, so its letters are washed
    // as any row's are. They're matched in the name drawn, which a place
    // asked by name can say otherwise than the row it stands on. Worked
    // out only for the rows drawn, as each is drawn or what's typed
    // changes, and kept by nothing but the row drawing it, so a list of
    // matches holds no more.
    function matchedIn(choice) {
        if (choice.byWord || choice.answered)
            return [];
        const drawn = choice.prepared?.text === choice.name ? choice.prepared
            : Fuzzy.prepare(choice.name);
        return Fuzzy.match(drawn, words, true).positions ?? [];
    }

    // A name with the letters that matched in Search's ink, as styled
    // text, their ground drawn behind them (Wash.qml).
    function marked(choice, at) {
        const escape = text => text.replace(/&/g, "&amp;").replace(/</g, "&lt;");
        let out = "";
        for (let i = 0; i < choice.name.length; i++) {
            const letter = escape(choice.name[i]);
            out += at.includes(i)
                ? `<font color="${Theme.wash.ink}">${letter}</font>`
                : letter;
        }
        // A command's form, what's typed after its start, is muted, so
        // what to type reads apart from what to put there.
        if ((choice.form ?? "") !== "")
            out = out.replace(/\u27e8[^\u27e9]*\u27e9/g, said => `<font color="${Theme.muted}">${said}</font>`);
        return out;
    }

    // Acting on a thing says it was what was meant by what was typed; in a
    // collection or a category, the one picked from it was. Learned before
    // the launcher closes, while what's typed and held is still there.
    function learn(choice) {
        trials.picked(choice, typed, held.length);
        const first = held.length > 0 ? held[0] : { row: choice, typed: typed };
        const picking = !!first.row.from || !!first.row.category;
        const meant = picking ? choice : first.row;
        // One that can't be taken back is latched only once its first word's
        // been typed (Rank.js's leastOf), so a slip of two letters never
        // powers off.
        picks.picked(picking ? typed : first.typed, meant.key, knowing());
        // Inside a thing's own page, what was picked is learned for what was
        // typed there too, apart from the top, so the same letters land on
        // it there next time (Rank.js's stepped). A collection or category
        // held alone has just learned it under what's typed, which it reads
        // (inCategory); one stepped into past it learns it here too, so a
        // page inside Machine and Machine reached from a page both find it.
        const step = held.length > 0 ? held[held.length - 1].row : null;
        if (!(picking && held.length === 1) && step && step !== choice && step.key)
            picks.picked(Rank.stepped(step.key, typed), choice.key, null);
    }

    function run(choice, below) {
        if (!choice) {
            open = false;
            return;
        }
        // Its thing has gone, so there's nothing left to do it to.
        if (choice.gone)
            return;
        traces.began("act", { key: choice.key ?? "", depth: held.length });
        // A command puts its start in the field, to be finished there.
        if (choice.start !== undefined) {
            fill(choice.start);
            return;
        }
        // A row whose Return does one of its acts (returnOf) does it: one
        // asking for words asks only now, and the rest close the launcher,
        // the row learned as what was meant, never its act.
        if (choice.returns) {
            ranReturn(choice, choice.returns);
            return;
        }
        // A category, or a row holding a provider's things, steps in.
        if ((choice.category || choice.group) && enter(choice))
            return;
        // A verb that needs a target asks for it, and one that needs words
        // asks for them. A row with an act of its own besides stepping in
        // does it, as a meeting's row joins it (Now.qml), and the
        // right arrow still finds what it holds.
        if (held.length > 0 && choice.targets && (!choice.run || choice.verb === "more")) {
            enter(choice);
            return;
        }
        if (choice.given) {
            ask(choice);
            return;
        }
        // One that would only copy what it says steps into what it holds.
        if (copiesOnly(choice) && enter(choice))
            return;
        // One that can't be taken back, such as powering off, steps into
        // its verbs, which say what it does, so a second Return does it and
        // Escape or the left arrow backs out.
        if (choice.confirm && (held.length === 0 || collecting) && enter(choice))
            return;
        // One with nothing to do of its own, only more inside, steps in.
        if (!choice.run && enter(choice))
            return;
        // One that changes something, or whose outcome takes a while, keeps
        // the launcher, its row saying how it stands now; the right arrow
        // then Return does it and closes (stepRows).
        if (choice.stays) {
            if (keepsOpen(choice)) {
                learn(choice);
                acted = { key: choice.key, typed: typed, depth: held.length };
            }
            choice.run();
            refreshShown();
            return;
        }
        // One that opens a collection steps into it, the launcher kept open.
        if (choice.opens) {
            picks.picked(typed, choice.key, knowing());
            trials.picked(choice, typed, held.length);
            into(choice.opens);
            return;
        }
        // The clipboard takes text only from a program holding the keys,
        // so one that acts at once acts before the keys go. They go back
        // to the window in a frame of their own, before the closing's work,
        // which took a paste's keys 91 ms on Joel's machine (2026-11-10):
        // the clipboard's history presses the paste keys once niri says
        // the launcher let go (clipboard/src/main.rs).
        if (choice.now) {
            choice.run();
            lettingGo = true;
            Report.say("let go of the keys");
            afterFrame(() => closedAfter(choice));
            return;
        }
        learn(choice);
        // Said as the foot's other words are, beside the key they follow.
        if (verbOf(choice) === "copy")
            leaveSaying("copied");
        open = false;
        if (below && choice.below)
            choice.below();
        else if (choice.later)
            runOnceGone(choice, true);
        else
            choice.run();
    }

    // A row that acted at once learned, said and closed for, once the frame
    // giving the keys back has been drawn.
    function closedAfter(choice) {
        learn(choice);
        if (verbOf(choice) === "copy")
            leaveSaying("copied");
        open = false;
    }

    function ranReturn(row, act) {
        if (act.given) {
            ask(act);
            return;
        }
        learn(row);
        // One that changes something where Joel is, as a session's Wake does,
        // keeps the launcher, the row saying how it stands now, as it would
        // from its step in.
        if (act.stays) {
            act.run();
            return;
        }
        open = false;
        if (act.later)
            runOnceGone(act, true);
        else
            act.run();
    }

    // Runs a row that waits for the launcher to be gone, as a screenshot
    // does, once its surface has drawn itself closed and niri has had a
    // moment to show it so. A fixed wait alone let a loaded machine take
    // the screenshot before the closed frame was drawn, the launcher in it
    // (2026-11-03: drawn 1.9 s after it closed). One closed already draws
    // nothing, so it waits that moment alone, and a frame that never comes
    // is waited for a second at most.
    function runOnceGone(choice, shown) {
        pending = choice;
        closedFrame = false;
        if (shown)
            undrawn.restart();
        else
            later.restart();
    }

    // Whether a frame has begun since the launcher closed. A frame begun
    // before may still be swapped after, the launcher drawn in it, so only
    // a swap once one has begun is the closed frame's.
    property bool closedFrame: false

    Connections {
        target: card.Window.window
        enabled: undrawn.running

        function onAfterAnimating() {
            root.closedFrame = true;
        }

        function onFrameSwapped() {
            if (!root.closedFrame)
                return;
            undrawn.stop();
            later.restart();
        }
    }

    Timer {
        id: undrawn
        interval: 1000
        onTriggered: later.restart()
    }

    Timer {
        id: later
        interval: Theme.motion.short
        onTriggered: {
            if (root.pending === null)
                return;
            root.pending.run();
            root.pending = null;
        }
    }

    // A copy closes with nothing else to show it worked, so the card
    // stays drawn a moment past closing, its foot saying "copied" where
    // Return's key was, then leaves as anything leaving does (Joel,
    // 2026-10-26: "copied" as it closes). The keys go back to the window
    // at once, so nothing waits on it.
    property string parting: ""
    property bool fading: false

    // Whether the card is seen: open, or leaving with a parting word. Qt
    // draws a window for any change inside it, clear or hidden alike, so
    // what the card reads live follows only while it's seen, and catches up
    // as it opens. Closed, the foot's facts changing (the time, the lag,
    // the volume) drew the launcher a frame each that nobody saw
    // (2026-11-01).
    readonly property bool cardSeen: cardUp || parting !== ""

    // The steps the last opening took, let go once its card is gone, so
    // the next opening finds home already there: Joel's opening after one
    // closed two steps deep took 224 ms forgetting them, against 109 to
    // 126 after one closed at home (Y2YBTS, 2026-11-10). Each is written
    // only when it holds something, since every write of held sets off
    // every binding that reads it, the same steps or not.
    function leaveSteps() {
        if (held.length > 0)
            held = [];
        if (jumpsBack.length > 0)
            jumpsBack = [];
        if (jumpsAhead.length > 0)
            jumpsAhead = [];
        if (tagsBack.length > 0)
            forgetFollows();
        prompt.forget();
    }
    // Long enough for the closing frame to be drawn first, so letting the
    // steps go never holds the card on screen.
    Timer {
        id: leavingLater
        interval: 250
        onTriggered: root.leaveSteps()
    }
    onCardSeenChanged: {
        if (!cardSeen)
            leavingLater.restart();
    }

    // What the closed card held, brought up to now as it opens. The
    // bindings holding it follow cardSeen only once the opening's handler
    // is done, and that handler reports what the foot says.
    function footCatchUp() {
        standing.facts = standing.factsNow;
        foot.said = foot.saidNow.footer;
        foot.saidStands = foot.saidNow.stands;
        foot.saidNote = foot.saidNow.note;
        foot.told = foot.toldNow;
    }
    function leaveSaying(word) {
        parting = word;
        fading = false;
        partingLater.restart();
        Report.say(`parted ${word}`);
    }

    // Long enough to read one word, short of where it would linger.
    Timer {
        id: partingLater
        interval: 600
        onTriggered: {
            root.fading = true;
            root.parting = "";
        }
    }

    // What a key says down the socket, a line each: "toggle", or
    // "collection" and a collection's name. Each connection is closed from
    // this end once its line is read, so launcher-say returns at once and
    // Quickshell has no peer's hang-up to warn of.
    SocketServer {
        active: true
        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/launcher.sock`
        handler: Socket {
            id: caller

            parser: SplitParser {
                onRead: line => {
                    caller.connected = false;
                    root.askedToOpen();
                    const said = line.trim().split(/\s+/);
                    // launcher-say ends its line with when the chord ran it
                    // (chordAt), which a line from anything else may leave out.
                    const chord = /^@[0-9.]+$/.test(said[said.length - 1] ?? "") ? said.pop()
                        : "";
                    root.chordHeard(chord);
                    const words = said;
                    if (words[0] === "toggle")
                        root.toggled();
                    else if (words[0] === "collection")
                        root.pressed(words.slice(1).join(" "));
                    else if (words[0] === "command-line" && words.length >= 3)
                        root.viewerCommandLine(words[1], words[2], words.slice(3).join(" "));
                    else
                        console.warn(`The launcher was told "${line}", which it doesn't know.`);
                    root.askDone();
                }
            }
        }
    }

    IpcHandler {
        target: "launcher"

        function toggle(): void {
            root.askedToOpen();
            root.toggled();
            root.askDone();
        }

        // Says a program outside the shell, asked from the launcher, may put
        // a window up for Joel: its app ids, space apart, and how many ms
        // it may take (handing).
        function handsOver(apps: string, patience: int): void {
            root.handsOver(apps.split(" ").filter(app => app !== ""), patience);
        }

        // Opens the launcher into a collection by its provider's name.
        function collection(name: string): void {
            root.askedToOpen();
            root.chordInto(name);
            root.askDone();
        }

        // Each of the last openings, from its ask to its first frame, as
        // JSON (openingsTimed).
        function openings(): string {
            return JSON.stringify(root.openingsTimed);
        }

        // Each key presented since it opened, from its press and when
        // niri presented it, on the monotonic clock, as JSON.
        function keysPresented(): string {
            return JSON.stringify(root.keysPresented);
        }

        // Each key presented where its glide came to rest, as keysPresented
        // says them, as JSON.
        function keysLanded(): string {
            return JSON.stringify(root.keysLanded);
        }

        // Each daemon change shown since it opened, by its place, from its
        // line and when niri presented it, on the monotonic clock, as JSON.
        function updatesPresented(): string {
            return JSON.stringify(root.updatesPresented);
        }

        // The command line over a viewer, by its name, where it is and the
        // name of what it shows.
        function commandLine(viewer: string, where: string, name: string): void {
            root.viewerCommandLine(viewer, where, name);
        }

        function showing(): string {
            return root.showing();
        }

        // The catalogue, every place and action by id, as one line of JSON.
        function list(): string {
            return JSON.stringify(root.catalogue());
        }

        // Does what an id names, or shows it ready when it ends something,
        // saying what it did as JSON.
        function run(id: string): string {
            return JSON.stringify(root.catalogueRun(id, ""));
        }

        // The same, for an action that takes words, such as Talk.
        function runWith(id: string, words: string): string {
            return JSON.stringify(root.catalogueRun(id, words));
        }

        // What a place or a row shows, its drawer and the rows inside it,
        // for `launcher show` in a terminal, as JSON: the path's steps
        // joined by a "\u203a", each a row's name or its steady id, and
        // none for home. Its `reading` says a provider of a row shown is
        // still reading it, as `--json` prints it, and the program asks
        // again while it's true.
        function show(path: string): string {
            return JSON.stringify(root.shownToTerminal(path, false, false));
        }

        // The same for completion, which wants names alone, so it's read
        // for nothing (shownToTerminal's namesOnly).
        function names(path: string): string {
            return JSON.stringify(root.shownToTerminal(path, false, true));
        }

        // The same, with each row's drawer, for `launcher show --drawers`.
        function drawers(path: string): string {
            return JSON.stringify(root.shownToTerminal(path, true, false));
        }

        // Does what a terminal names, as launcher do asks it.
        function doing(asked: string): string {
            return JSON.stringify(root.doneFromTerminal(asked));
        }

        // Where every word on the card is drawn, for the check that nothing
        // moves without changing.
        function laidOut(): string {
            return root.laidOut();
        }

        // Every row Return does nothing on, as JSON, for the check.
        function actless(): string {
            return JSON.stringify(root.actless());
        }

        // How many times the search's rows have been joined, for the check
        // that a closed launcher makes nothing of what changes.
        function joined(): string {
            return String(root.choicesJoined);
        }

        // How many lines the shell has read of niri's windows, Niri.qml's
        // and each source's that hears the windows daemon, by name, for the
        // check that the shut shell hears nothing of a window resized.
        function linesHeard(): string {
            const by = { niri: Niri.linesHeard() };
            for (const source of root.sources) {
                if (typeof source.linesHeard === "function")
                    by[source.settings?.name ?? ""] = source.linesHeard();
            }
            return JSON.stringify(by);
        }

        // How many changes a source has said, open or closed, so that check
        // knows the closed launcher heard one before counting what it made.
        function changes(name: string): string {
            return String(root.changesIn[name] ?? 0);
        }

        // How many searches of what's typed have run, for the check that
        // text typed at once is searched once.
        function searched(): string {
            return String(root.searched);
        }

        // When each of the last keys, typed or arrowed, was pressed, when
        // the launcher's handler took it, when that handler was done, when
        // the frame it's drawn in began, when that frame's swap began and
        // when it reached the screen, in milliseconds of the monotonic
        // clock, oldest first, a key at each place in all six, its press
        // null if no press made it; and how many keys have reached a frame
        // since the shell started.
        function framed(): string {
            const drawn = root.keysFramed.length;
            return JSON.stringify({ timed: root.keysTimed, clock: "monotonic",
                pressed: root.keysPressed.slice(0, drawn),
                heard: root.keysHeard.slice(0, drawn), done: root.keysDone.slice(0, drawn),
                begun: root.keysBegun, rendered: root.keysRendered,
                frames: root.keysFramed });
        }

        // How many rows each source has made since the shell started, by
        // its name, and how many times the open launcher has refreshed,
        // how many changes it took as a key's own, the rows Ctrl+]'s looks
        // for where a row lives made (livesMade), and how often the list was
        // composed and how many rows were drawn afresh, for the check that a
        // source nobody sees makes none, the list refreshes at most four
        // times a second, and an arrow takes no change as its own, composes
        // nothing and draws no row afresh.
        function made(): string {
            return JSON.stringify({ rows: root.tallies(), looked: root.tallies("looked"),
                refreshes: root.refreshesDone,
                keyed: root.refreshesKeyed, walked: root.livesMade, composed: root.rowsMade,
                drawn: root.rowsDrawn, built: root.rowsBuilt });
        }

        // The keys of a kind a pick would know now (knowing), for the check
        // that a shut launcher knows a window come since it last made its
        // rows, with none made.
        function known(kind: string): string {
            return JSON.stringify(Object.keys(root.knowing())
                .filter(key => key.startsWith(`${kind}:`)));
        }

        // How many frames the card has drawn since this was first asked,
        // for the check that a launcher left still draws nothing.
        function frames(): string {
            root.framesCounted = true;
            return String(root.framesDrawn);
        }

        // The spans of the last openings (Traces.qml), oldest first, as
        // JSON: each opening, and each step, back, act and search in it
        // with where its list laid its first row.
        function traced(): string {
            return traces.traced();
        }
    }

    // Frames drawn, counted only once a check asks, so a launcher in use
    // pays nothing for it.
    property bool framesCounted: false
    property int framesDrawn: 0

    // Every word drawn in an item, the item itself and all inside it, each
    // with where it's drawn on the card: its box's left and top and where
    // its letters start, which an aligned text moves within its box.
    function wordsIn(item, into) {
        if (!item || !item.visible || item.opacity === 0)
            return [];
        const words = [];
        if (typeof item.text === "string" && typeof item.contentWidth === "number"
            && item.text !== "" && item.width > 0) {
            const at = item.mapToItem(into, 0, 0);
            const spare = Math.max(0, item.width - item.contentWidth);
            const inset = item.horizontalAlignment === Qt.AlignRight ? spare
                : item.horizontalAlignment === Qt.AlignHCenter ? spare / 2 : 0;
            // How wide it's drawn and whether it's cut, so a test can say
            // where a name is cut held still. A word in a box that clips it,
            // as a selected name reading on is, is cut by that box rather
            // than by an ellipsis, and the box is the room it holds.
            const clipper = item.parent && item.parent.clip ? item.parent : item;
            const box = clipper.mapToItem(into, 0, 0);
            // Styled words, as a side's of items with its dots muted, are
            // said as they read, which their ReadsOn holds as `plain`.
            const styled = item.textFormat === Text.StyledText
                && typeof item.parent?.plain === "string";
            words.push({ text: styled ? item.parent.plain : item.text,
                // And as it's marked up, so a test can say which ink each
                // of its items wears.
                styled: styled ? item.text : "",
                x: Math.round(at.x), y: Math.round(at.y),
                ink: item.color === undefined ? "" : String(item.color),
                starts: Math.round(at.x + inset), width: Math.round(item.width),
                inked: Math.round(item.contentWidth), cut: item.truncated === true
                    || (clipper !== item && item.implicitWidth > clipper.width),
                room: { x: Math.round(box.x), width: Math.round(clipper.width) } });
        }
        for (const child of item.children)
            for (const word of wordsIn(child, into))
                words.push(word);
        return words;
    }

    // Where the card's words are, as JSON: the field's, the foot's, and each
    // row's shown, with what the row is and how it reads, so a test can say
    // a row that reads the same was drawn in the same place (Joel,
    // 2026-10-25: "text alignment and shit constantly shifting around").
    function laidOut() {
        const rows = [];
        for (let at = 0; at < root.matches.length; at += 1) {
            const drawn = list.itemAtIndex(at);
            const row = root.matches[at];
            if (drawn && drawn.visible)
                rows.push({ at, is: Rank.identity(row), selected: at === root.current,
                    depth: row.depth ?? 0,
                    reads: [row.name, row.count ?? "", row.side ?? "", root.kindOf(row)]
                        .join(" \u00b7 "), words: wordsIn(drawn, card),
                    // Its band, its own room a gap above it aside.
                    band: Math.round(drawn.mapToItem(card, 0, drawn.height - root.rowTall).y),
                    // Whether its name is what was found, and the name's
                    // letters washed beneath, a run each, as many as its
                    // wash draws: a cut name's last may not.
                    byName: !row.byWord && !row.answered,
                    washed: drawn.drawnRow.finds.slice(0, drawn.drawnRow.washed.length)
                        .map(run => row.name.slice(run[0], run[1])),
                    // How many times its cut words have read on since it
                    // was selected (Choice.qml's readsDone).
                    readsDone: drawn.drawnRow.readsDone });
        }
        // Where the selection is drawn, and where its row is, so a test can
        // say it's drawn on its row.
        const chosen = list.itemAtIndex(root.current);
        const ground = picked.mapToItem(card, 0, 0);
        const selection = !picked.visible || !chosen ? null : {
            y: Math.round(ground.y),
            x: Math.round(ground.x), width: Math.round(picked.width),
            height: Math.round(picked.height), ink: String(picked.color),
            // Where its row's ground would stand, its band's top.
            row: Math.round(chosen.mapToItem(card, 0, chosen.height - root.rowTall).y)
        };
        // Where the first row is drawn below the top of the list, so a test
        // can say a list shown from its start leaves no space above it:
        // 0 there, less once scrolled, and null when it isn't drawn.
        const first = list.itemAtIndex(0);
        const firstAt = first ? Math.round(first.y - list.contentY) : null;
        // The command line's words while it's open, and its menu's, so a
        // test can say where the line stands and where its lines start;
        // and the gutter's numbers, so it can say where they stand.
        // The frame's inside across, and the hairlines' tops, so a test can
        // say where a ground meets them. And how far past the view the list
        // builds rows ahead, and how many it holds, so a test can say a
        // step drew only the rows in reach rather than every one; and how
        // many fact rows the drawer has made and how many it holds, so it
        // can say an arrow keeps them.
        const inside = listBox.mapToItem(card, 0, 0);
        return JSON.stringify({ typed: root.typed, field: wordsIn(prompt, card),
            numbers: wordsIn(rowsGutter, card), rowTall: root.rowTall,
            inside: { x: Math.round(inside.x), width: Math.round(listBox.width),
                y: Math.round(inside.y), height: Math.round(listBox.height) },
            hairlines: [headLine, footLine].map(line => ({
                y: Math.round(line.mapToItem(card, 0, 0).y), height: line.height,
                ink: String(line.color) })),
            foot: wordsIn(foot, card), rows, selection, made: root.rowsMade,
            drawn: root.rowsDrawn, built: root.rowsBuilt, factsMade: drawerCard.factsMade,
            factsHeld: drawerCard.facts.length,
            ahead: list.cacheBuffer, listed: list.count, firstAt,
            line: wordsIn(commandLoader.item, card),
            lines: wordsIn(commandRows, card) });
    }

    // The drawer beside the card (Drawer.qml): what the selected row shows
    // beyond its line, its own or one made of the picture and the whole
    // words it has. It's out only while the selected row is a thing and the
    // drawer says something the row doesn't, the same at home, in a search
    // and in every place; an act, a switch or a choice has none, and a
    // place's names what it holds (drawable) (VKCF3Z, 2026-10-29, after
    // Joel's "shows when it's not needed, and only shows in certain
    // places"; dae7fb67). Between two rows that both have one it stays, and what it
    // shows changes in place; onto a row without one, or from one, it fades
    // only once the selection has rested, so holding an arrow never
    // flickers it. `told`, where given, is told as `unread` of a drawer
    // read as it's first asked for, a turn later, that isn't in yet (the
    // row's `drawerUnread`, as a keyboard's), so a terminal asks again.
    function drawerOf(row, told) {
        if (!drawable(row))
            return null;
        const own = typeof row.drawer === "function" ? row.drawer()
            : row.drawer ?? heldDrawer(row);
        if (told && typeof row.drawerUnread === "function" && row.drawerUnread())
            told.unread = true;
        const drawer = own ? own : { title: row.name, body: row.reads ?? "",
            picture: row.picture ?? "", across: row.across ?? 0, down: row.down ?? 0 };
        // One that would only repeat its row is never shown (Showing.adds).
        return Showing.adds(drawer, row) ? drawer : null;
    }

    // Whether a row has a drawer: a thing, a plated row such as Now at
    // home, or a place, whose drawer names what it holds (PlaceRow.js), its
    // row saying nothing after its name, so one key onto it shows what's
    // inside (VKCF3Z, dae7fb67), a Recent or an Unread inside another
    // place included (heldDrawer). A place stays no thing (detailed).
    function drawable(row) {
        return detailed(row) || row?.plated === true
            || (!!(row?.opens || row?.category) && !!row.drawer)
            || (Array.isArray(row?.heldNames) && row.heldNames.length > 0);
    }

    // The drawer of a place inside another, a Recent or an Unread, which
    // gives the names of what it holds as `heldNames` rather than after
    // its name: a place's, with the same eyebrow, since whose it is is
    // the head's path above it (VKCF3Z, dafc2199). Null for any other row.
    function heldDrawer(row) {
        const names = row?.heldNames;
        return Array.isArray(names) ? PlaceRow.drawer(row.name, names, placeSaid) : null;
    }

    // Whether a row is a thing with details, by its kind rather than by
    // what it happens to hold, so the drawer stays out between two things
    // that each have more to say (VKCF3Z, 2026-10-29). A thing's maker says so with `thing`, as each
    // provider's maker of a program, a session, a device and the rest
    // does; a drawer composed for a row never makes it one, or the rule
    // would go by content again. A component's row (a mail, a meeting, a file, a person, a document)
    // is one by its kind, made by it or keyed as one; a value is one only
    // while it has a history, its graph a spark. An act, a switch, a
    // choice, a place and a plain value have none.
    function detailed(row) {
        if (!row || row.opens || row.category)
            return false;
        if (row.thing === true || !!row.component)
            return true;
        if (row.graph?.shape === "spark")
            return true;
        return components.some(one => one.keyed.test(row.key ?? ""));
    }

    // What the selected row has for the drawer, or null: typed to nothing,
    // nothing is selected, and it has none.
    readonly property var drawerWanted: !open ? null : listing ? listedPlate
        : commandPlate(commandLoader.item?.chosenLine ?? null)
            ?? (drawerRow ? drawerOf(drawerRow) : null)
    // The row the drawer is made for: the selection, once it has rested a
    // moment, so a key's own frame draws its highlight and never waits on a
    // drawer being made, and a held arrow's drawer follows once it's let go.
    // Made on the key's path, a session's or a program's drawer took 56 to
    // 122 ms a key in a VM (WENUOW, 2026-11-09), and Joel: "the highlight
    // doesn't always perfectly match my keyboard interactions" (2026-11-08).
    // Closed, it's let go at once.
    property var drawerRow: null
    readonly property int drawerFollows: 50
    onSelectedChanged: {
        // Once the event that selected it has run, since a step selects
        // its first row and again the row its arriving rows put first, and
        // the first read was thrown away: 4 to 7 ms a step into Desktop in
        // the loaded test machine (2026-11-10).
        Qt.callLater(root.readSelectedActions);
        if (!open || selected === null) {
            drawerFollowing.stop();
            drawerRow = selected;
        } else {
            drawerFollowing.restart();
        }
    }
    Timer {
        id: drawerFollowing
        interval: root.drawerFollows
        onTriggered: {
            const began = KeyTimes.now();
            const was = root.drawerShows;
            root.drawerRow = root.selected;
            root.drawerAsked = Math.round(KeyTimes.now() - began);
            // A drawer already out is made again for the row at once.
            if (root.drawerShows !== was)
                root.drawerMade(began, root.drawerAsked);
            Report.say(`drawer follows ${root.selected?.key ?? ""}`);
        }
    }
    // How long working out the drawer's row's drawer took when it was last
    // followed, which a drawer that comes out after resting made first.
    property int drawerAsked: 0
    // The drawer last drawn and timed, by its row's key, and how many have
    // been, for the check that times each place's drawer under load; the
    // journal says only those slow enough to feel. It's set anew, once a
    // drawer, so what the launcher shows is said again with it.
    property var drawerTimedLast: ({
        key: "", took: 0, making: 0, waiting: 0, drawing: 0, count: 0
    })
    // Times a drawer from the work begun for it, at `began` on KeyTimes's
    // clock, which keys and openings are timed by and setting the wall clock
    // never moves, to the first frame begun after, as a key is timed, and
    // `making` ms of it the launcher's own. Slow ones are said beside the
    // keys' lines in the journal (Trials.qml's drawerTimed). It's made off
    // the key's path, so the keys' own times never held it. One whose
    // launcher closes before its frame isn't timed, its frame never coming.
    function drawerMade(began, making) {
        if (!open)
            return;
        const key = drawerRow?.key ?? "";
        const made = KeyTimes.now();
        afterFrame(begun => {
            if (!root.open)
                return;
            const now = KeyTimes.now();
            const took = Math.round(now - began);
            const waiting = Math.round(begun - made);
            const drawing = Math.round(now - begun);
            root.drawerTimedLast = {
                key, took, making, waiting, drawing, count: root.drawerTimedLast.count + 1
            };
            trials.drawerTimed(key, took, { making, waiting, drawing });
        });
    }
    property bool drawerOut: false
    property var drawerShows: null
    onDrawerWantedChanged: {
        if (!open) {
            drawerResting.stop();
            drawerOut = false;
        } else if (drawerOut && drawerWanted !== null) {
            drawerResting.stop();
            if (!root.sameMoment(drawerShows, drawerWanted))
                drawerShows = drawerWanted;
        } else {
            drawerResting.restart();
        }
    }
    // Whether two drawers are the same moment's plate, saying the same:
    // home is made again on every refresh, and the plate kept rather than
    // set again keeps its groups rather than remaking them. Its time
    // counts on by itself.
    function sameMoment(shows, wanted) {
        return shows?.now === true && wanted?.now === true
            && JSON.stringify(shows) === JSON.stringify(wanted);
    }
    function drawerSettled() {
        const began = KeyTimes.now();
        drawerOut = drawerWanted !== null;
        if (!drawerOut)
            return;
        drawerShows = drawerWanted;
        // Timed with what working out its row's drawer took as it was
        // followed, but not the rest between, which is the drawer's pace.
        drawerMade(began - drawerAsked, Math.round(KeyTimes.now() - began) + drawerAsked);
    }
    // How long the selection rests before the drawer comes or goes.
    Timer {
        id: drawerResting
        interval: Theme.motion.medium
        onTriggered: root.drawerSettled()
    }

    // The drawer's room beside the card, and where the card sits (VKCF3Z's
    // ruling, after Joel's "never passes the bound of the screen and
    // resizes to fit there"): the card stays centred, where Super+Space has
    // always opened it, and the drawer takes what's left on its right, up
    // to drawerMost. Only where that's less than drawerLeast, too narrow to
    // hold a line of mail, does the card sit left of centre, the drawer
    // given drawerLeast, or where the screen hasn't that, the card's place
    // (drawerOver). Both are worked out for the
    // screen alone, never for what's shown, so the card never moves.
    readonly property int drawerMost: 520
    // The least holds a screenshot's name, "2026-11-03 00-10-41.png", on one
    // line of the head, its text an inset and a step in from each side; a
    // name is never broken across lines, and its time tells two apart.
    readonly property int drawerLeast: 288
    // Card and drawer are two frames that belong together, so they stand
    // the frames rung apart, wider than the inset either holds its text
    // in, and the pair the screen rung from the screen's edges, wider
    // again, so they read as two frames rather than one box's seam
    // (docs/design.md, The Ladder).
    readonly property real drawerGap: Theme.space.rung.frames
    readonly property real drawerMargin: Theme.space.rung.screen
    // The card's lines the drawer's are set on, down from its top: the
    // field's centre, the field filling the head's band, and the first
    // row's, below the field and its hairline.
    readonly property real fieldLine: Theme.border + prompt.height / 2
    readonly property real fieldBase: headBase
    readonly property real rowLine: Theme.border + headTall + Theme.border
        + Theme.space.rung.tie + rowTall / 2
    // How tall a row is, four beats; its middle is a centre, half of it,
    // rather than a space of its own.
    readonly property real rowTall: 4 * Theme.space.beat
    // The card's columns, from its content's edge, the inset in from the
    // frame (Card.qml), as the held notices' and a notice's are
    // (Columns.js): the gutter's numbers on the content's edge, as wide as
    // the room they hold for the place's list (the gutter keeps it) and a
    // mark after, the glyphs' column, and the text column a mark past a
    // glyph's square cell, so a launcher row differs from a held notice by
    // its gutter's width alone (Joel, 2026-11-06: "they need to share more
    // code and UI/UX design to stop them from constantly drifting").
    readonly property var columns: Columns.across(card.contentWidth,
        Columns.gutterOf(rowsGutter.room, Theme.space.rung),
        Columns.ofMark(Theme.level.name.pitch), Theme.space.rung)
    // A glyph's column is a mark's square cell, a line pitch wide, and the
    // mark rung before its text (VKCF3Z, 2026-11-05).
    readonly property real glyphColumn: columns.text - columns.mark
    // Where the rows stand, from the frame's inside edge: a row (Choice.qml)
    // keeps the card's inset inside itself, as the card's content does, so
    // it starts the inset before its glyph, which then stands on the mark
    // column, and the mark column's place counted from the frame is where
    // the row starts.
    readonly property real textAreaAt: columns.mark
    // Where the field's baseline stands, down from the card's top: centred
    // by eye in the head's band, the frame's inside to its hairline, which
    // the field fills (docs/design.md: text in a band centres in it).
    readonly property real headBase: Theme.border + prompt.baselineOffset
    // How tall the head is above its hairline: a row's band, its line
    // centred by the band rule, so the card keeps one rhythm, a head band,
    // the hairline, the rows' bands, the hairline and the status line, now
    // its Title is the line's size (VKCF3Z, 2026-11-06).
    readonly property real headTall: rowTall
    // How tall a row's capitals stand, which its ground is set round.
    readonly property real rowCaps: nameCaps.tightBoundingRect.height
    // Where the foot's baseline stands under its hairline: its capitals
    // centred in a row's height, as a row's are, and an inset above the
    // frame under it (VKCF3Z, 2026-11-05).
    readonly property real footBase: (rowTall + footCaps.tightBoundingRect.height) / 2
    // The gap that sets a page's groups apart, above a row that has one,
    // as Network's does; home has none; the card's height, the list and the rows' lines all read
    // it, so the plate's lines can't drift from the list's.
    readonly property real apart: matches.some(row => row.gap) ? Theme.space.beat : 0
    // A length to a hundredth, as reports say one, or null for none.
    function hundredths(length) {
        return length === null || length === undefined ? null : Math.round(length * 100) / 100;
    }
    readonly property real centred: !screenHere ? 0 : (screenHere.width - width) / 2
    readonly property real drawerRoom: !screenHere ? 0
        : screenHere.width - centred - width - drawerGap - drawerMargin
    // Whether the screen can't give the drawer its least beside the card
    // even with the card left of centre, as a 1080p screen at 2 can't.
    // There it isn't squeezed until a word breaks (VKCF3Z, 2026-11-05:
    // "Depl/oy/log"): it waits until the keys go in, then takes the card's
    // place and width, as Vim's preview opens in the one window there is,
    // and gives it back as they come out.
    readonly property bool drawerOver: !!screenHere
        && screenHere.width - width - drawerGap - 2 * drawerMargin < drawerLeast
    // Whether the keys are in the drawer, choosing in it or not.
    readonly property bool drawerHolds: QueryKeys.paged(prompt.mode)
    // The room beside the card the surface holds for the drawer.
    readonly property real drawerBeside: drawerOver ? 0 : drawerGap + drawerWidth
    readonly property real drawerWidth: !screenHere ? 0 : drawerOver ? width
        : drawerRoom >= drawerLeast ? Math.min(drawerMost, drawerRoom) : drawerLeast
    readonly property real cardLeft: !screenHere ? 0 : Math.max(drawerMargin,
        Math.min(centred, screenHere.width - drawerMargin - drawerBeside - width))
    // A quarter down; over a viewer low, its foot a twelfth of the screen
    // up, as the lock screen sits, so the picture or the video its line
    // acts on stays in view above it, as Vim's command line sits at the
    // bottom. Never so low it leaves no screen margin below it, on a
    // screen too short for a quarter down.
    readonly property real cardTop: !screenHere ? 0 : Math.round(over !== null
        ? screenHere.height * 11 / 12 - card.height
        : Math.max(Theme.space.rung.screen, Math.min(screenHere.height / 4,
            screenHere.height - Theme.space.rung.screen - card.height)))

    // What the launcher shows, as JSON: whether it holds the keyboard,
    // what's typed and held, the first twenty rows' names in order (a
    // collection holds thousands), and the row Return
    // takes, with its chord and whether an answer is still being asked for. A script waits on this before it
    // presses a key, rather than guessing how long the launcher takes.
    function showing() {
        const row = root.selected;
        const lines = drawerCard.baselines();
        return JSON.stringify({
            open: root.open,
            keys: root.open && prompt.Window.active,
            // How many rows' keys the foot has weighed afresh (noteKeys).
            weighed: root.rowsWeighed,
            typed: root.typed,
            // The line's mode, find, insert or normal, and where its caret
            // is, between letters on a line and on one in normal.
            mode: prompt.mode,
            // What leads the line, the find's / at every depth, and its hue
            // by the theme's role, and a held thing's glyph after it.
            sigil: prompt.cellMark,
            sigilHue: Qt.colorEqual(prompt.cellHue, Theme.accent) ? "accent"
                : Qt.colorEqual(prompt.cellHue, Theme.muted) ? "muted" : "",
            lead: prompt.lead,
            // The row Return landed on from the find, until the next key,
            // and whether a Return now would come too soon after it.
            landed: root.landedOn(root.selected) ? root.verbOf(root.selected) : "",
            landedFresh: root.landedJustNow(),
            // Whether a Return now would still be the one that wrote an
            // ending row's line, so a test waits it out before running it.
            linedFresh: Date.now() - root.linedAt < Options.keyTiming.wait,
            returned: root.returned,
            // What the foot says for a key in place of the keys, as q's words
            // or "not here", in the foot's order, until the next key.
            saying: root.sayingNow ? root.saying.slice(1).concat([root.saying[0]])
                .map(key => root.saidKey(key)) : [],
            // The places Ctrl+O and Ctrl+I go back and forward through.
            jumps: { back: root.jumpsBack.length, ahead: root.jumpsAhead.length },
            // And those Ctrl+] left, which Ctrl+T comes back through.
            tags: root.tagsBack.length,
            caret: prompt.caret,
            // A question asked once in the line's place, or "", and
            // whether it takes keys yet: those in its first moment don't.
            question: prompt.asking,
            takesAnswer: prompt.asking !== "" && root.askTaking,
            questionWhole: prompt.askedWhole,
            questionShown: prompt.askedShown,
            placeSays: root.placeSays,
            // What the mode word's room says: a count, a search, what's
            // chosen; and the foot's start, the mode in three capitals.
            modeSays: modeWord.visible ? modeWord.said : "",
            modeMark: statusLine.marked ? statusLine.mark : "",
            // A count typed in normal, waiting for its move, and the keys
            // of a sequence begun, as gg's first g.
            counted: prompt.standing.count,
            // How many rows wait behind rows held still (holdingRows).
            waiting: root.waitingRows,
            begun: prompt.standing.typed.join(""),
            // The command line, while it's open: what it holds, the line
            // chosen, and each line its menu shows, as its command and
            // title; and what Return on it said went wrong, for that key.
            command: !commandLoader.item ? null : {
                line: commandLoader.item.text,
                caret: commandLoader.item.caret,
                chosen: commandLoader.item.choosing.at,
                menu: commandLoader.item.shown.map(line => (line.argument ?? "") === ""
                    ? `${line.command} ${line.title}`
                    : `${line.command} ${line.argument} ${line.title}`),
                from: root.commanding?.from ?? "",
                // The range it was opened with, as visual's '<,'>, and
                // how many rows its acts act on.
                range: root.commanding?.range ?? "",
                things: root.commanding?.things.length ?? 0,
                // What it's aimed at over a viewer, whose rows aren't drawn.
                // After :q over a viewer the line is gone while the viewer
                // stands a moment, so commanding may be null here.
                over: root.over ? (root.commanding?.things ?? []).map(thing => thing.name) : null
            },
            // Where the card's top and foot sit on a screen this tall.
            card: {
                top: root.cardTop,
                foot: root.cardTop + card.height,
                screen: root.screenHere ? root.screenHere.height : 0,
                // And its sides, on a screen this wide.
                left: root.cardLeft,
                right: root.cardLeft + root.width,
                across: root.screenHere ? root.screenHere.width : 0,
                // Its height but for its rows, as summed and as drawn.
                chrome: root.cardChrome,
                drawn: card.height - listBox.height
            },
            commandSaid: root.commandSays ? root.commandSaid : "",
            // Whether Space ? lists the keys in the drawer, and whether
            // there's a place used last for Tab to go back to.
            listing: root.listing,
            // Normal's gutter: whether it shows, its numbers top to bottom,
            // the cursor row's, how many it made and how wide its room is.
            gutter: {
                shown: rowsGutter.shown, relative: rowsGutter.relative,
                numbers: rowsGutter.said().map(number => number.text),
                cursor: (rowsGutter.said().find(number => number.ink === "text") ?? {}).text
                    ?? "",
                made: rowsGutter.made, room: Math.round(rowsGutter.width),
                // One figure's width, which a test measures the numbers by.
                figure: rowsGutter.widest("9")
            },
            lastPlace: root.lastPlace !== null,
            // An operator waiting for its motion, as d's first d, and the
            // foot's word for one done, as d's ask, its mark and its word;
            // and the rows y flashes, by their place.
            operator: prompt.standing.operator?.key ?? "",
            operated: foot.operatorWord === null ? ""
                : `${foot.operatorWord[0]} ${foot.operatorWord[1]}`,
            flashing: Object.keys(root.flashing).map(Number),
            // The mode word's room as it reads, and the rows that wear
            // visual's tint, by their place: visual's, what a pending
            // operator will take, and what an ask covers.
            modeWord: modeWord.visible ? modeWord.said : "",
            tinted: Array.from({ length: root.matches.length }, (_, at) => at)
                .filter(at => root.tints(at)),
            // Which row is selected, counted from 0, -1 for an answer's
            // headline, and the first and last rows wholly in view.
            current: root.current,
            inView: list.inView(),
            // How wide its caret is drawn, and whether it's drawn: not in
            // normal. How lit it is as it blinks isn't said, since this is
            // said on every change.
            caretWidth: root.hundredths(prompt.caretWidth),
            caretShown: prompt.caretShown,
            // Whether the line has modes: not while it asks for a secret.
            modal: prompt.modal,
            // How the field finishes what's typed, as " = 4" after a sum.
            completion: root.completion,
            // What the field asks while nothing's typed.
            hint: prompt.hint,
            // How many the find found, said after what's typed in normal.
            found: prompt.narrowSaid ? prompt.narrowed : "",
            held: root.held.map(step => step.row.chip ?? step.row.name),
            // Whether, closed, it holds home kept for the next opening.
            kept: root.homeHeld,
            path: root.path,
            // Whether each of the path's steps is said whole in the field.
            pathWhole: prompt.chipsWhole,
            // How each step is drawn: its words, whether they're cut, and
            // the room past them before the next step's mark.
            pathDrawn: prompt.pathDrawn(),
            rows: root.matches.slice(0, 20).map(row => row.name),
            // The rows the card shows, by what each is and what it says at its
            // right, so a check sees each frame of an opening filling in.
            seen: root.rowsSaid(),
            // How many rows there are in all, beyond the twenty named.
            count: root.matches.length,
            // What each of those rows is, and how it reads whole, so a
            // test can say no thing is listed twice.
            things: root.matches.slice(0, 20).map(row => ({
                is: Rank.identity(row),
                gone: !!row.gone,
                reads: [row.name, row.count ?? "", row.side ?? "", root.kindOf(row)]
                    .filter(part => part !== "").join(" \u00b7 ")
            })),
            asking: !!root.asking,
            // What the list says in place of rows, where it has none.
            nothing: nothingShown.visible ? nothingShown.name : "",
            selected: !row ? null : {
                name: row.name,
                is: Rank.identity(row),
                verb: root.verbOf(row),
                chord: (row.chord ?? []).map(key => key.text).join(" "),
                side: row.side ?? "",
                // What's drawn after its name, what a row holds.
                count: row.count ?? "",
                // Its glyph's role, and whether what it says at its right
                // takes the alert's.
                hue: row.hue ?? "",
                alerts: !!row.alerts,
                // What's said below its name, a collection's note.
                note: row.note ?? "",
                // The whole words drawn below the rows for it, if any.
                reads: row.reads ?? "",
                // What the drawer beside the rows shows of it, if anything,
                // made as drawerOf makes it when the row makes it late.
                drawer: (typeof row.drawer === "function" ? row.drawer() : row.drawer) ?? null,
                // The keys the foot says for it, each a mark and its word.
                told: root.keysFor(row).map(key => root.saidKey(key)),
                // The key its side names where it ends something.
                carried: root.carriedBy(row),
                settled: !(row === root.headline || row.answered)
                    || !(root.answer && root.answer.asking)
            },
            kind: root.kind,
            footer: foot.said,
            // What the cursor row says at its right, and its note as it's
            // drawn after its name, fitted, so a test can hold each where
            // it's said.
            stands: foot.saidStands,
            noted: list.itemAtIndex(root.current)?.drawnRow.noteWhole ?? "",
            glance: standing.said,
            // Where Return's key starts, which holds still as the arrows go.
            keysAt: Math.round(foot.returnAt),
            // Where the status line's last ink should end: the inset in
            // from the frame, as its first starts.
            footEnd: Math.round(foot.mapToItem(card, foot.width - card.inset, 0).x),
            // How far in the keys may reach, room held for keys a row
            // may say included, which the facts are fitted short of.
            keysFrom: Math.round(foot.keysFrom),
            // The room held for the widest keys since opening, and what the
            // keys said now take, so a test can tell which held it; and the
            // room the facts at their least and the mode's word leave them,
            // with what every key told would take, so a key given way can
            // be told short by how much.
            keysRoom: { held: Math.round(Math.min(root.footKeysWidest, foot.keysRoom)),
                said: Math.round(root.keysWide(foot.kept)), left: Math.round(foot.keysRoom),
                told: Math.round(root.keysWide(foot.told, foot.loose)),
                // Return's place beside what its hint says, wider only
                // while the place is held for keys.
                place: Math.round(foot.returnPlace),
                placeSays: foot.told.length === 0 ? 0
                    : Math.round(statusLine.keys.advance(foot.told[0][0], foot.told[0][1])),
                mode: Math.round(modeWord.room), least: Math.round(standing.least),
                // How much wider the keys' room holds Return than its place
                // draws it: the widest verb the words name, held so the
                // facts never slide, against the rows' own (footReset).
                returnSlack: Math.round(Math.max(0,
                    statusLine.keys.advance("\u21b5", root.footVerbRoom)
                    - root.returnPlace(foot.kept[0] ?? ["\u21b5", ""]))) },
            // The keys the foot says now, each its mark and its word.
            hints: foot.kept.map(key => `${key[0]} ${key[1]}`),
            // And the keys it would say with room for all, before any give way.
            wanted: foot.toldNow.map(key => `${key[0]} ${key[1]}`),
            // How far the list is scrolled, which only a key changes.
            scrolled: Math.round(list.contentY),
            // The drawer: whether it's out, what it's titled and says, the
            // title it draws itself, whether it holds more than fits, and
            // where it and the card sit.
            drawer: {
                out: root.drawerOut,
                // The row it follows, whether that row has one, and the row
                // whose drawer was last drawn and timed.
                row: root.drawerRow?.key ?? "",
                wanted: root.drawerWanted !== null,
                timed: root.drawerTimedLast,
                // Whether there's no room for it beside the card, and
                // whether it's drawn.
                over: root.drawerOver,
                shown: drawerCard.out,
                drawn: drawerCard.title,
                title: root.drawerShows?.title ?? "",
                facts: (root.drawerShows?.facts ?? []).filter(fact => fact)
                    .map(fact => fact.label ?? ""),
                // And what each says, in the same order.
                values: (root.drawerShows?.facts ?? []).filter(fact => fact)
                    .map(fact => String(fact.value ?? "")),
                body: root.drawerShows?.body ?? "",
                // Its words as drawn, a run laid in lines (Drawer.qml's bodySaid).
                bodyDrawn: drawerCard.bodySaid,
                // How many lines Qt draws them in.
                bodyLines: drawerCard.bodyLines,
                eyebrow: root.drawerShows?.eyebrow ?? "",
                // Whether it's set as written, naming things, or in capitals.
                eyebrowNames: !!root.drawerShows?.eyebrowNames,
                // What of it is drawn in the room it has.
                eyebrowDrawn: drawerCard.eyebrowSaid,
                // Where its lines' baselines are and the card's they're
                // set on, down from the top the two share, to a hundredth.
                lines: {
                    eyebrow: root.hundredths(lines.eyebrow),
                    eyebrowMiddle: root.hundredths(lines.eyebrowMiddle),
                    title: root.hundredths(lines.title),
                    titleLast: root.hundredths(lines.titleLast),
                    preview: root.hundredths(lines.preview),
                    previewEnd: root.hundredths(lines.previewEnd),
                    previewLines: lines.previewLines.map(line => root.hundredths(line)),
                    words: root.hundredths(lines.words),
                    facts: lines.facts.map(line => root.hundredths(line)),
                    time: root.hundredths(lines.time),
                    timeMiddle: root.hundredths(lines.timeMiddle),
                    seconds: root.hundredths(lines.seconds),
                    groups: lines.groups.map(group => group.map(line => root.hundredths(line)))
                },
                // What the moment's plate says, line by line.
                moment: drawerCard.moment ? {
                    time: drawerCard.momentTime,
                    seconds: drawerCard.momentSeconds,
                    groups: drawerCard.groups.map(group => group.facts.map(fact =>
                        [fact.label, fact.value, fact.qualifier ?? ""]))
                } : null,
                cardLines: {
                    field: root.hundredths(root.fieldLine),
                    // The field's baseline, which the drawer's eyebrow
                    // sits on.
                    fieldBase: root.hundredths(root.fieldBase),
                    // Each of the first twenty rows' names' baselines,
                    // null for one not drawn.
                    rows: root.matches.slice(0, 20)
                        .map((_, at) => root.hundredths(list.baselineOf(at))),
                    // Each of the same rows' words, where they sit.
                    words: root.matches.slice(0, 20).map((_, at) => list.wordsOf(at))
                },
                // The facts it draws, by name, whether each name is on one
                // line whole, and where its last line's box may end.
                drawnFacts: drawerCard.facts.map(fact => fact.heading ?? fact.label ?? ""),
                // Each fact's value as it's drawn, null while it's asked.
                values: drawerCard.facts.map(fact => fact.value ?? null),
                // Each fact's figure drawn beside its value, by its shape,
                // "" for none or one given way.
                figures: drawerCard.figuresDrawn(),
                // Its preview: the picture it's of, or how many lines.
                preview: drawerCard.preview === null ? null : {
                    picture: drawerCard.preview.picture ?? "",
                    lines: (drawerCard.preview.lines ?? []).length
                },
                labelsWhole: drawerCard.labelsWhole(),
                factsWrapped: drawerCard.factsWrapped(),
                factsUnder: drawerCard.factsUnder(),
                // Whether its facts read as a table or a list, never a mix.
                laidOneWay: drawerCard.laidOneWay(),
                // Whether every value keeps its words whole where it's drawn.
                valuesWhole: drawerCard.valuesWhole(),
                // Whether every value beside its name takes one line there.
                besideOnOneLine: drawerCard.besideOnOneLine(),
                // Whether a script's own digits stand as tall as figures.
                digitsStandAsFigures: drawerCard.digitsStandAsFigures(),
                foot: root.hundredths(drawerCard.foot),
                more: drawerCard.more,
                // What its last slot says of the things it hides below, and
                // that slot's baseline, "" and -1 when it hides none.
                moreSays: drawerCard.moreSays,
                // The way in as it's drawn, key by key.
                moreKeys: drawerCard.moreKeys,
                moreBase: drawerCard.moreSays === "" ? -1
                    : root.hundredths(drawerCard.moreBase),
                scrolled: Math.round(drawerCard.scrolled),
                // Whether the view's top cuts a line partway, which it never
                // should, as Vim's topline is always a whole line.
                cut: drawerCard.topCut(),
                // Whether it holds the keys, and its cursor while it does:
                // its line, of how many, the letter along it, the words
                // the line says and where the line is, down from the
                // drawer's top, and where its rule is drawn, in from the
                // drawer's left.
                keys: drawerCard.keys,
                frame: String(drawerCard.keys ? Theme.accent : Theme.edge),
                cursor: drawerCard.block === null ? null : {
                    line: drawerCard.cursor.line,
                    lines: drawerCard.laid.length,
                    letter: drawerCard.cursor.letter,
                    on: drawerCard.block.letter,
                    says: drawerCard.block.said,
                    top: Math.round(Theme.border + drawerCard.block.y - drawerCard.scrolled),
                    height: Math.round(drawerCard.block.height),
                    rule: drawerCard.rule()
                },
                // Its search: what's being typed, the last run, what's
                // washed, and "wrapped" or "missing" for where it landed.
                search: {
                    typing: drawerCard.seeking,
                    last: drawerCard.lastSought ? drawerCard.lastSought.pattern : "",
                    washing: drawerCard.washing,
                    landing: drawerCard.landing,
                    // Which match of how many, [at, of], as the foot counts.
                    found: drawerCard.found ? [drawerCard.found.at, drawerCard.found.of] : null
                },
                // Its visual: what it chooses by, "letters" or "lines", or
                // ""; each line it takes, [line, from, to]; how many the
                // mode word counts; what y copied last, and whether it's
                // flashing what it took.
                chosen: {
                    by: drawerCard.choosing,
                    taken: drawerCard.taken.map(one => [one.line, one.from, one.to]),
                    count: drawerCard.chosenCount,
                    copied: drawerCard.copied,
                    flashing: drawerCard.choosing === "" && drawerCard.flashing.length > 0,
                    // V's bands as drawn, each { x, width, top, height } in
                    // the letters' column, and that column's width.
                    bands: drawerCard.bandsDrawn(),
                    measure: Math.round(drawerCard.measure)
                },
                cardAt: Math.round(card.mapToGlobal(0, 0).x),
                cardWidth: Math.round(card.width),
                at: Math.round(drawerCard.mapToGlobal(0, 0).x),
                width: Math.round(drawerCard.width),
                height: Math.round(drawerCard.height),
                cardHeight: Math.round(card.height)
            }
        });
    }

    // Said each time it changes, while reports are on, so the test hears
    // every step rather than asking over and over. It's said as the event
    // loop's turn ends, which is what a frame can draw: said as it changed,
    // a search was heard partway, what's typed already new and its rows
    // not yet found, which no frame ever showed (the answers test's
    // "came and went", 2026-11-02, once text typed at once was searched
    // once).
    //
    // It's made once a turn too, not once for each thing it reads that
    // moves: as a binding on showing() it was made again for every one, 28
    // times in one arrow in Programs that took 283 ms, each laying the
    // drawer's identifiers again, so keeps-up's keys queued past its 30 s
    // (net a99645168 and a VM's timings, 2026-11-09), a cost Joel's shell,
    // with reports off, never pays. Made, it reads what showing() reads; the first of those
    // to move makes it again cheaply, reading none of them but
    // `showingTurn` and asking for the turn's end, so the rest move
    // unheard, and at the turn's end `showingTurn` moves and it's made
    // whole, once.
    //
    // That turn's end waits for the frame begun after it is drawn, and
    // for the turn after that frame's own handlers, so making it never
    // holds back the frame a key or step is timed to: made as the turn
    // ended, it was 2 to 11 ms of each step into a program's place under
    // load, between its handler and its frame (keeps-pace's scene graph
    // timings, 2026-11-10), time Joel's shell never spends. What it says
    // is still a whole turn's, since nothing is made partway.
    property int showingTurn: 0
    readonly property var showingMade: ({ turn: -1, said: "" })
    readonly property string shown: {
        const turn = showingTurn;
        if (!Options.reports)
            return "";
        if (showingMade.turn === turn) {
            Qt.callLater(showingTurnDue);
            return showingMade.said;
        }
        showingMade.turn = turn;
        showingMade.said = showing();
        return showingMade.said;
    }
    property string shownSaid: ""
    function showingTurnDue() {
        afterFrame(() => Qt.callLater(showingTurnEnds));
    }
    function showingTurnEnds() {
        showingTurn += 1;
        sayShown();
    }

    onShownChanged: Qt.callLater(sayShown)
    function sayShown() {
        if (shown === shownSaid)
            return;
        shownSaid = shown;
        Report.say(`showing ${shown}`);
    }

    // The window the launcher opened over, as niri describes it, or null on
    // an empty workspace: what Return pastes into (Pasting.qml), and whose
    // program's keys Shortcuts leads with (launcher/KeyMap.qml).
    function cameFrom() {
        return Pasting.under();
    }

    // A key that opens a collection, pressed (launcher-say): with the
    // launcher closed it opens into the collection, and while it shows, the
    // collection may take the key for what it means there, as Print takes
    // the screen with the launcher in it rather than changing what's shown
    // (Capture.qml). With no shell listening, qs ipc carries the key to
    // into, the launcher being closed then anyway.
    function pressed(name) {
        const source = collections.find(s => s.settings.name === name);
        if (open && source && source.pressedOver(held.length > 0 && held[0].row.from === source))
            return;
        chordInto(name);
    }

    // Opens straight into a collection, all of it shown until something's
    // typed; - steps out to the search. A name no collection
    // has opens the search.
    function into(name) {
        if (!open && !opening)
            openChord = name;
        intoWhole(name);
        keepPlace(name);
    }

    // Opens into a collection as its chord does: closed, a place kept from
    // its last opening is drawn at once and filled in after its frame
    // (intoKept), as home is. Only a chord's opening is, since what else
    // opens one steps on at once, as the file chooser does into its ask's
    // folder, and filling a kept place in opens it whole again, at its
    // top: every ask after the first opened on / (vm-desktop-looks-file-
    // picker, 2026-11-11).
    function chordInto(name) {
        if (!open && !opening && intoKept(name))
            return;
        into(name);
    }

    // Each place a chord opens, as it last opened: the steps to it, its
    // rows, how deep it is and what its rows follow of the window it
    // opened over (keptUnder), by the chord's name. Its chord opened it
    // whole every time, 203 to 597 ms to its frame, while home drew from
    // what was kept in 27 to 56 (Joel, 2026-11-10, 20:39 to 20:43). Plain,
    // so keeping notifies nothing.
    readonly property var placesKept: ({})
    // The chord a kept opening is to fill in, and the one the opening
    // being timed came by, said in its line.
    property string openInto: ""
    property string openChord: ""
    function keepPlace(name) {
        if (held.length === 0)
            return;
        placesKept[name] = { held: held, matches: matches, depth: homeDepth,
            under: keptUnder(name) };
    }
    // What a place's rows follow of the window the launcher opens over, as
    // its source says (keptOver): Shortcuts' the keys that work there,
    // Windows' the window itself, which they leave out. The rest follow
    // none, so they're drawn as kept over any window: kept with the app
    // opened over, every other chord pressed over another program's window
    // was made whole.
    function keptUnder(name) {
        const source = collections.find(one => one.settings.name === name);
        return typeof source?.keptOver === "function" ? source.keptOver(cameFrom()) : "";
    }
    // Draws a place as it was kept, while the closed card holds home and
    // what its rows follow of the window under it is as it was kept. It's
    // filled in after its frame by opening it whole (filled), which lands
    // its rows where the kept ones stood.
    function intoKept(name) {
        const kept = placesKept[name];
        if (!kept || !homeHeld || parting !== "" || kept.under !== keptUnder(name))
            return false;
        keeping = true;
        try {
            held = kept.held;
            homeDepth = kept.depth;
            listing = false;
            listedInDrawer = false;
            steadying = false;
            typed = "";
            candidates = [];
            prompt.text = "";
            matches = kept.matches;
            current = 0;
            tall = rows * rowTall + (matches.slice(0, rows).some(row => row.gap) ? apart : 0);
        } finally {
            keeping = false;
        }
        homeHeld = false;
        openChord = name;
        openInto = name;
        beginOpening();
        return true;
    }

    function intoWhole(name) {
        openInto = "";
        const source = collections.find(s => s.settings.name === name) ?? null;
        // Unless home is to be found as it opens, with none kept yet, since
        // the card's height is home's.
        entering = open || homeKept.length > 0 ? source : null;
        // Opened by it, the place is shown rather than home (openingInto).
        openingInto = !open;
        open = true;
        openingInto = false;
        // A name a source leads somewhere deeper, as Now leads "notices"
        // to For You's Desktop (`leadsTo`), opens on its place and steps
        // through those rows as Joel would, so - climbs back out the same
        // way and Escape at rest closes there.
        const { leading, rows } = leadingTo(name) ?? { leading: null, rows: [] };
        if (leading !== null && leading.settings.name !== name) {
            into(leading.settings.name);
            for (const row of rows) {
                if (!enter(row))
                    break;
            }
            openedHere();
            return;
        }
        if (!source)
            return;
        held = [Object.assign(here(), {
            row: {
                key: "",
                name: source.collection.noun,
                chip: source.collection.noun,
                title: source.collection.title,
                ask: source.collection.ask,
                lead: source.collection.glyph,
                leadHue: "voice",
                from: source
            },
            rows: []
        })];
        entering = null;
        source.opened();
        restart("");
        // Every place opens on its first row, the same each time, so Joel
        // knows where the keys are before he looks (VKCF3Z, 2026-10-29). A
        // collection may lead into one of its parts, as Keyboard does into
        // the program in use's keys, which opens on its first row too.
        const lead = source.leading ? source.leading() : null;
        if (lead)
            enter(lead);
        openedHere();
    }

    // Says the place it shows now is where it opened, so normal's Escape at
    // rest closes it here; a collection that leads somewhere deeper than
    // into() does, as the file chooser does into its folder, says so too.
    function openedHere() {
        homeDepth = held.length;
        // A place it never showed on the way is none to go back to.
        lastPlace = null;
        jumpsBack = [];
        jumpsAhead = [];
        forgetFollows();
    }

    // How deep the place it opened on is: home's 0, or a collection opened
    // straight into, where :q closes it, as it does at home, rather than
    // stepping out to a home it never showed.
    property int homeDepth: 0

    // Whether the open launcher has had the keys since it opened: niri
    // hands them over a frame after it asks, so until then a window still
    // has them and that says nothing (keysMoved).
    property bool keysHeld: false

    // A click or a touch on a window takes the keys from the open launcher
    // (modules/niri/on-demand-when-asked.patch), and one left open that
    // hears nothing reads as broken: a phantom touch on a cracked screen
    // left Joel's keys going nowhere he could see, and Super+Space then
    // closed it rather than giving them back (2026-11-09). So once it has
    // held them, a window taking them closes it, as a click outside a menu
    // puts it away; launcher/KeysHeld.js says when it holds them and when
    // they're taken. Called as the keys or niri's focused window change,
    // in either order.
    function keysMoved(): void {
        if (!open && !opening)
            return;
        const moved = KeysHeld.moved(keysHeld, {
            active: prompt.Window.active,
            focused: Niri.focused,
            lettingGo
        });
        if (moved === "held") {
            keysHeld = true;
        } else if (moved === "taken") {
            console.info("launcher: closed, since a window took the keys");
            if (opening)
                unopened();
            else
                open = false;
        }
    }

    // Says whether the launcher is open, in $XDG_RUNTIME_DIR/quickshell-launcher,
    // so what can't ask the shell, as a switch run by root waiting until
    // Joel isn't in it, can read it: "open" or "closed", written as it
    // changes and once as the shell starts, since one a shell that died left
    // says nothing of this one; the unit removes it as the shell stops.
    FileView {
        id: openMark
        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/quickshell-launcher`
        printErrors: false
        Component.onCompleted: root.markOpen(root.open)
        onSaved: root.markWritten()
        onSaveFailed: error => root.markFailed(error)
    }
    // What the mark says now, so each word is written once as it changes,
    // and when it last said open, on the clock keys are timed by.
    property string markSays: ""
    property real markedOpenAt: -1
    // The word a write off the shell's thread is still putting down, or
    // none, and how many writes have held the shell while they were done,
    // for the check that an opening makes none.
    property string markWriting: ""
    property int markBlocked: 0
    function markOpen(shown) {
        const word = shown ? "open" : "closed";
        if (word === markSays)
            return;
        markSays = word;
        if (shown)
            markedOpenAt = KeyTimes.now();
        if (markWriting === "")
            markWrite();
    }
    // Open is written off the shell's thread, since written blocking it
    // held a kept opening's fill 3 to 121 ms on a loaded builder (ZINAWZ,
    // 2026-11-11), and a switch reading it a moment late only waits.
    // Closed blocks, so it's said by the time the shell says it closed. A
    // word wanted while another is still being written follows once that
    // one is saved, so the last said is always the last written.
    function markWrite() {
        const blocking = markSays !== "open";
        openMark.blockWrites = blocking;
        markWriting = blocking ? "" : markSays;
        if (blocking)
            markBlocked += 1;
        openMark.setText(`${markSays}\n`);
    }
    // A mark it couldn't write leaves its readers reading the last word,
    // so that's said, once a shell, since each opening would say it again
    // (Y2YBTS, 2026-11-11).
    property bool markFailSaid: false
    function markFailed(error) {
        if (!markFailSaid) {
            markFailSaid = true;
            console.warn(`launcher: couldn't write the open mark: ${FileViewError.toString(error)}`);
        }
        markWritten();
    }
    function markWritten() {
        const written = markWriting;
        markWriting = "";
        if (written !== "" && written !== markSays)
            markWrite();
    }

    Connections {
        target: Niri
        enabled: root.cardUp

        function onFocusedChanged() {
            root.keysMoved();
        }
    }

    // Joel handed to another program's window: what asks a program that
    // may put a window up for him while the launcher stays open, as
    // stepping into Passwords asks 1Password, which shows its unlock,
    // says so first (`provider.launcher?.handsOver(apps, patience)`), naming the program by
    // the app ids its windows carry. The launcher holds the keys above
    // every window, so a window that came up under it waited there
    // unseen (Joel, 2026-11-10: "the unlock screen shows
    // below the launcher. very poor UX"). So a window of one of them
    // opening or coming forward within the patience given closes the
    // launcher and takes the focus, his own Return having asked for it.
    // Only a program named counts, so a window anything else opens, as
    // a session's, never takes it. Null while nothing is handed over;
    // `until` on KeyTimes' monotonic clock.
    property var handing: null

    function handsOver(apps, patience): void {
        if (!open || apps.length === 0)
            return;
        handing = { apps: apps, until: KeyTimes.now() + patience };
    }

    function handedTo(window): void {
        const handed = handing;
        if (handed === null)
            return;
        if (!open || KeyTimes.now() > handed.until) {
            handing = null;
            return;
        }
        if (!handed.apps.includes(window.app_id))
            return;
        handing = null;
        console.info(`launcher: closed for ${window.app_id}'s window, which Joel asked for`);
        Report.say(`handed to ${window.app_id}`);
        open = false;
        Niri.act({ FocusWindow: { id: window.id } });
    }

    Connections {
        target: Niri
        enabled: root.handing !== null

        function onWindowOpenedOrChanged(window) {
            root.handedTo(window);
        }
    }

    onOpenChanged: {
        // Each part of an opening's own work, timed as it's done, so the
        // line its frame says names what an opening slow to draw spent its
        // time on (Joel, 2026-11-10: the launcher takes forever to load).
        let lapped = KeyTimes.now();
        const parts = { before: open && openAsked >= 0 ? lapped - openAsked : 0 };
        const lap = name => {
            const now = KeyTimes.now();
            parts[name] = (parts[name] ?? 0) + now - lapped;
            lapped = now;
        };
        // Opened from what it kept, it was shown already, holding the keys
        // since then; what follows fills it in.
        const filling = open && opening;
        const began = lapped;
        opening = false;
        markOpen(open);
        if (!open)
            Report.say("closing");
        keysHeld = filling && keysHeld;
        lettingGo = false;
        handing = null;
        // A viewer's line is over once the launcher closes (viewerCommandLine),
        // and gv has nothing to bring back in the rows it opens on; nor is a
        // line Return wrote and Joel left, which would hold the next
        // opening's rows still and take its next line's place.
        if (!open) {
            // Shut while its sources woke, the rest are woken, so a source
            // walked from outside or a closed launcher's check finds it as
            // ever; shut, none of its rows are wanted, so none are made.
            wakeRest();
            // What waits on a frame runs, since no frame of the launcher's
            // is coming.
            afterFramesDue();
            over = null;
            lastVisual = null;
            rowChanges = Changes.changes();
            lastChange = null;
            commanding = null;
            // Shut in the mode it opens in, so what it keeps is weighed for
            // that mode while shut, and the opening's take() changes no
            // mode. Closed by :q from normal, each reopening weighed its
            // rows' keys for normal as the kept rows landed, then again for
            // the find as take() changed the mode, 5 and 4 ms of Super+Tab's
            // opening over 40 windows on a builder (ZINAWZ, 2026-11-11).
            prompt.mode = prompt.typingMode;
        }
        madeSince(open ? "opening" : "closing");
        if (!open)
            keysPresentedSaid();
        if (open)
            traces.opened({});
        else
            traces.closed({ depth: held.length, rows: matches.length });
        if (open) {
            openWoke = wakeEach();
            lap("waking");
            // Joined whole each time it opens, stale or not: a
            // collection's count reads live state (Agents' sessions, the
            // machine's room) that marks nothing stale as it changes, so
            // home would open saying the count of the last join.
            choicesStale = true;
            lap("marking");
            freshen();
            lap("joining");
            trials.opened();
            lap("trials");
            partingLater.stop();
            fading = false;
            parting = "";
            footCatchUp();
            lap("catching up");
            footStage = 0;
            if (Object.keys(factsWidest).length > 0)
                factsWidest = {};
            figuresStep = "";
            homeDepth = 0;
            lastPlace = null;
            listing = false;
            listedInDrawer = false;
            // Left behind once the card is gone (leaveSteps), so an opening
            // finds home already there and pays only for steps taken while
            // shut.
            leavingLater.stop();
            leaveSteps();
            lap("forgetting");
            often = picks.counts();
            lap("counting");
            spotsLater.stop();
            spotsFreshened.stop();
            // Home as it's found now, kept for the next opening to draw at
            // once (keepHome). One opening into a place (into) shows the
            // place, so home isn't found only to be put away: Super+V's
            // openings made home's 51 and 60 rows first, in 124 and 138 ms
            // of settling (Joel's journal, 2026-11-10, 20:40). With no home
            // kept yet it's found, since the card's height is home's.
            const homeFound = !openingInto || homeKept.length === 0;
            if (homeFound) {
                restart("");
                homeKept = matches;
            }
            homeHeld = false;
            lap("finding");
            // Nothing to rest from as it opens, so the drawer is out at once
            // when the first row has details.
            drawerResting.stop();
            drawerSettled();
            lap("drawer");
            // Always the same height, eight rows, kept until it closes, so
            // the card is the same shape however much home holds and its
            // rows fill it to the foot, none held under the last.
            const sized = homeFound ? matches : homeKept;
            tall = rows * rowTall + (sized.slice(0, rows).some(row => row.gap)
                ? apart : 0);
            // Before niri hands over the keys, so the first one lands.
            prompt.take();
            lap("keys");
            // The gutter as wide as every place's.
            gutterDigits = leastDigits;
            numberDigits = 1;
            noteDigits();
            lap("digits");
            // Weighed once an opening, not on every mode change, which
            // walks every thing's verbs.
            const weighed = Pacing.verbRoom(footVerbsWeighed, Words.all,
                statusLine.keys.measured, words => widestVerb(words.things ?? []));
            openVerbsWeighed = weighed === footVerbsWeighed ? 0
                : (Words.all.things ?? []).length;
            footVerbsWeighed = weighed;
            footVerbRoom = weighed.widest;
            lap("verbs");
            // The foot's room once the line is back on the find: set
            // before, the rows noted their keys while it was still in
            // the normal it closed in, and the find opened with normal's
            // room, cutting the glance's date and charge.
            footReset();
            lap("foot");
            // Into a place, its steps come next (into), and its rows note
            // their keys as they're listed: noted here, with the steps let
            // go, a kept place's rows were weighed under home's key, and
            // home's own weighed again after (ZINAWZ, 2026-11-11).
            if (!openingInto)
                noteKeys();
            lap("noting");
            if (toldOnOpen.length > 0) {
                say(toldOnOpen, toldLoose);
                Report.say(`told ${JSON.stringify(toldOnOpen)}`);
                toldOnOpen = [];
            }
            Report.say(`stands ${standing.said}`);
            lap("telling");
            if (openAsked >= 0) {
                openParts = parts;
                openWorked = KeyTimes.now() - openAsked;
                afterFrame(openDrawn);
            } else if (filling) {
                fillParts = parts;
                fillWorked = KeyTimes.now() - began;
            }
        } else {
            // An ask closing with it says no, as Escape would, and is
            // never asked again.
            const closing = asked;
            asked = null;
            if (closing !== null)
                closing.no();
            screenHere = focusedScreen;
            trials.closed();
            spotsLater.restart();
            if (unlooked.length > 0 || unready.length > 0)
                readying.start();
        }
    }

    // What each source made as the launcher opened or closed, and how many
    // texts were made ready to match, said once it has settled. The first
    // opening after each start took the shell's heap from about 60 to 260
    // MB within a second (2026-10-30, 22:50), which a copy of the shell on
    // the same data didn't do, so the shell names it from what it makes.
    // Closing counts the spots made after it too.
    property var madeFrom: ({ rows: {}, texts: 0, changes: {}, when: "" })
    // How often each source has said it changed (its things, an answer or
    // its rows), kept in place so counting tells no binding. The spots a
    // closing remakes come from what changed, if anything did, so the line
    // says which did, to learn from Joel's own shell whether remaking them
    // only after a change would spare most closings (2026-10-31).
    readonly property var changesIn: ({})
    function tallies(counted) {
        const by = {};
        for (const source of sources)
            by[source.settings?.name ?? ""] = source.tally?.[counted ?? "rows"] ?? 0;
        return by;
    }
    // When the launcher was last asked to open, as its chord asks, on the
    // monotonic clock its keys are timed by, or -1 while no opening waits
    // for its frame; how long each part of that opening's own work took,
    // and all of it, from the ask until it could draw; and each of
    // the last openings, from its ask to its first frame, with the rows
    // made by then, said in the journal as it's drawn. The first since the shell
    // started makes every source's rows it reads, which is the one Joel
    // meets after each switch (Y2YBTS, 2026-11-10: the start is slow), so
    // it says so, and the keeps-pace check times it under load.
    property real openAsked: -1
    property var openParts: ({})
    property real openWorked: 0
    property var openingsTimed: []
    property int openingsDrawn: 0
    function askedToOpen() {
        if (!open && !opening) {
            openAsked = KeyTimes.now();
            openSettled = -1;
            openBegun = -1;
            openKept = false;
            openAskedWeighings = keysWeighings;
            openAskedRedone = redone();
            presentedBegin(openAsked);
            openChord = "";
        }
    }
    // When the ask that opened it was done, every binding the opening
    // changed made again with it, and when the frame that draws it began:
    // what's left of an opening's time past its own work is the shell
    // settling, then waiting for a frame, then drawing it, said apart so
    // a slow one says which held it (Joel, 2026-11-10: 6,184 ms to the
    // frame for 107 ms of its own work).
    property real openSettled: -1
    // How many times the foot had weighed rows by the ask, and by the
    // opening's first frame, or -1 before it.
    property int openAskedWeighings: 0
    property int openFirstWeighings: -1
    // And how many of the head's words were drawn to be measured, and how
    // often the rows drawn were told a place before they gave way to its
    // own, by the ask and by the first frame, or null before it.
    property var openAskedRedone: ({})
    property var openFirstRedone: null
    function redone() {
        return { measured: prompt.crumbsMade + prompt.stepsMade, toldEarly: rowsToldEarly,
            blocked: markBlocked };
    }
    property real openBegun: -1
    function askDone() {
        if (openAsked >= 0 && openSettled < 0)
            openSettled = KeyTimes.now();
    }

    // When the chord that asked for this opening ran launcher-say, on the
    // wall clock in ms, and how long the shell took to hear it: Joel feels
    // the time from his chord, and a shell busy with other work hears it
    // late, which the time from hearing it can't show (WCJQSG, 2026-11-10:
    // "to its frame" isn't what he feels; he feels how long until it's
    // usable). -1 for an ask that didn't say, as qs ipc's don't.
    property real chordAt: -1
    property real chordHeardAfter: -1
    function chordHeard(word) {
        if (open || opening || word === "") {
            chordAt = -1;
            return;
        }
        chordAt = Number(word.slice(1)) * 1000;
        chordHeardAfter = Date.now() - chordAt;
    }

    // The chord's toggle: shut while shown, and opened while shut, from
    // what's kept where it can be (opening), or else made whole first, as
    // the first opening since the shell started is, with nothing kept.
    function toggled() {
        if (opening)
            unopened();
        else if (open)
            open = false;
        else if (homeHeld)
            beginOpening();
        else
            wakeOpen();
    }

    // An opening not kept opens with every source asleep (Provider.qml),
    // so each wakes in turn as it opens (wakeEach) rather than all at once,
    // in whichever part first reads them, where the rows they make would be
    // said as that part's: Joel's first opening at 13:04 said 165 ms of
    // forgetting, which was capture's and windows' 159 rows (Y2YBTS,
    // 2026-11-10). Each sleeps on its own flag, so waking one makes only
    // its own wanted again.
    property var openWoke: ({})
    property bool wakingEach: false
    function wakeOpen() {
        for (const source of sources)
            source.asleep = true;
        wakingEach = true;
        open = true;
    }
    function wakeEach() {
        const took = {};
        if (!wakingEach)
            return took;
        wakingEach = false;
        for (const source of sources) {
            const began = KeyTimes.now();
            source.asleep = false;
            took[source.settings?.name ?? ""] = KeyTimes.now() - began;
        }
        return took;
    }
    function wokenSaid(took) {
        const most = Pacing.openingParts(took).split(", ").slice(0, 4).join(", ");
        return Object.values(took).some(ms => ms >= 1) ? ` (${most})` : "";
    }

    // Home made and kept a few seconds after the shell starts, one source
    // a turn so nothing the shell draws waits long on it, so the first
    // opening after a start draws from what's kept as every later one
    // does. Made on its chord instead, Joel's first opening after a switch
    // made 328 rows and took 1,196 ms to its frame (2026-11-10, 13:04). One
    // opened before it's done is made whole, as it was.
    property int warmed: -1
    property real warmedTook: 0
    Timer {
        interval: 6000
        running: true
        onTriggered: root.warmNext()
    }
    Timer {
        id: warmingOn

        // A frame's room between sources, so the shell's own drawing and a
        // chord pressed meanwhile come between them.
        interval: 16
        repeat: true
        onTriggered: root.warmNext()
    }
    function warmNext() {
        if (open || opening || homeHeld) {
            warmingOn.stop();
            return;
        }
        const began = KeyTimes.now();
        warmed += 1;
        if (warmed < sources.length) {
            sources[warmed].warming = true;
            sources[warmed].warming = false;
            warmedTook += KeyTimes.now() - began;
            warmingOn.start();
            return;
        }
        warmingOn.stop();
        choicesStale = true;
        freshen();
        restart("");
        keepHome();
        warmedTook += KeyTimes.now() - began;
        const rows = Object.values(tallies()).reduce((sum, each) => sum + each, 0);
        console.info(`launcher: home kept as the shell started, ${rows} rows made in`
            + ` ${Math.round(warmedTook)} ms over ${sources.length} turns`);
    }

    // Whether the opening began from what was kept, and what filled it in:
    // its first frame, or a key or a click before that frame was drawn,
    // whose fill is said once the frame is, since it's that opening's.
    property bool openKept: false
    property string openFilledBy: ""
    property real fillEarly: -1

    // A second after an opening asked for home as it was kept, rows still
    // unfilled say how they stand as being read (PlaceRow.unfilled), since
    // a reading held that long could mislead: Joel's openings under load
    // took seconds to their frame (2026-11-10).
    Timer {
        id: keptUnfilled

        interval: 1000
        onTriggered: {
            if (root.opening && !root.open)
                root.matches = PlaceRow.unfilled(root.matches);
        }
    }

    // Home as the last opening found it, and whether the closed card holds
    // it, kept (keepHome), so the next opening can draw it at once.
    property var homeKept: []
    // Whether the opening under way is into a place, by its chord (into).
    property bool openingInto: false
    property bool homeHeld: false
    // Whether the card is being put back to home, which searches nothing.
    property bool keeping: false

    // Shows the card as it's kept, taking the keys before niri hands them
    // over so the first one lands, and asks for its frame; nothing else
    // wakes until that frame is drawn (openShown).
    function beginOpening() {
        keptUnfilled.restart();
        openKept = true;
        openFilledBy = "";
        fillEarly = -1;
        fillPresented = false;
        fillUnpresented.stop();
        openFramesSized = 0;
        openFramesWaited = 0;
        openFirstDrawn = -1;
        openFirstMade = null;
        openAskedMade = tallies();
        // What the last closing left for later is put off, as the opening's
        // own work puts it off once open: its spots made half a second after
        // it, or a minute after that, came between a quick reopening's ask
        // and its first frame, making Network's rows before it (a gate of
        // #2971, 2026-11-10). Shut again unfilled, they're due again.
        spotsLater.stop();
        spotsFreshened.stop();
        leavingLater.stop();
        opening = true;
        // Asked once opening, as the card's size follows what it shows.
        openSizedAtAsk = drawnAsAsked();
        keysHeld = false;
        // Said open from the ask, written off the shell's thread, so a
        // switch waiting on the mark never restarts the shell under the
        // card, which is on the screen before it's filled in (Y2YBTS,
        // 2026-11-11).
        markOpen(true);
        prompt.take();
        if (openAsked >= 0) {
            openWorked = KeyTimes.now() - openAsked;
            openParts = { showing: openWorked };
        }
        afterFrame(openShown);
    }

    // An opening whose rows changed the card's height grows or shrinks the
    // surface, and niri answers the new size on this thread. A frame can
    // be swapped before that answer is taken, one Joel never sees: filled
    // in on it, the thread was held for seconds before the card stood at
    // its size (Joel, 2026-11-10 15:10: "focus leaves my current window
    // right when I hit the key combo", the card 7 s later). So the fill
    // waits for two frames drawn at the size asked, the second begun after
    // the first was given to niri, or 20 frames at most, so a size niri
    // never gives can't hold it. One already at its size as it was asked
    // for, as the surface keeps it while closed, moves in its first frame,
    // so that one is filled in.
    property int openFramesSized: 0
    property int openFramesWaited: 0
    property bool openSizedAtAsk: false
    // When its first frame was swapped, so its drawing is that frame's
    // own, and the frames waited for niri's size are said apart as sizing.
    property real openFirstDrawn: -1
    // The rows each source had made by then, so what's said made before
    // the frame is made before that first one, not by a source while
    // niri's size was awaited. They're counted from the ask, as what the
    // last opening woke can answer after it closed.
    property var openFirstMade: null
    property var openAskedMade: null
    function drawnAsAsked() {
        const window = card.Window.window;
        return window !== null && Math.abs(window.width - surface.implicitWidth) < 1
            && Math.abs(window.height - surface.implicitHeight) < 1;
    }

    // Its first frame drawn, the opening is timed to it; drawn at its size,
    // it's filled in.
    function openShown(begun, frame) {
        if (!opening)
            return;
        const sized = drawnAsAsked();
        openFramesSized = sized ? openFramesSized + 1 : 0;
        // Resized after all, it waits for niri's answer as any other does.
        openSizedAtAsk = openSizedAtAsk && sized;
        // The first frame at its size is the first Joel can see the card
        // in: the fill's own when the card was already at its size, else
        // the one a frame before the fill.
        if (openFramesSized === 1)
            presentedSized(frame);
        openFramesWaited += 1;
        if (openFirstDrawn < 0) {
            openFirstDrawn = KeyTimes.now();
            openFirstMade = tallies();
            openFirstWeighings = keysWeighings;
            openFirstRedone = redone();
        }
        if (openFramesSized < (openSizedAtAsk ? 1 : 2) && openFramesWaited < 20) {
            afterFrame(openShown);
            return;
        }
        // One never answered at its size is filled anyway, rather than
        // left shut, and said, since it shows at a size it wasn't asked.
        if (openFramesSized === 0)
            Report.say(`filled an opening not at its size, after ${openFramesWaited} frames`);
        openDrawn();
        // Filled once niri has presented that frame, not as it's swapped:
        // filled at once, niri presented the kept frame a median 132 ms
        // after it was drawn, against 33 ms for a letter's, since the
        // fill's work held it back, and 20 ms once the fill waited (a lab
        // of 24 kept openings a side on nested niri, 2026-11-11). Joel's
        // wait runs to the frame he sees. A frame niri hasn't said it
        // presented within `fillUnpresented` is filled anyway, and one
        // already presented, or on a surface niri can't say of, at once.
        // Niri always says a frame was presented or discarded, so that
        // wait only guards one that never answers; a key fills at once.
        if (presentedFrom >= 0) {
            fillPresented = true;
            fillUnpresented.restart();
        } else {
            filled("frame");
        }
    }
    property bool fillPresented: false
    function presentedFill(by) {
        if (!fillPresented)
            return;
        fillPresented = false;
        fillUnpresented.stop();
        filled(by);
    }
    Timer {
        id: fillUnpresented

        // Past 100 ms, every one of five openings on a builder running
        // the gate's other nested niris beside it filled unpresented,
        // niri slow to present there, though each was presented alone.
        interval: 500
        onTriggered: root.presentedFill("unpresented")
    }

    // Brings a card shown from what it kept up to now: open, every source
    // it reads wakes, home is found again where its rows stand, and its
    // drawer comes. A key or a click before its first frame is drawn does
    // this first, so it acts on what's there now, never on what was kept.
    function filled(by) {
        if (!opening || open)
            return;
        openFilledBy = by;
        const began = KeyTimes.now();
        fillFrom = feltFrom < 0 ? -1 : began - feltFrom;
        // Its sources wake a few a frame from the next (wakeNext), so home,
        // or the place a chord opened, is found again here from what each
        // made last.
        wakeQueue = wakingOrder();
        for (const source of wakeQueue)
            source.asleep = true;
        wakeSlices = [];
        waking = true;
        // A place a chord opened, opened whole now, as its chord would.
        const place = openInto;
        if (place !== "")
            into(place);
        else
            open = true;
        wakeBegan = KeyTimes.now();
        afterFrame(wakeNext);
        if (openAsked >= 0)
            fillEarly = KeyTimes.now() - began;
        else
            filledSaid(KeyTimes.now() - began);
    }

    // Whether a kept opening's sources are waking, and those still to wake,
    // in order, each `asleep` till then (Provider.qml). Each woke inside
    // `open` at once before, and Joel's kept openings stood 195 to 220 ms
    // past their first frame with the thread busy throughout: a fifth of
    // it was Quests sorting as it woke, and a third the collector (a
    // profile of 20:39 to 20:43, 2026-11-10).
    property bool waking: false
    property var wakeQueue: []
    property var wakeSlices: []
    property real wakeBegan: 0
    // Half a frame, so a slice and its frame's drawing fit in one.
    readonly property real wakeBudget: 8

    // Those home's rows read, in home's order, so the rows shown fill
    // first, then the rest; none that says what it holds itself, whose rows
    // home never wants.
    function wakingOrder() {
        const named = id => sources.find(one => one.settings.name
            === (Words.of(id).provider ?? id)) ?? null;
        const place = id => {
            const category = placeNamed(id)?.category;
            return category ? { stands: category.stands ?? null, holds: holdings(category) }
                : null;
        };
        return Wanted.wakingOrder(Words.home, place, named, sources)
            .filter(one => !one.saysAtHome);
    }

    // Wakes the next of a filling opening's sources, as many as fit in the
    // budget, then waits for the frame that draws what they made, so no
    // slice holds the thread past a frame and a key between two acts on
    // what's there then. Each slice is said by the sources whose rows end
    // it. Typed into, what's typed searches every source already, so the
    // rest wake in one.
    function wakeNext() {
        if (!open || !waking)
            return;
        const budget = typed === "" ? wakeBudget : Infinity;
        const began = KeyTimes.now();
        const slice = [];
        while (wakeQueue.length > 0 && KeyTimes.now() - began < budget) {
            const source = wakeQueue[0];
            wakeQueue = wakeQueue.slice(1);
            const at = KeyTimes.now();
            source.asleep = false;
            slice.push({ name: source.settings.name, took: KeyTimes.now() - at });
        }
        wakeSlices = wakeSlices.concat([{ took: KeyTimes.now() - began, woke: slice }]);
        if (wakeQueue.length > 0) {
            afterFrame(wakeNext);
            return;
        }
        waking = false;
        wokeSaid(KeyTimes.now() - wakeBegan);
        if (choicesStale)
            afterFrame(freshen);
    }

    // Every source still asleep woken at once, as a closing wants none.
    function wakeRest() {
        for (const source of wakeQueue)
            source.asleep = false;
        wakeQueue = [];
        waking = false;
    }

    // How the sources woke, said once the last has, with the opening it
    // filled in, so its slices, the longest and what ended each are read.
    function wokeSaid(span) {
        const slices = wakeSlices.map(one => ({ took: Math.round(one.took),
            woke: one.woke.map(each => each.name) }));
        const longest = slices.reduce((most, one) => Math.max(most, one.took), 0);
        const last = openingsTimed[openingsTimed.length - 1];
        if (last?.kept && last.slices === undefined) {
            openingsTimed = openingsTimed.slice(0, -1)
                .concat([Object.assign({}, last, { slices, longestSlice: longest })]);
        }
        const said = wakeSlices.map(one => one.woke.map(each =>
            `${each.name} ${Math.round(each.took)}`).join(", ")).join("; ");
        console.info(`launcher: woke the opening's sources in ${slices.length} slices over`
            + ` ${Math.round(span)} ms, the longest ${longest} ms (${said})`);
    }

    // Shut again before it was filled in, as a second press of the chord
    // does: nothing woke for it, so nothing has to settle.
    function unopened() {
        opening = false;
        openInto = "";
        spotsLater.restart();
        if (unlooked.length > 0 || unready.length > 0)
            readying.start();
        openAsked = -1;
        markOpen(false);
        afterFramesDue();
    }

    // Once the closed card is clear, it's put back to home as the last
    // opening found it, or as it was left there, each row as it stood, so
    // the next opening draws it with nothing made and nothing woken. Every
    // row is read again as that opening fills in, landing where it stands,
    // and only what changed is drawn anew.
    // A kept card is drawn at the height it was kept at, never laid out
    // again as it opens, so a screen that shrinks while it's shut (a new
    // scale, another output) keeps it again at as many rows as now fit;
    // kept at 1 and opened at 1.5, its foot ran 30 px off the bottom.
    onRowsChanged: {
        if (homeHeld)
            keepHome();
    }

    function keepHome() {
        if (open || opening || parting !== "")
            return;
        const home = held.length === 0 && typed === "" && matches.length > 0 ? matches
            : homeKept;
        if (home.length === 0)
            return;
        // Home found while shut, as the shell starts (warmNext), is kept as
        // an opening's find is, so the first chord after a restart goes into
        // its place alone rather than finding home again on its way.
        homeKept = home;
        keeping = true;
        try {
            held = [];
            homeDepth = 0;
            listing = false;
            listedInDrawer = false;
            steadying = false;
            typed = "";
            candidates = [];
            prompt.text = "";
            matches = home;
            current = 0;
            tall = rows * rowTall + (matches.slice(0, rows).some(row => row.gap) ? apart : 0);
        } finally {
            keeping = false;
        }
        homeHeld = true;
    }

    // The rows the card shows, by what each is, what each says at its
    // right, and where the cursor is, for the check that rows filled in
    // land where the kept ones stood.
    function rowsSaid() {
        const seen = matches.slice(0, rows);
        return { keys: seen.map(row => pairedId(row)), sides: seen.map(row => row.side ?? ""),
            at: current };
    }

    // What filling an opening in took, said once it's done, with the rows
    // it shows then beside those its first frame showed.
    // Its own work's parts and all of it, and the rest, every binding and
    // source that woke as it opened, said apart, so a fill that holds the
    // first keys says which held them.
    property var fillParts: ({})
    property real fillWorked: 0
    // When the fill began, from the chord or the ask, as `presented` is.
    property real fillFrom: -1
    function filledSaid(took) {
        const last = openingsTimed[openingsTimed.length - 1];
        if (!last || !last.kept || last.filling !== undefined)
            return;
        const worked = Math.round(fillWorked);
        const filled = { filling: Math.round(took), fillWorked: worked, fillParts: fillParts,
            fillFrom: Math.round(fillFrom), verbs: openVerbsWeighed, filled: rowsSaid(), filledBy: openFilledBy, woke: openWoke,
            madeBy: madeBy() };
        openingsTimed = openingsTimed.slice(0, -1).concat([Object.assign({}, last, filled)]);
        const made = Object.values(filled.madeBy).reduce((sum, rows) => sum + rows, 0);
        console.info(`launcher: filled the opening in ${filled.filling} ms, ${worked} ms`
            + ` working (${partsSaid(fillParts)}) and the rest waking; ${made} rows made`
            + ` (${Pacing.openingParts(filled.madeBy)})`);
    }
    // An opening's parts, with waking's split by the source each woke.
    function partsSaid(parts) {
        return Pacing.openingParts(parts).replace(/\bwaking \d+/,
            part => part + wokenSaid(openWoke));
    }
    // The rows each source has made since the launcher last opened or closed.
    function madeBy() {
        const now = tallies();
        const by = {};
        for (const name of Object.keys(now)) {
            const rows = now[name] - (madeFrom.rows[name] ?? 0);
            if (rows > 0)
                by[name] = rows;
        }
        return by;
    }

    function openDrawn(beganAt, frame) {
        const asked = openAsked;
        openAsked = -1;
        if ((!open && !opening) || asked < 0)
            return;
        const sized = KeyTimes.now();
        const drawn = openFirstDrawn < 0 ? sized : openFirstDrawn;
        const took = Math.round(sized - asked);
        // Joel's wait runs from his chord to the frame niri shows at the
        // card's size, so it's felt from here, not from the first frame.
        const felt = chordAt < 0 ? -1 : Math.round(Date.now() - chordAt);
        feltBegin(felt < 0 ? asked : sized - felt, felt >= 0);
        const heard = Math.round(chordHeardAfter);
        chordAt = -1;
        const parts = partsSaid(openParts);
        const settled = openSettled < 0 ? asked + openWorked : openSettled;
        const begun = openBegun < 0 ? drawn : openBegun;
        const after = { settling: Math.round(settled - asked - openWorked),
            waiting: Math.round(begun - settled), drawing: Math.round(drawn - begun),
            sizing: Math.round(sized - drawn) };
        const worked = Math.round(openWorked);
        const first = openingsDrawn === 0;
        openingsDrawn += 1;
        const now = openFirstMade ?? tallies();
        const from = openFirstMade === null ? madeFrom.rows : openAskedMade;
        openFirstMade = null;
        openFirstDrawn = -1;
        const made = Object.keys(now).map(name => ({ name, rows: now[name]
            - (from[name] ?? 0) })).filter(each => each.rows > 0)
            .sort((a, b) => b.rows - a.rows || a.name.localeCompare(b.name));
        const rows = made.reduce((sum, each) => sum + each.rows, 0);
        const most = made.slice(0, 4).map(each => `${each.name} ${each.rows}`).join(", ");
        const kept = openKept;
        const weighings = (openFirstWeighings < 0 ? keysWeighings : openFirstWeighings)
            - openAskedWeighings;
        openFirstWeighings = -1;
        const redoneBy = openFirstRedone ?? redone();
        const measured = redoneBy.measured - (openAskedRedone.measured ?? 0);
        const toldEarly = redoneBy.toldEarly - (openAskedRedone.toldEarly ?? 0);
        const blocked = redone().blocked - (openAskedRedone.blocked ?? 0);
        openFirstRedone = null;
        // How long after the ask the mark said open, or -1 while it hasn't.
        const marked = markSays === "open" && markedOpenAt >= asked
            ? Math.round(markedOpenAt - asked) : -1;
        // Numbered, since only the last 20 are kept, so a check opening more
        // than that still sees each new one come.
        const timed = { nth: openingsDrawn, took, worked, parts: openParts, after, rows, weighings,
            marked, measured, toldEarly, blocked, made: most, verbs: kept ? 0 : openVerbsWeighed, first,
            kept, felt,
            frames: kept ? openFramesWaited : 1, woke: kept ? {} : openWoke,
            into: openChord };
        if (kept)
            timed.shown = rowsSaid();
        openingsTimed = openingsTimed.concat([timed]).slice(-20);
        if (kept && fillEarly >= 0) {
            filledSaid(fillEarly);
            fillEarly = -1;
        }
        console.info(`launcher: opening took ${took} ms to its frame, ${worked} ms working`
            + ` (${parts}), then ${after.settling} settling, ${after.waiting} waiting for a`
            + ` frame and ${after.drawing} drawing it, ${rows} rows made (${most})`
            + (kept ? `, drawn from what was kept, at its size after ${openFramesWaited} frames`
                + ` and ${after.sizing} ms` : "")
            + (first ? ", the first since the shell started" : "")
            + (openChord === "" ? "" : `, into ${openChord}`)
            + (felt < 0 ? "" : `; ${felt} ms from the chord, heard ${heard} ms after it`));
        // Once its record is made, so niri's answer, which may already
        // have come, is kept with this opening.
        presentedDrawn(feltFrom, feltFromChord, frame);
    }

    // How long Joel waited for an opening to be usable, from his chord (or
    // the ask, for one that didn't say when it ran) to the first 100 ms in
    // which the shell's thread answered a 16 ms tick on time, and the
    // longest stall between, which its first frame can't say: his 15:08
    // openings logged 437 ms to their frame and showed the launcher 7 s
    // after his chord, when the thread came free (Joel, 2026-11-10: "It
    // takes 7 seconds for me to see the launcher window"). It ticks only
    // from an opening's first frame until then, or 15 s, never while shut.
    // When niri showed Joel the opening, from his chord (or the ask): the
    // first frame niri presented that was the opening's first frame or
    // after it, as frame-presentation.patch says (Joel, 2026-11-10: "we
    // need 50ms launch ... that is time from trigger to visible on the
    // screen"). Each frame from the ask asks again, so the frame the
    // opening drew is told even when the thread is busy after it; the
    // answer is niri's own time, however late the thread reads it. It asks
    // only while an opening waits to be shown, never while shut.
    property real presentedFrom: -1
    property bool presentedFromChord: false
    property int presentedFrame: -1
    // When niri presented the opening's frame, held until the opening is
    // timed and its record made, since a frame drawn at the card's size
    // can be presented before the fill's second frame times it.
    property real presentedAt: -1
    property bool presentedTimed: false
    // Whether the journal has said niri gives no presentation times, said
    // once a shell: without them no opening, key or change says when niri
    // showed it, and nothing fails silently (Y2YBTS, 2026-11-11).
    property bool presentedUnavailableSaid: false
    function presentedAvailable() {
        if (surface.FramePresentation.available)
            return true;
        if (!presentedUnavailableSaid) {
            presentedUnavailableSaid = true;
            console.warn("launcher: niri gives no presentation times, so no opening, key "
                + "or change says when niri showed it, only when the shell drew it");
        }
        return false;
    }
    function presentedBegin(from) {
        if (!presentedAvailable())
            return;
        presentedFrom = from;
        presentedFromChord = false;
        presentedFrame = -1;
        presentedAt = -1;
        presentedTimed = false;
        surface.FramePresentation.ask();
    }
    // The frame begun with the card first drawn at its size, numbered as
    // the window counts them.
    function presentedSized(frame) {
        if (presentedFrom >= 0 && presentedFrame < 0 && typeof frame === "number")
            presentedFrame = frame;
    }
    function presentedDrawn(from, fromChord, frame) {
        if (presentedFrom < 0)
            return;
        presentedFrom = from;
        presentedFromChord = fromChord;
        presentedTimed = true;
        if (presentedFrame < 0)
            presentedFrame = typeof frame === "number" ? frame
                : surface.FramePresentation.framesBegun;
        if (presentedAt >= 0)
            presentedSaid(Math.round(presentedAt - presentedFrom));
    }
    function framePresented(frame, at) {
        presentedDue(frame, at);
        if (presentedFrom < 0)
            return;
        // An opening is shown from when its card is asked up, before it's
        // open, so only a card put down has nothing left to present.
        if (!cardUp) {
            presentedSaid(-1);
            return;
        }
        if (presentedFrame < 0 || frame < presentedFrame) {
            surface.FramePresentation.ask();
            return;
        }
        if (!presentedTimed) {
            presentedAt = at;
            return;
        }
        presentedSaid(Math.round(at - presentedFrom));
    }
    function frameDiscarded(frame) {
        if (presentedWaiting.length > 0) {
            if (cardUp)
                surface.FramePresentation.ask();
            else
                presentedWaiting = [];
        }
        if (presentedFrom < 0)
            return;
        if (cardUp)
            surface.FramePresentation.ask();
        else
            presentedSaid(-1);
    }
    function presentedSaid(ms) {
        const from = presentedFromChord ? "the chord" : "the ask";
        presentedFrom = -1;
        const last = openingsTimed[openingsTimed.length - 1];
        if (last)
            openingsTimed = openingsTimed.slice(0, -1)
                .concat([Object.assign({}, last, { presented: ms })]);
        console.info(ms >= 0
            ? `launcher: opening presented ${ms} ms from ${from}`
            : "launcher: opening closed before niri presented it");
        presentedFill("presented");
    }

    // What a key changed, timed as the opening is, from its press (or,
    // for a key a program typed with no time, from when it was heard) to
    // the frame niri presented with it: each waits for the frame about to
    // begin, numbered as the window counts them, and is told by the first
    // presented at or after it, a frame niri dropped being shown by the
    // next. Asked for only while a key's frame waits, never while shut.
    property var presentedWaiting: []
    function presentedAwait(from, said) {
        if (!presentedAvailable())
            return;
        presentedWaiting = presentedWaiting.concat([{ from, said,
            frame: surface.FramePresentation.framesBegun + 1 }]);
        surface.FramePresentation.ask();
    }
    function presentedDue(frame, at) {
        if (presentedWaiting.length === 0)
            return;
        const due = presentedWaiting.filter(one => frame >= one.frame);
        presentedWaiting = presentedWaiting.filter(one => frame < one.frame);
        for (const one of due)
            one.said(at);
        if (presentedWaiting.length > 0)
            surface.FramePresentation.ask();
    }

    // Each key presented since the launcher opened, from its press and
    // when niri presented it, on the monotonic clock, for the check that
    // sets it against niri's screen; said in the journal as it closes, with
    // when the shell heard it and its frame began, to say where it went.
    property var keysPresented: []
    function keyPresented(from, at, key) {
        const one = { from, at, heard: key.heard, begun: key.begun };
        keysPresented = keysPresented.concat([one]).slice(-keysKept);
        // A key that set no glide going stood where it went at its first
        // frame, so it landed there.
        if (!keysGlided.includes(from))
            keysLanded = keysLanded.concat([one]).slice(-keysKept);
    }

    // Each key presented where the selection, and the list's scroll with
    // it, came to rest: a key that set a glide going counts as seen only at
    // the first frame niri presented once the glide ended, since the
    // highlight still moving isn't yet on its row (VKCF3Z, 2026-11-10).
    // Keys pressed while one glide runs all land as it ends, since none of
    // their rows was stood on before.
    property var keysLanded: []
    // The presses whose glide runs now, and every press that glided since
    // the launcher opened, by their press times.
    property var keysGliding: []
    property var keysGlided: []
    function glideBegun() {
        const pressed = movingFrom > 0 ? movingFrom : KeyTimes.pressed();
        if (pressed <= 0 || keysGliding.includes(pressed))
            return;
        // A held arrow keeps one glide going, so its presses are kept as
        // many as the rest.
        keysGliding = keysGliding.concat([pressed]).slice(-keysKept);
        keysGlided = keysGlided.concat([pressed]).slice(-keysKept);
    }
    function glideEnded() {
        const landing = keysGliding;
        keysGliding = [];
        if (landing.length === 0)
            return;
        presentedAwait(landing[0], at => {
            keysLanded = keysLanded.concat(landing.map(from => ({ from, at })))
                .slice(-keysKept);
        });
    }

    // A daemon's change shown since the launcher opened, by its place, from
    // the earliest line its draw showed to when niri presented it, for the
    // check that sets it against niri's screen; said in the journal by
    // place as it closes. A change drawn where nothing shows it is let go,
    // since no frame carries it.
    property var updatesWaiting: ({})
    property var updatesPresented: []
    function updateDrawn(place, shown) {
        const from = updatesWaiting[place];
        if (from === undefined)
            return;
        updatesWaiting = Presented.drawn(updatesWaiting, place);
        if (shown && open)
            presentedAwait(from, at => {
                updatesPresented = updatesPresented.concat([{ place, from, at }])
                    .slice(-keysKept);
            });
    }

    function keysPresentedSaid() {
        // Where a key's time went is said of its first frame alone, since a
        // key that glided landed frames later.
        const first = Presented.keysSaid("presented", keysPresented, true);
        const landed = Presented.keysSaid("landed", keysLanded, false);
        const updates = Presented.changesSaid(updatesPresented);
        keysPresented = [];
        keysLanded = [];
        keysGliding = [];
        keysGlided = [];
        updatesPresented = [];
        updatesWaiting = {};
        // A frame shut takes nothing a key changed to the screen.
        presentedWaiting = [];
        for (const said of [first, landed].filter(line => line !== "").concat(updates))
            console.info(said);
    }

    // What a key changed in a place, presented: said when slow enough to
    // feel, as its frame is (actsDrawn).
    function actPresented(one, at) {
        const took = Math.round(at - one.heard);
        if (took > 50)
            console.info(`launcher: what a key changed in ${one.source.settings.name} `
                + `was presented ${took} ms from the key`);
    }

    readonly property var feltPace: ({ every: 16, within: 17, calmFor: 100 })
    property var feltTicks: Pacing.feltBegun(0)
    property real feltFrom: -1
    property bool feltFromChord: false
    function feltBegin(from, fromChord) {
        if (feltTicking.running)
            feltSaid();
        feltTicks = Pacing.feltBegun(KeyTimes.now());
        feltFrom = from;
        feltFromChord = fromChord;
        feltTicking.start();
    }
    function feltTicked() {
        const now = KeyTimes.now();
        feltTicks = Pacing.feltTicked(feltTicks, now, feltPace);
        if (feltTicks.usable >= 0 || !cardUp || now - feltFrom > feltGiveUp)
            feltSaid();
    }
    readonly property int feltGiveUp: 15000
    function feltSaid() {
        feltTicking.stop();
        const usable = feltTicks.usable < 0 ? -1 : Math.round(feltTicks.usable - feltFrom);
        const stalled = Math.round(feltTicks.stall);
        const last = openingsTimed[openingsTimed.length - 1];
        if (last)
            openingsTimed = openingsTimed.slice(0, -1)
                .concat([Object.assign({}, last, { usable, stalled })]);
        const from = feltFromChord ? "the chord" : "the ask";
        const waited = Math.round(KeyTimes.now() - feltFrom);
        console.info(usable >= 0
            ? `launcher: opening usable ${usable} ms from ${from}, its longest stall ${stalled} ms`
            : `launcher: opening not usable ${waited} ms from ${from}, its longest stall`
                + ` ${stalled} ms` + (cardUp ? "" : ", closed first"));
    }
    Timer {
        id: feltTicking

        interval: root.feltPace.every
        repeat: true
        onTriggered: root.feltTicked()
    }

    function madeSince(when) {
        madeFrom = { rows: tallies(), texts: Fuzzy.keptSoFar(), changes: Object.assign({}, changesIn),
            when: when };
        madeSettling.restart();
    }
    Timer {
        id: madeSettling

        interval: 3000
        onTriggered: root.madeSaid()
    }
    function madeSaid() {
        const now = tallies();
        const made = Object.keys(now).map(name => ({ name, rows: now[name]
            - (madeFrom.rows[name] ?? 0) })).filter(each => each.rows > 0)
            .sort((a, b) => b.rows - a.rows || a.name.localeCompare(b.name));
        const total = made.reduce((sum, each) => sum + each.rows, 0);
        const texts = Fuzzy.keptSoFar() - madeFrom.texts;
        const most = made.slice(0, 8).map(each => `${each.name} ${each.rows}`).join(", ");
        const changed = Object.keys(changesIn).map(name => ({ name, times: changesIn[name]
            - (madeFrom.changes[name] ?? 0) })).filter(each => each.times > 0)
            .sort((a, b) => b.times - a.times || a.name.localeCompare(b.name));
        const times = changed.reduce((sum, each) => sum + each.times, 0);
        const which = changed.slice(0, 8).map(each => `${each.name} ${each.times}`).join(", ");
        console.info(`launcher: ${madeFrom.when} made ${total} rows (${most}) and ${texts} texts,`
            + ` after ${times} changes (${which})`);
    }

    // What's inside things is read again a moment after the launcher
    // closes, from what it just showed, so the next opening has it
    // ready. Read on the third letter instead, as it was, it held that
    // key 115 to 250 ms once every opening (launcher-trials.json,
    // 2026-10-26: "wha" 252 ms, "cpu" 248, "key" 211).
    Timer {
        id: spotsLater

        interval: 500
        onTriggered: root.spotsMadeClosed()
    }

    // And a few seconds after the shell starts, once its devices have been
    // heard, since every switch restarts it and its first opening read them
    // on its third letter: 236 and 183 ms on two keys (launcher-trials.json,
    // 2026-10-29 and 30). One opened sooner still reads them on that letter.
    Timer {
        interval: 5000
        running: true
        onTriggered: {
            if (!root.spotsMade)
                root.spotsMadeClosed();
            root.textsMadeClosed();
        }
    }

    // Never as a kept opening waits for its frame either, which beginOpening
    // puts spotsLater off for, but the start's timer above came due in it:
    // every closing reopened within half a second leaves no spots made, so
    // it made them between a reopening's ask and its frame (2026-11-10).
    function spotsMadeClosed() {
        if (open || opening)
            return;
        freshen();
        spots = spotsFrom();
        spotsMade = true;
        spotsFreshened.restart();
        Report.say(`spots made while closed ${JSON.stringify(spotsSaid)}`);
    }

    // The figures of spots whose make held (spotsFrom), made again a minute
    // after a closing, if the launcher is still closed, so a level moved
    // reads as it is at the next opening, much as a remaking on every
    // closing had it, and never as Joel goes back to what he was doing.
    // Said as the closing is (madeSaid), when it made any.
    Timer {
        id: spotsFreshened

        interval: 60000
        onTriggered: {
            if (root.open || !root.spotsMade)
                return;
            root.madeSince("freshening");
            root.spots = root.spotsFrom(true);
            if (root.spotsSaid.made.length === 0)
                madeSettling.stop();
            Report.say(`spots freshened while closed ${JSON.stringify(root.spotsSaid)}`);
        }
    }

    // So is every text a collection's search looks through, which each
    // makes ready to match when first searched, and which a restart
    // forgets: each switch's first search of two letters made them, up to
    // 968 ms of "ol", nearly all People, Quests and notices
    // (launcher-trials.json, 2026-10-29 and 30). They're made a
    // collection a turn, then a few milliseconds at a time, so nothing else
    // the shell draws waits on them, and left while it's open or opening,
    // for the search to make what it needs. Looked through all at once, as
    // the start's timer came due over Joel's first opening, notices made
    // 942 rows and quests 1,084 in the 3 s after its frame (2RCBOJ,
    // 2026-11-10, 21:29).
    property var unlooked: []
    property var unready: []
    property var lookedTook: ({})

    function textsMadeClosed() {
        // Not one searched only once stepped into, as Emoji is, whose
        // thousands would be kept for a search that may never come.
        unlooked = collections.filter(one => one.collection.searched !== false);
        unready = [];
        lookedTook = {};
        if (!(open || opening))
            readying.start();
    }

    // One collection's texts gathered, timed as each source's turn is.
    function lookThroughNext() {
        const source = unlooked.shift();
        const began = KeyTimes.now();
        for (const row of source.looksThrough())
            unready.push({ from: source, row: row });
        lookedTook[source.settings?.name ?? ""] = KeyTimes.now() - began;
        if (unlooked.length === 0)
            console.info("launcher: looked through the searched collections a turn each"
                + wokenSaid(lookedTook));
    }

    Timer {
        id: readying

        interval: 16
        repeat: true
        onTriggered: root.readySome()
    }

    function readySome() {
        if (open || opening) {
            readying.stop();
            return;
        }
        if (unlooked.length > 0) {
            lookThroughNext();
            return;
        }
        const until = Date.now() + 4;
        while (unready.length > 0 && Date.now() < until) {
            const next = unready.pop();
            next.from.readyTexts(next.row);
        }
        if (unready.length === 0) {
            readying.stop();
            Report.say("texts made ready while closed");
        }
    }

    // The screen the keys are on, followed while the launcher is closed so
    // its surface is already there when Super+Space comes, and kept while
    // it's open so it never jumps from under Joel's eyes.
    // With no screen left, Qt lists a stand-in of its own, which has no name
    // and can hold no surface, so it's never one.
    readonly property var focusedScreen: {
        void Niri.revision;
        void Quickshell.screens;
        return screenFocused();
    }

    // The focused screen as the screens stand now, for a handler that runs
    // as they change, before focusedScreen's binding may have heard: read
    // from the binding there, a screen just unplugged came back as the one
    // to make the surface on, and the launcher showed on none.
    function screenFocused() {
        const workspace = Niri.workspace();
        const real = Quickshell.screens.filter(screen => screen.name !== "");
        return real.find(screen => workspace && screen.name === workspace.output)
            ?? real[0] ?? null;
    }
    property var screenHere: focusedScreen

    onFocusedScreenChanged: {
        if (!open)
            screenHere = focusedScreen;
    }

    // A screen coming or going (a dock, the lid, a monitor) can leave the
    // surface nowhere. With no screen left, Qt moves the window onto a
    // stand-in that can hold no layer surface; when a screen comes back, Qt
    // moves the window onto it by itself, so Quickshell sees no change of
    // screen and never makes the surface again, and Super+Space finds
    // nothing. So once the screens have changed, the launcher closes and its
    // surface is made again on the screen the keys are on, as the margin's
    // are made anew for each screen. A screen coming or going beside the one
    // the surface is drawn on moves nothing, so that surface is kept, open
    // or not: remade, a Super+Space pressed just before the shell heard a
    // screen plugged in opened the launcher and closed it at once. With no
    // screen at all, there's nowhere to make it: niri would refuse it and
    // Quickshell warn, so it waits unmade for the next screen, which changes
    // the screens again.
    Connections {
        target: Quickshell

        // A screen still listed is remade from once the change settles. The
        // one the surface is on, gone, is let go at once: until then every
        // binding that reads its size (the card's width, height and place)
        // reads a screen Qt has freed, and Quickshell warns it's dangling
        // whenever one runs first, as a displays refresh did (the net's
        // quarantine of vm-screens-keep-the-launcher, 2026-11-05).
        function onScreensChanged() {
            if (root.screenHere !== null && !Quickshell.screens.includes(root.screenHere))
                root.remake();
            else
                Qt.callLater(root.remake);
        }
    }

    function remake() {
        const real = Quickshell.screens.filter(screen => screen.name !== "");
        const kept = surface.visible && screenHere !== null && real.includes(screenHere);
        // Said for the screens test, which waits on the shell having heard
        // the screens change before it looks at what that did.
        const done = kept ? "kept" : "remade";
        Report.say(`launcher heard ${Quickshell.screens.length} screens, ${done}`);
        if (kept)
            return;
        open = false;
        screenHere = screenFocused();
        surface.visible = false;
        surface.visible = screenHere !== null;
    }

    // Nothing at all: the pointer's reach and niri's blur while closed.
    Region {
        id: nowhere
    }

    // The card and the drawer out beside it, for the pointer and the blur.
    Region {
        id: cardAndDrawer
        regions: [card.area, drawerCard.area]
    }

    // Mapped all along, so opening makes no surface and draws no first
    // frame before niri hands over the keys. It keeps the card's size while
    // closed, so an opening moves it rather than growing it: grown from a
    // pixel, each opening waited on niri's configure and new buffers before
    // a frame at its size, 100 to 150 ms past its own work on Joel's laptop
    // (2026-11-10). Closed, it sits up and left of the screen but for its
    // last pixel, since niri draws a surface kept out of screencasts
    // (config.kdl) as a black box the size of the whole surface, shown or
    // not, so a screencast shows that pixel as the one-pixel surface did.
    // Wholly off the screen, niri answered its frames once a second, and Qt,
    // waiting on each, took the surface for hidden and drew it again as it
    // was answered, a frame a second for as long as it stayed closed.
    PanelWindow {
        id: surface

        Counted { name: "launcher" }
        visible: true
        screen: root.screenHere
        // Placed from the left, so the card sits where cardLeft says with
        // the drawer's room on its right, sized once for the screen.
        anchors.top: true
        anchors.left: true
        // Quickshell's type files can't place margins, so qmllint can't.
        margins.top: root.cardUp ? root.cardTop - card.reach // qmllint disable unqualified
            : 1 - implicitHeight
        margins.left: root.cardUp ? root.cardLeft - card.reach // qmllint disable unqualified
            : 1 - implicitWidth
        implicitWidth: root.width + root.drawerBeside + 2 * card.reach
        implicitHeight: card.height + 2 * card.reach
        exclusionMode: ExclusionMode.Ignore
        color: "transparent"
        mask: !root.cardUp ? nowhere : root.drawerOut ? cardAndDrawer : card.area
        BackgroundEffect.blurRegion: !root.cardUp ? nowhere
            : root.drawerOut ? cardAndDrawer : card.area
        WlrLayershell.layer: WlrLayer.Overlay
        // Asked for on demand once niri gives them so (Niri.asksForKeys), so
        // the newest surface Joel opened has the keys and a click on a
        // window takes them, closing the launcher (keysMoved).
        WlrLayershell.keyboardFocus: root.cardUp && !root.lettingGo ? Niri.asksForKeys
            : WlrKeyboardFocus.None
        WlrLayershell.namespace: "launcher"
        // When niri showed an opening's frames (presentedBegin).
        FramePresentation.onPresented: (frame, at) => root.framePresented(frame, at)
        FramePresentation.onDiscarded: frame => root.frameDiscarded(frame)

        Connections {
            target: card.Window.window
            enabled: root.cardUp

            function onActiveChanged() {
                root.keysMoved();
            }
        }

        Connections {
            target: card.Window.window
            enabled: root.keysDrawing.waiting.length + root.keysDrawing.framing.length > 0

            function onAfterAnimating() {
                if (root.keysDrawing.waiting.length > 0)
                    root.keysFrameBegun();
            }

            function onAfterRendering() {
                root.keysFrameRendered();
            }

            function onFrameSwapped() {
                root.keysFrameSwapped();
            }
        }

        // An act's refresh is drawn by a frame begun after it, so a swap
        // of one already under way when it ran isn't taken for it, and an
        // act refreshed while a frame is drawn holds back none in that
        // frame (launcher/Pacing.js's frameSwapped).
        Connections {
            target: card.Window.window
            enabled: root.actsDrawing.waiting.length + root.actsDrawing.framing.length > 0

            function onAfterAnimating() {
                if (root.actsDrawing.waiting.length === 0)
                    return;
                for (const one of root.actsDrawing.waiting.filter(each => each.heard
                    === root.keyHeardAt))
                    root.presentedAwait(one.heard, at => root.actPresented(one, at));
                root.actsDrawing = Pacing.frameBegun(root.actsDrawing);
            }

            function onFrameSwapped() {
                const swapped = Pacing.frameSwapped(root.actsDrawing);
                if (swapped.shown.length === 0)
                    return;
                root.actsDrawing = swapped.acts;
                root.actsDrawn(swapped.shown);
            }
        }

        Connections {
            target: card.Window.window
            enabled: root.afterFrames.length > 0 || root.afterFramesDrawing.length > 0

            // Each is told when its frame began, so a drawer's time says how
            // much of it waited for its frame and how much drew it.
            function onAfterAnimating() {
                const begun = KeyTimes.now();
                // The frame about to begin, as FramePresentation will number it.
                const frame = surface.FramePresentation.framesBegun + 1;
                if ((root.afterFrames.includes(root.openDrawn)
                        || root.afterFrames.includes(root.openShown)) && root.openBegun < 0)
                    root.openBegun = begun;
                root.afterFramesDrawing = root.afterFramesDrawing
                    .concat(root.afterFrames.map(then => () => then(begun, frame)));
                root.afterFrames = [];
            }

            function onFrameSwapped() {
                const due = root.afterFramesDrawing;
                root.afterFramesDrawing = [];
                for (const then of due)
                    then();
                if (root.afterFrames.length > 0)
                    card.Window.window?.update();
            }
        }

        // Each frame while open, so how smoothly it moves is kept with the
        // opening (Trials.qml). Its fade as it closes comes after the
        // opening is kept, so isn't counted.
        Connections {
            target: card.Window.window
            enabled: root.open

            function onFrameSwapped() {
                trials.drew(Date.now(), card.Screen.refreshRate);
            }
        }

        Connections {
            target: card.Window.window
            enabled: root.framesCounted

            function onFrameSwapped() {
                root.framesDrawn += 1;
            }
        }

        Card {
            id: card
            x: reach
            y: reach
            width: root.width
            // Laid in bands, its head, rows and foot each reaching from the
            // frame's inside edge and centring its own line.
            bands: true
            // Its frame says the keys are here, unless the drawer holds them.
            keys: !QueryKeys.paged(prompt.mode)
            // Clear rather than hidden while closed, since a hidden field
            // can't hold the focus, and a key that came before the card was
            // shown would be lost.
            opacity: root.cardUp || root.parting !== "" ? 1 : 0
            // Clear, it's put back to home for the next opening, once the
            // closing's own handler is done (keepHome checks it's still shut).
            onOpacityChanged: {
                if (opacity === 0)
                    Qt.callLater(root.keepHome);
            }

            // A click before an opening's first frame is drawn fills it in
            // first, then reaches the row beneath as it would have. It fills
            // the bands, not the card's content: the content is as tall as
            // what it holds, this included, so filling that held the card at
            // its tallest, and a card kept on a screen that then shrank ran
            // off its foot (vm-desktop-says-what-happens, 2026-11-10).
            MouseArea {
                anchors.fill: cardBands
                z: 1
                enabled: root.opening
                onPressed: mouse => {
                    root.filled("click");
                    mouse.accepted = false;
                }
            }

            // Only a parting word's card leaves on the curve; anything
            // else closes on the next frame, as it opens.
            Behavior on opacity {
                enabled: root.fading
                NumberAnimation {
                    duration: Theme.motion.short
                    easing.type: Easing.BezierSpline
                    easing.bezierCurve: Theme.motion.curve
                }
            }

            // From the frame's inside edge to its other, since every band
            // reaches it, each standing its ink on the card's columns.
            Column {
                id: cardBands

                x: -card.inset
                width: card.contentWidth + 2 * card.inset

                // The faces whose capitals set the rows' and the foot's
                // lines (root.rowCaps, root.footBase): TextMetrics rather
                // than a FontMetrics' call, which a binding asks once,
                // before the face has loaded, and reads small ever after
                // (HVD6O3, 2026-11-05).
                TextMetrics {
                    id: nameCaps
                    text: "H"
                    font.family: Theme.level.name.family
                    font.pointSize: Theme.level.name.pointSize
                }

                TextMetrics {
                    id: footCaps
                    text: "H"
                    font.family: Theme.level.note.family
                    font.pointSize: Theme.level.note.pointSize
                }

                // What's typed, in the band from the frame's inside to its
                // hairline, its line centred there by eye.
                Item {
                    width: parent.width
                    height: root.headTall
                    opacity: root.wordsShown

                    Prompt {
                        id: prompt
                        // It gives the head to the command line while that's
                        // open, and to what Return on it said until the key
                        // after, as Vim's / and : are one line, keeping the
                        // keys for that key: unseen, not gone.
                        opacity: root.commanding === null && !root.commandSays ? 1 : 0
                        // From the drawer, :{n} and :$ go to its lines,
                        // as they do in his Neovim's window; from visual,
                        // the line takes the rows chosen.
                        onCommanded: range => root.commandLine(root.over
                            ?? (QueryKeys.paged(prompt.mode)
                                ? { go: to => drawerCard.go(to === "$" ? "last" : "first",
                                    to === "$" ? 0 : to) }
                                : root.visual ? { range, things: root.visualRows() }
                                : { range }))
                        steady: true
                        // Vim's / line, opening on the find, but for a
                        // secret, which has no modes, as Vim's inputsecret()
                        // has none (VKCF3Z's vi-paradigm spec, Insert).
                        modal: !prompt.secret
                        // A step holding words, as a save's name, or a row
                        // asking for them, is typed in insert.
                        words: root.holdsWords || !!root.asking
                        editsWords: root.holdsWords && root.selected?.names === true
                        awake: root.open
                        // Shut, it takes no key niri sent before it heard
                        // the launcher let them go (Prompt's taking).
                        taking: root.cardUp
                        fade: Options.caret.fade
                        stopsAfter: Options.caret.stopsAfter
                        wait: Options.keyTiming.wait
                        // From the frame, its own inset inside, at the card's
                        // inset over the gutter's numbers, as Vim's / and :
                        // lines start at column 0, left of its number column:
                        // the line is the screen's, the gutter the buffer's,
                        // and a long ask keeps its room (VKCF3Z, 2026-11-05).
                        // It fills the head's band, so its own centring is
                        // the band's.
                        width: parent.width
                        height: parent.height
                        // A thing held leads the line with its own glyph.
                        readonly property var thing: root.held.length > 0
                            ? root.held[0].row : null
                        // A secret is typed beside the padlock.
                        secret: root.asking !== null && root.asking.secret === true
                        // The line stands on the rows' columns: a held
                        // thing's glyph, or the padlock, over their glyphs,
                        // where the find's / and the : line's : take its
                        // cell while they last, and what's typed or a
                        // place's name over their names, so nothing moves
                        // between normal, / and : (VKCF3Z, 2026-11-06, on
                        // Joel's "when in normal mode, it shouldn't have a
                        // / at the front"). An ask
                        // leads with nothing, since the line isn't a find
                        // while it stands, its words where the find's are.
                        readonly property bool sigilLeads: !secret && !words && root.asked === null
                        // Home is a place like any other, so its head says
                        // where Joel is as a place's does: its glyph and its
                        // Title, typing after them (VKCF3Z, 2026-11-06: a
                        // bare band said nothing at the top of the screen
                        // he sees most).
                        readonly property bool home: sigilLeads && root.held.length === 0
                        // Nothing in the gutter's column in any mode: the
                        // find's / takes the place glyph's cell while it
                        // lasts, as Vim keeps one cell for its mode, so
                        // normal never shows a bare cell the find fills
                        // (VKCF3Z, 2026-11-06, for Joel's "an extra space
                        // in front of the launcher heading when in command
                        // mode that we don't have in find").
                        glyph: ""
                        cellMark: sigilLeads && mode === "find" ? "/" : ""
                        cellHue: Theme.accent
                        lead: root.asked !== null ? "" : secret ? Theme.glyph.secret
                            : thing ? thing.lead ?? thing.glyph
                            : home ? Theme.glyph.home : ""
                        // Home's mark in a place's own hue, since home's
                        // head is every place's head (VKCF3Z, 2a429f67).
                        leadHue: secret ? Theme.machine
                            : thing ? Theme[thing.leadHue ?? thing.hue] : Theme.voice
                        glyphWidth: root.textAreaAt
                        leadColumn: root.glyphColumn
                        // A folder's path gives way before any other step.
                        chips: home ? [{ text: Words.field.home ?? "", yields: false }]
                            : root.path.map(step => ({ text: step, yields: /^[~/]/.test(step) }))
                        // Asked only where the line asks something in
                        // particular, as a name to save by or a file a
                        // program wants: on a find, the mark and the caret
                        // already say "type", so a bare "which?" there was
                        // one more level in a head held to two (VKCF3Z,
                        // 2026-11-06, on Joel's "a bumpy hill").
                        readonly property string asks: root.held.length === 0 ? ""
                            : root.held[root.held.length - 1].row.ask ?? ""
                        hint: asks !== Words.anyAsk ? asks : ""
                        ghost: root.completion
                        // An answer's headline, which y copies, is said in
                        // its ghost, so that flashes in its row's place.
                        ghostFlashed: root.flashing[-1] === true
                        // Nothing at its right: what the selection has to
                        // say is said on its row, at the far end (Choice's
                        // `say`), and where the cursor is, by the gutter's
                        // numbers and the status line, so the head holds
                        // the mark, the path and what's typed alone.
                        aside: ""
                        // A find narrowing the rows says how many it found,
                        // in normal.
                        narrowed: root.typed !== "" && root.matches.length > 0
                            ? (Words.field.found ?? "").replace("{count}",
                                String(root.matches.length)) : ""
                        onPressing: root.filled("key")
                        // How the place it's in stands, where its source
                        // says that presses, as People's down daemon.
                        placeStands: root.held.length === 0 ? ""
                            : root.held[0].row.from?.pressing ?? ""
                        onTextChanged: {
                            // Put back to home while closed, nothing is
                            // searched or timed (keepHome).
                            if (root.keeping)
                                return;
                            const due = root.keysDue;
                            trials.retyped(due.length > 0 ? due[due.length - 1].text
                                : root.typed, text);
                            root.searchSoon(text, Date.now(), KeyTimes.now(),
                                KeyTimes.pressed());
                        }
                        // Return, as the vi-paradigm spec's table has it:
                        // words asked for are handed over; a thing whose
                        // act changes something is stepped into; from the
                        // find, a row that changes something is landed on,
                        // leaving the line for normal; anything else opens
                        // or is done, so a name and one Return opens it;
                        // and one that ends something, from either, only
                        // writes its command on the : line.
                        onAccepted: {
                            const heard = KeyTimes.now();
                            const pressed = KeyTimes.pressed();
                            root.searchNow();
                            trials.pressed("returns");
                            const act = root.asking ? "gave" : root.landedJustNow() ? ""
                                : root.stepsThing(root.selected) ? "entered"
                                : root.endsSomething(root.selected) ? "lined"
                                : root.lands(root.selected) ? "landed" : "ran";
                            if (act === "gave")
                                root.give();
                            else if (act === "entered")
                                root.enter(root.selected);
                            else if (act === "lined")
                                root.lineFor(root.selected);
                            else if (act === "landed")
                                root.land(root.selected);
                            else if (act === "ran")
                                root.run(root.selected, false);
                            // A doubled Return is the landing's own press,
                            // so the landing still says what a second does.
                            else
                                root.landedKeys = prompt.keysTaken;
                            root.returned = { act: act, pressed: pressed > 0 ? pressed : null,
                                heard: heard, done: KeyTimes.now() };
                        }
                        // Ctrl+Y on an answer puts the answer in place of
                        // the question, so a sum can be carried on; one
                        // still being asked for leaves the question as it is.
                        onCompleted: {
                            root.searchNow();
                            trials.pressed("tabs");
                            // Ctrl+Y does something only while the field
                            // shows a grey completion, and only takes it, as
                            // his Neovim's completion menu does; stepping in
                            // is normal's right arrow's (Joel, 2026-10-26).
                            // At home there's none, since nothing is typed.
                            if (root.completion === "")
                                return;
                            // A command puts its start in the field.
                            if (root.current !== -1 && root.selected?.start !== undefined)
                                root.fill(root.selected.start);
                            // A question only started is finished first.
                            else if (root.question !== ""
                                && (root.current === -1 || root.selected?.answered))
                                text = root.question;
                            else if (root.current !== -1 || !root.answer)
                                text = text + root.completion;
                            // An answer that does something has nothing to
                            // put in the question's place.
                            else if (!root.answer.run && (root.answer.copy ?? root.answer.headline) !== "")
                                text = root.answer.copy ?? root.answer.headline;
                            root.searchNow();
                        }
                        // Normal's Escape at rest: the key list let go, or
                        // the launcher closed, a popup, from any depth, as
                        // his Neovim closes its floats; it never steps out
                        // (the vi-paradigm spec, Normal).
                        onCancelled: {
                            if (root.listing)
                                root.listing = false;
                            // Words asked for, as a password, are given up,
                            // back to the row that asked, as Vim's
                            // inputsecret() gives up on Escape.
                            else if (root.asking)
                                root.back();
                            else
                                root.open = false;
                        }
                        // Escape at rest lets the key list go, and words
                        // asked for, as it leaves what's begun, and closes
                        // nothing (Joel, 2026-11-05: "fuck escape quitting").
                        onRested: {
                            if (root.listing)
                                root.listing = false;
                            else if (root.asking)
                                root.back();
                        }
                        // A key with no meaning here says so, until the next.
                        onIdled: key => root.say([[ListedKeys.face(key),
                            (Words.field.hints ?? {}).notHere ?? ""]])
                        // - goes up, as his oil's; at home, with nothing
                        // above, it says so, as any key meaning nothing does.
                        onBackward: {
                            root.searchNow();
                            trials.pressed("backs");
                            if (!root.canGoUp)
                                root.say([[root.hintKey("out"),
                                    (Words.field.hints ?? {}).notHere ?? ""]]);
                            else
                                root.back();
                        }
                        onOperated: (act, count, motion) => root.operated(act, count, motion)
                        onLettered: (act, count, letter) => {
                            root.searchNow();
                            root.toLetter(act, count, letter);
                        }
                        // The ask, while it stands, takes every key, and one
                        // no keys asked is said in the line's place.
                        confirming: root.asked !== null
                        asking: root.asked?.question ?? ""
                        askingParts: root.asked?.parts ?? null
                        onAnswered: key => root.answerAsk(key)
                        // There's a drawer to go into while it's out with
                        // words to sit on.
                        drawerHere: root.drawerOut && drawerCard.lined
                        reselectable: root.lastVisual !== null
                        // A search run in the drawer puts its cursor on
                        // the match.
                        onSought: (pattern, forward, count) => {
                            const went = drawerCard.seek(pattern, forward, count);
                            root.saySought(went, went.pattern);
                        }
                        // The highest the selection goes: an answer's headline,
                        // or the row a step's line holds, sits above the rows,
                        // at -1.
                        readonly property int highest: (root.answer && !root.parted(root.answer))
                            || root.lineHeads ? -1 : 0
                        // The rows' keys, in either mode (QueryKeys.js).
                        // Every one is a move, which Operators.moved
                        // says the end of from the rows in view, as the
                        // list has them. Ctrl+D and Ctrl+U scroll the rows
                        // as far as they move the selection, so it keeps
                        // its place in view, as Vim's do.
                        // In the drawer, every one is the drawer's: where
                        // its cursor goes and its view with it, n and N
                        // going on with its search, and o, gv and y its
                        // visual's, y's copy going on the clipboard.
                        onStepped: (act, count) => {
                            root.searchNow();
                            if (QueryKeys.paged(mode)) {
                                if (act === "seekNext" || act === "seekPrevious") {
                                    const went = drawerCard.seekOn(act, count);
                                    root.saySought(went, went.pattern);
                                }
                                else if (act === "otherEnd")
                                    drawerCard.otherEnd();
                                else if (act === "reselect")
                                    drawerCard.reselect();
                                else if (act === "copy" || act === "copyLines")
                                    Quickshell.clipboardText = drawerCard.copy(act === "copyLines");
                                else if (act === "keys")
                                    root.listInDrawer();
                                else if (act === "quitSaid")
                                    root.sayQuit();
                                // A page can't be changed, so d c p x say
                                // Vim's E21 where the line was, as E492 is.
                                else if (act === "cannotChange")
                                    root.commandSay((Words.field.hints ?? {}).E21 ?? "", true);
                                else
                                    drawerCard.go(act, count);
                                return;
                            }
                            switch (act) {
                            case "xit":
                                root.xit();
                                return;
                            case "undoChange":
                                root.undoChange(count);
                                return;
                            case "redoChange":
                                root.redoChange(count);
                                return;
                            case "again":
                                root.doAgain(count);
                                return;
                            case "close":
                                root.open = false;
                                return;
                            case "commandAgain":
                                root.commandAgain();
                                return;
                            case "numbers":
                                root.numbersRelative = !root.numbersRelative;
                                return;
                            case "keys":
                                root.listing = !root.listing;
                                return;
                            case "lastPlace":
                                root.toLastPlace();
                                return;
                            case "jumpBack":
                                root.jump(-1);
                                return;
                            case "jumpForward":
                                root.jump(1);
                                return;
                            // Tab goes into a row holding parts; on any other
                            // the find's is its next row, and normal's says
                            // it isn't here.
                            case "into":
                                if (root.tabbedIn(mode))
                                    return;
                                act = "next";
                                break;
                            case "follow":
                                root.follow(root.matches[root.current] ?? null);
                                return;
                            case "tagBack":
                                root.tagBack();
                                return;
                            // h and l on a row say Vim's way out and in,
                            // and Backspace as h, read as the one beside
                            // :q is: "l not here \u00b7 \u21b5 open".
                            case "intoSaid":
                            case "outSaid":
                                root.sayTheWay(act);
                                return;
                            case "quitSaid":
                                root.sayQuit();
                                return;
                            case "nextFile":
                                root.beside(1);
                                return;
                            case "previousFile":
                                root.beside(-1);
                                return;
                            case "openOwn":
                                root.matches[root.current]?.openOwn?.();
                                return;
                            // n goes the find's way and N against it.
                            case "matchNext":
                            case "matchBack":
                                root.toMatch((act === "matchNext") !== findsBack, count,
                                    QueryKeys.keyOf(act, "normal"));
                                return;
                            case "letterAgain":
                            case "letterAgainBack":
                                root.toLetter(act, count, "");
                                return;
                            case "otherEnd":
                            case "reselect":
                            case "inParagraph":
                            case "aParagraph":
                                root.visualStep(act, count);
                                return;
                            }
                            const view = list.inView();
                            const to = Operators.moved(act, count, {
                                at: root.current,
                                highest,
                                last: root.matches.length - 1,
                                top: view.top,
                                bottom: view.bottom,
                                screen: root.rows,
                                runOf: at => root.runOf(at)
                            });
                            if (act === "halfDown" || act === "halfUp")
                                list.scrollRows(to - root.current);
                            // Timed under the arrows' names, as they were
                            // before the rows had vi's moves, and the
                            // rest under their first key.
                            const timedAs = { previous: "\u2191", next: "\u2193",
                                pageUp: "page up", pageDown: "page down" };
                            root.arrowed(to, timedAs[act] ?? QueryKeys.keyOf(act, "normal"));
                        }
                    }

                    // The command line, in the find line's place while : is
                    // open, its : where the find's / stands and what's typed
                    // on the rows' names, over the name it completes.
                    Loader {
                        id: commandLoader
                        anchors.fill: parent
                        active: root.commanding !== null
                        onLoaded: item.take(root.commanding?.range ?? "",
                            root.commanding?.typed ?? "")

                        sourceComponent: CommandLine {
                            // Its : in the place glyph's cell, as the find's
                            // / stands, and nothing in the gutter's column.
                            markMiddle: prompt.cellMiddle
                            typedAt: card.inset + root.columns.text
                            menu: root.commanding?.menu ?? []
                            argued: (entry, arg) => root.commandArguments(entry, arg)
                            onRan: read => root.commandRan(read)
                            // Let go from the drawer, the keys go back onto
                            // its page however it was let go, choosing
                            // nothing, as Vim's visual ends at its : line.
                            onLetGo: how => root.commandLeft(
                                QueryKeys.paged(root.commanding?.from ?? "") ? "page"
                                : how === "dropped" ? root.commanding?.from ?? "normal"
                                : "normal")
                        }
                    }

                    // What Return on the command line said, in its place
                    // for the next key alone, as Vim's command line says
                    // E492 or volume=40: its words in text where what's
                    // typed stood, and for what went wrong the failed mark
                    // in the alert where the : stood.
                    Item {
                        anchors.fill: parent
                        visible: root.commandSays

                        Text {
                            x: card.inset
                            anchors.baseline: commandSaidText.baseline
                            visible: root.commandFailed
                            text: Theme.glyph.failed
                            color: Theme.alert
                            font.family: Theme.level.figure.family
                            font.pointSize: Theme.level.figure.pointSize
                        }

                        Text {
                            id: commandSaidText
                            x: card.inset + root.columns.text
                            width: parent.width - x - card.inset
                            y: Bands.baseline(parent.height, saidCaps.tightBoundingRect.height,
                                saidSmalls.tightBoundingRect.height) - baselineOffset
                            text: root.commandSaid
                            textFormat: Text.PlainText
                            elide: Text.ElideRight
                            color: Theme.text
                            font.family: Theme.level.figure.family
                            font.pointSize: Theme.level.figure.pointSize
                            font.weight: Theme.level.figure.weight
                            font.features: Theme.level.figure.features
                        }

                        TextMetrics {
                            id: saidCaps
                            text: "H"
                            font: commandSaidText.font
                        }
                        TextMetrics {
                            id: saidSmalls
                            text: "x"
                            font: commandSaidText.font
                        }
                    }
                }

                // What it reaches, split from what's typed by a hairline,
                // inset to inset, so it spans the ink and never cuts the
                // card into boxes against the frame (VKCF3Z, 2026-11-05).
                Rectangle {
                    id: headLine
                    x: card.inset
                    width: parent.width - 2 * x
                    height: Theme.border
                    color: Theme.edge
                    opacity: root.wordsShown
                }

                // The rows and the foot, from frame to frame: the rows' bands
                // a tie from the head's hairline and from the foot's, so a
                // ground never touches a rule (VKCF3Z, 2026-11-06), and the
                // foot's baseline an inset above the frame.
                Column {
                    width: parent.width

                    Item {
                        width: parent.width
                        height: Theme.space.rung.tie
                    }

                    // As tall as it opened, whatever's found, so the foot
                    // never jumps and a row never moves because others
                    // came or went: the rows it doesn't fill are space.
                    Item {
                        id: listBox
                        width: parent.width
                        // Over a viewer only the line's menu, as tall as its
                        // lines, ten at most, as his pumheight.
                        height: root.over !== null
                            ? Math.min(commandRows.count, 10) * root.rowTall : root.tall
                        opacity: root.wordsShown
                        // It cuts the rows at its edges rather than the list
                        // at its own, so a row's ground reaches out over the
                        // gutter to the frame.
                        clip: true

                        // The command line's menu, while : is open: its lines
                        // are the list, in the rows' place, each a row as any
                        // other is, its glyph, its command in Vim's help
                        // notation on the names' column under what's typed,
                        // the act it names in its words and its key at its
                        // right, rather than a menu indented under the line
                        // (Joel, 2026-11-06: "i don't like how auto
                        // completions are indented, they signal the wrong
                        // organizational concept"). The place's rows stay
                        // as they were beneath it, the cursor on its row, so
                        // they come back as the line is let go.
                        ListView {
                            id: commandRows
                            x: root.textAreaAt
                            width: parent.width - x
                            height: parent.height
                            visible: root.commanding !== null
                            clip: true
                            interactive: false
                            boundsBehavior: Flickable.StopAtBounds
                            model: commandLoader.item?.shown ?? []
                            currentIndex: commandLoader.item?.choosing.at ?? -1
                            onCurrentIndexChanged: {
                                if (currentIndex >= 0)
                                    positionViewAtIndex(currentIndex, ListView.Contain);
                                else
                                    positionViewAtBeginning();
                            }
                            onContentYChanged: Qt.callLater(root.placeNumbers)
                            onCountChanged: Qt.callLater(root.placeNumbers)

                            delegate: Item {
                                id: commandRow

                                required property int index
                                required property var modelData
                                readonly property var act: modelData.act ?? null
                                readonly property string named: Commands.called(modelData)
                                    + (modelData.after ?? "")

                                width: commandRows.width
                                height: root.rowTall
                                // Its name's baseline, down from its top, which
                                // the gutter sets its number on.
                                readonly property real nameBaseline: commandChoice.nameBaseline

                                Choice {
                                    id: commandChoice
                                    anchors.fill: parent
                                    groundFrom: commandRows.x
                                    selected: commandRow.index === commandRows.currentIndex
                                    shown: card.opacity > 0
                                    glyph: commandRow.modelData.glyph
                                        || (commandRow.act?.glyph ?? "")
                                    hue: Theme[commandRow.modelData.hue
                                        || (commandRow.act?.hue ?? "")] ?? Theme.text
                                    name: commandRow.named
                                    plainName: commandRow.named
                                    count: commandRow.modelData.title ?? ""
                                    side: commandRow.modelData.key ?? ""
                                }
                            }
                        }

                        // The rows' numbers (Gutter.qml), in every mode, in
                        // the room the place's list can number, so no row
                        // moves on a mode change or a letter.
                        Gutter {
                            id: rowsGutter
                            x: card.inset
                            // Over the rows' grounds, which reach under it.
                            z: 1
                            height: parent.height
                            clip: true
                            slots: Math.ceil(height / root.rowTall) + 1
                            // On the : line, its menu's chosen line, which
                            // its numbers count as the rows' are counted.
                            current: root.commanding !== null
                                ? commandRows.currentIndex : root.current
                            relative: root.numbersCounted
                            digits: root.gutterDigits
                            // On the : line, as many as its lines need.
                            figures: root.commanding !== null
                                ? String(Math.max(1, commandRows.count)).length
                                : root.numberDigits
                            shown: root.numbered
                        }

                        // Where the rows are drawn moves the gutter's numbers
                        // with them, a glide's every frame included.
                        Connections {
                            target: list
                            function onContentYChanged() { root.placeNumbers(); }
                            function onContentHeightChanged() { root.placeNumbers(); }
                            function onOriginYChanged() { root.placeNumbers(); }
                            function onYChanged() { root.placeNumbers(); }
                        }

                        ListView {
                            id: list

                            visible: root.over === null && root.commanding === null
                            // Beside the gutter, which numbers its rows in
                            // normal and keeps its room everywhere else, out
                            // to the frame, its rows keeping their own inset.
                            x: root.textAreaAt
                            width: parent.width - x
                            anchors.top: parent.top
                            // Below the empty line, where it says a place
                            // holds nothing but ways elsewhere.
                            anchors.topMargin: nothingShown.visible ? root.rowTall : 0
                            anchors.bottom: parent.bottom
                            // How many characters the widest of what graphed rows
                            // have said at their right in this step, as Choice
                            // says it, so their graphs stand in one column that
                            // holds still as 9% becomes 10% (root.figuresWidest).
                            readonly property int figures: root.figuresWidest
                            // And the widest any row has said there.
                            readonly property int sides: root.sidesWidest
                            boundsBehavior: Flickable.StopAtBounds
                            // A row built past the view each way, so the
                            // next an arrow reaches is drawn before it's
                            // shown. Qt's own cache, about seven rows, made
                            // a first letter or a step into a place draw 15
                            // rows where 8 are shown, and a letter's layout
                            // took 27 ms where one row ahead took 14 to 17
                            // in the sandbox (2026-11-10). Held arrows still
                            // draw one row a step either way. A pixel short
                            // of a row, since Qt builds a row whose top
                            // meets the cache's edge, and a view a whole
                            // number of rows tall drew a row with no pixel
                            // in reach (qquicklistview.cpp's pos <= fillTo).
                            cacheBuffer: root.rowTall - 1
                            // The rows by what each is (steadyId), each
                            // drawn once and kept while it's there: a refresh,
                            // each second inside Machine and four times a
                            // second while Sound's meter runs, made every row
                            // afresh, restarting whatever moved on it (the
                            // launcher audit's finding 30, Joel's yes on
                            // 2026-10-26). A row kept reads what it says now
                            // from root.matches by its place.
                            model: ListModel {
                                id: rowsKept
                            }
                            // A row let go is kept aside and given to the
                            // next row new to the list, rather than each
                            // being built from nothing: a step into Desktop
                            // or Machine built every row it showed, 2.5 to 3
                            // ms each, two thirds of the step's own work,
                            // and taking them from those let go took that
                            // from 40 ms to 14 under load (2026-11-10). A row
                            // kept aside says nothing and is selected by
                            // nothing, so it takes nothing of what it was
                            // to the row it's given (entry.aside).
                            reuseItems: true

                            // What each kept row is, in the order they stand,
                            // so keeping them never reads the model back.
                            property var keptKeys: []
                            // How many rows the last keeping drew afresh, and
                            // how long laying them out took, which a key's
                            // showing says beside its own time, or null
                            // until the rows it composed were kept.
                            property var keptLast: null
                            // Whether keeping them last put every row in
                            // again (RowsKept.afresh), so they stand from
                            // the list's start, whatever it was scrolled to.
                            property bool afresh: false

                            // Brings the kept rows into line with these, by
                            // what each is (RowsKept.js): a row listed before
                            // stays wherever it moves, and only a row new to
                            // the list is drawn afresh. Says whether they
                            // were all the same. The rows go in between the
                            // removing and the rest, so no kept row ever
                            // reads past the end of the rows it reads: the
                            // removing leaves none past the old ones' end,
                            // and the new ones reach as far as what's left
                            // and what's added.
                            function keep(rows) {
                                const keys = rows.map(row => root.pairedId(row));
                                // More rows moving than the card shows are
                                // listed afresh, which reads each row in view
                                // once, where each move read them again.
                                const steps = RowsKept.steps(list.keptKeys, keys, root.rows);
                                list.afresh = RowsKept.whole(list.keptKeys, steps);
                                list.keptKeys = keys;
                                if (steps.length === 0) {
                                    list.rows = rows;
                                    list.keptLast = { fresh: 0, laying: 0 };
                                    return true;
                                }
                                for (const step of steps.filter(one => one.remove !== undefined))
                                    rowsKept.remove(step.remove, step.count);
                                list.rows = rows;
                                // Listed afresh, the rows let go are laid
                                // aside before the new ones are put in, so
                                // those are given them: a view lets go of
                                // the rows a change removed only after it
                                // has made the rows it put in, so with none
                                // aside yet, as on a shell's first key, it
                                // built every row anew, Joel's "m" 8 of
                                // them in 15 ms (2026-11-11).
                                // Its laying is said with the rest.
                                const asideFrom = Date.now();
                                if (list.afresh)
                                    list.forceLayout();
                                const aside = Date.now() - asideFrom;
                                for (const step of steps.filter(one => one.remove === undefined)) {
                                    if (step.move !== undefined)
                                        rowsKept.move(step.move, step.to, 1);
                                    else
                                        rowsKept.insert(step.insert,
                                            step.keys.map(key => ({ key })));
                                }
                                // Laid out now rather than at the next frame,
                                // so the rows drawn stand where they will and
                                // the selection is placed from them (rowY).
                                // How many rows the laying built and took
                                // back from those let go are said beside it,
                                // since Joel's "ca" laid for 191 ms with 15
                                // rows new and "batt" for 1 with 1
                                // (2026-11-11): rows moving into view are
                                // drawn too, and a laying long for the rows
                                // it drew is spent elsewhere.
                                const laying = Date.now();
                                const built = root.rowsBuilt;
                                const drawn = root.rowsDrawn;
                                list.forceLayout();
                                list.keptLast = {
                                    fresh: steps.reduce((all, step) => all
                                        + (step.keys ?? []).length, 0),
                                    laying: Date.now() - laying + aside,
                                    built: root.rowsBuilt - built,
                                    reused: root.rowsDrawn - drawn - (root.rowsBuilt - built)
                                };
                                return false;
                            }

                            // The launcher's one motion is the selection
                            // moving to its row, the list scrolling with it
                            // on the same timing, on the theme's curve (the
                            // launcher audit's finding 33, Joel's yes on
                            // 2026-10-26: it jumped, and the list snapped a
                            // row at a time). The launcher's selection, and
                            // the list's scroll with it, lands within 50 ms
                            // of the key, a short glide on the curve, since a
                            // highlight still moving once Joel has read its
                            // row is lag (2026-11-10): Glide.js says how
                            // long, shorter for a key already late, and none
                            // for one too late for a frame of it, where the
                            // theme's medium, 150 ms, ended 100 ms past the
                            // bar. Only a move among the
                            // same rows glides: rows made afresh, by a letter,
                            // a step or a refresh, are a new list, and its
                            // selection is where it is at once, so typing
                            // never glides. Everything else stays instant.
                            property bool settled: false
                            readonly property var shown: root.matches
                            property string typedWas: ""
                            // The rows as the kept ones read them, set by
                            // keep partway through, so none reads past the end.
                            property var rows: []
                            onShownChanged: {
                                const same = list.keep(root.matches);
                                // Told where they stand once they're the
                                // rows drawn, so those they replace aren't.
                                root.rowsPlaced();
                                // The same rows read again leave a glide under
                                // way; a letter typed never glides, even onto
                                // the same rows.
                                // A glide under way is aimed again where its
                                // row now is, should a row above it have
                                // changed height, so it never ends off it.
                                // So is the list's: aimed at a place rather
                                // than a row, it ended where the row had been,
                                // above the first row once rows above it went,
                                // leaving an empty band at the top (Joel,
                                // 2026-11-08).
                                if (same && root.typed === list.typedWas) {
                                    if (!picking.running)
                                        list.pick(false);
                                    else if (picking.to !== list.fillAt(root.current))
                                        list.pick(true);
                                    list.reaim();
                                    Qt.callLater(root.settledNow);
                                    return;
                                }
                                list.typedWas = root.typed;
                                list.settled = false;
                                // A glide under way gives way at once, its
                                // row shown where it now stands: ended where
                                // it was going, it went to a place the new
                                // rows no longer hold.
                                const gliding = scrolling.running;
                                scrolling.stop();
                                list.pick(false);
                                if (gliding && root.current >= 0)
                                    list.positionViewAtIndex(root.current, ListView.Contain);
                                Qt.callLater(() => list.settled = true);
                                Qt.callLater(root.settledNow);
                            }

                            // Where the selection is drawn, set by pick
                            // rather than bound, so a glide under way gives
                            // way at once to rows made afresh.
                            property real pickY: 0

                            // Where the fill sits on a row, in the list's
                            // own place, as its rows' y is.
                            function fillAt(at) {
                                return list.rowY(at) + list.rowHeight(at) - root.rowTall;
                            }

                            // Where a row's own place starts in the list,
                            // its gap above it included, counted from a row
                            // the list has drawn. Qt places what it hasn't
                            // drawn by the average height of what it has,
                            // so with a gap above some rows the origin and
                            // the heights summed from it put the fill off
                            // its row and aimed the list above its first,
                            // leaving an empty band at the top (Joel,
                            // 2026-11-08). With nothing drawn, the estimate
                            // is all there is.
                            function rowY(at) {
                                const drawn = list.itemAtIndex(at) !== null ? at
                                    : list.indexAt(list.width / 2, list.contentY + 1);
                                const item = drawn >= 0 ? list.itemAtIndex(drawn) : null;
                                return item === null ? list.originY + list.rowTop(at)
                                    : item.y + list.rowTop(at) - list.rowTop(drawn);
                            }

                            // A place to scroll to, kept within the rows:
                            // never above the first nor past the last.
                            function clamped(to) {
                                const last = root.matches.length - 1;
                                const least = list.rowY(0);
                                const most = last < 0 ? least : Math.max(least,
                                    list.rowY(last) + list.rowHeight(last) - list.height);
                                return Math.max(least, Math.min(to, most));
                            }

                            // How long a glide set going now runs, from
                            // when the key moving it was pressed and the
                            // panel's frame (Glide.js): none for a key too
                            // late for a frame of glide.
                            function lasting() {
                                const pressed = KeyTimes.pressed();
                                const rate = card.Screen.refreshRate;
                                return Glide.lasting(pressed > 0 ? KeyTimes.now() - pressed : 0,
                                    1000 / (rate > 0 ? rate : 60));
                            }

                            function pick(glide) {
                                const to = list.fillAt(root.current);
                                picking.stop();
                                const lasts = glide && to !== list.pickY ? list.lasting() : 0;
                                if (lasts > 0) {
                                    root.glideBegun();
                                    Report.say(`glides ${Math.round(list.pickY)} to `
                                        + `${Math.round(to)} in ${lasts} ms`);
                                    picking.duration = lasts;
                                    picking.to = to;
                                    picking.start();
                                } else
                                    list.pickY = to;
                            }

                            // A glide has ended once neither the selection
                            // nor the scroll moves, so the keys that set it
                            // going are timed to the next frame (keysLanded).
                            // Asked again once the handler that stopped it is
                            // done, since a glide aimed anew stops and starts
                            // in one go, and hasn't ended.
                            readonly property bool gliding: picking.running || scrolling.running
                            onGlidingChanged: {
                                if (!gliding)
                                    Qt.callLater(list.glideSettled);
                            }
                            function glideSettled() {
                                if (!list.gliding)
                                    root.glideEnded();
                            }

                            // Its duration is set by each glide (pick).
                            NumberAnimation {
                                id: picking
                                target: list
                                property: "pickY"
                                easing.type: Easing.BezierSpline
                                easing.bezierCurve: Theme.motion.curve
                            }

                            // Where a row's own place starts and ends in the
                            // list, its gap above it included.
                            function rowTop(at) {
                                let top = 0;
                                for (let each = 0; each < at && each < root.matches.length; each += 1)
                                    top += root.rowTall + (root.matches[each].gap ? root.apart : 0);
                                return top;
                            }

                            function rowHeight(at) {
                                return root.rowTall + (root.matches[at]?.gap ? root.apart : 0);
                            }

                            // Where a row's name has its baseline, down
                            // from the card's top, or null while it isn't
                            // drawn: the line the drawer's title sits on.
                            function baselineOf(at) {
                                const entry = list.itemAtIndex(at);
                                return entry ? entry.mapToItem(card, 0, entry.nameBaseline).y : null;
                            }
                            // A drawn row's words, what it holds or its
                            // note: their baseline on the card, how far
                            // they stand from its name, and their face;
                            // null for a row not drawn or saying none.
                            function wordsOf(at) {
                                const entry = list.itemAtIndex(at);
                                const row = entry?.drawnRow;
                                return row && row.detail !== "" ? {
                                    baseline: root.hundredths(
                                        entry.mapToItem(card, 0, row.y + row.detailBaseline).y),
                                    gap: root.hundredths(row.detailGap),
                                    face: row.detailFace
                                } : null;
                            }

                            // The first and last rows wholly in view, read
                            // from the rows the list has drawn, where it's
                            // going if it's gliding, as H M L take them.
                            function inView() {
                                const from = scrolling.running ? scrolling.to : list.contentY;
                                const whole = at => {
                                    const entry = list.itemAtIndex(at);
                                    return entry !== null && entry.y >= from - 0.5
                                        && entry.y + entry.height <= from + list.height + 0.5;
                                };
                                // From the row at the view's top, past one
                                // cut by it, to the last that ends in it.
                                let top = Math.max(0, list.indexAt(list.width / 2, from));
                                const most = Math.min(root.matches.length, top + 2 * root.rows);
                                while (top < most && !whole(top))
                                    top += 1;
                                if (top >= most)
                                    return { top: root.current, bottom: root.current };
                                let bottom = top;
                                while (bottom + 1 < most && whole(bottom + 1))
                                    bottom += 1;
                                return { top, bottom };
                            }

                            // Scrolls by a number of rows, down or up, as
                            // far as the rows go, gliding as reveal does, so
                            // the selection moved as far keeps its place.
                            function scrollRows(by) {
                                const top = Math.min(Math.max(0, list.inView().top + by),
                                    Math.max(0, root.matches.length - 1));
                                list.glide({ top }, list.clamped(list.rowY(top)));
                            }

                            // What a glide under way is going to: a row to
                            // show ({ shown }) or to stand at the top
                            // ({ top }), by its place among rows that stay
                            // the same, so it's aimed again where that row
                            // now is (reaim).
                            property var aim: null

                            function glide(aim, to) {
                                const from = scrolling.running ? scrolling.to : list.contentY;
                                if (to === from)
                                    return;
                                // On the selection's timing, so the two land
                                // together; a key too late for a frame of
                                // glide scrolls at once.
                                const lasts = list.settled ? list.lasting() : 0;
                                if (lasts === 0) {
                                    scrolling.stop();
                                    list.aim = null;
                                    list.contentY = to;
                                    return;
                                }
                                root.glideBegun();
                                list.aim = aim;
                                scrolling.stop();
                                scrolling.duration = lasts;
                                scrolling.to = to;
                                scrolling.start();
                            }

                            // Aims a glide under way again where its row
                            // stands now.
                            function reaim() {
                                if (!scrolling.running || list.aim === null)
                                    return;
                                const to = list.aim.top !== undefined
                                    ? list.clamped(list.rowY(list.aim.top))
                                    : list.showing(list.aim.shown, list.contentY);
                                if (to === scrolling.to)
                                    return;
                                scrolling.stop();
                                scrolling.to = to;
                                scrolling.start();
                            }

                            // Where the list stands to show a row, from
                            // where it stands, moving as little as it can.
                            function showing(at, from) {
                                const top = list.rowY(at);
                                const bottom = top + list.rowHeight(at);
                                return list.clamped(top < from ? top
                                    : bottom > from + list.height ? bottom - list.height : from);
                            }

                            // Scrolls just far enough to show the row, as
                            // positionViewAtIndex's Contain does, gliding
                            // there when the selection glides.
                            function reveal(at) {
                                if (!list.settled) {
                                    list.positionViewAtIndex(at, ListView.Contain);
                                    return;
                                }
                                // From where a glide under way is going, so a
                                // second key while it runs goes on from there.
                                const from = scrolling.running ? scrolling.to : list.contentY;
                                list.glide({ shown: at }, list.showing(at, from));
                            }

                            // Its duration is set by each glide (glide), and
                            // a glide aimed again keeps it.
                            NumberAnimation {
                                id: scrolling
                                target: list
                                property: "contentY"
                                easing.type: Easing.BezierSpline
                                easing.bezierCurve: Theme.motion.curve
                                // Ended within the rows, should they have
                                // changed under it after its last aim.
                                onFinished: {
                                    list.aim = null;
                                    const within = list.clamped(list.contentY);
                                    if (Math.abs(within - list.contentY) > 0.5)
                                        list.contentY = within;
                                }
                            }

                            // The selection itself, under the rows, which
                            // draw no fill of their own here (Choice's fills).
                            Ground {
                                id: picked

                                // Among the rows, so it scrolls with them:
                                // a child of the list itself stayed put as
                                // the list scrolled, and sat as far below
                                // its row as the list had gone down.
                                parent: list.contentItem
                                z: -1
                                // Its row's whole band (Ground.qml). A gap
                                // above a row stays outside its band.
                                from: list.x
                                width: listBox.width
                                height: root.rowTall
                                // Where its row is drawn (fillAt reads the
                                // drawn rows), so rows coming or going above
                                // a scrolled list, which move its origin and
                                // leave the rows in view where they are,
                                // leave it on its row too (Joel, 2026-10-29:
                                // the highlight sat off the selected text).
                                y: list.pickY
                                visible: root.current >= 0 && root.current < root.matches.length
                                // While the drawer holds the keys the row
                                // stays marked, in the raised role rather
                                // than the selection's, since the keys
                                // aren't here: a mark, not a selection.
                                ink: QueryKeys.paged(prompt.mode) ? "raised" : "selection"
                            }

                            delegate: Item {
                                id: entry

                                required property int index
                                // What it says now, read by its place, so a
                                // row kept across a refresh says the new. A
                                // row let go has no place (Qt gives it -1)
                                // until it's destroyed, a moment later, and
                                // says what it last said till then, since
                                // read by a place it hasn't, it said nothing
                                // and every binding on it failed.
                                readonly property var modelData: !entry.aside
                                    && entry.index >= 0 ? list.rows[entry.index] : entry.said
                                property var said: ({})
                                onModelDataChanged: {
                                    if (!entry.aside && entry.index >= 0)
                                        entry.said = entry.modelData;
                                }
                                // Whether it's kept aside, let go and not yet
                                // given a row (list.reuseItems): it reads
                                // only what it last said, and isn't selected,
                                // so a name reading on stops and rests at its
                                // start, and its turns are counted afresh.
                                property bool aside: false
                                ListView.onPooled: {
                                    entry.aside = true;
                                    choice.letGo();
                                }
                                ListView.onReused: {
                                    entry.aside = false;
                                    root.rowsDrawn += 1;
                                }
                                Component.onCompleted: {
                                    root.rowsDrawn += 1;
                                    root.rowsBuilt += 1;
                                }
                                // Its own glyph, or the glyph of what it's
                                // inside (rowsGlyph), for an act.
                                readonly property bool own: (modelData.glyph ?? "") !== ""
                                // Holding more (an answer's parts, a row of rows, a
                                // collection, a part whose verb is "more", a figure
                                // that copies and holds more) before doing, and a
                                // figure only once it does nothing else
                                // (Provider.qml's copyRow and holderRow).
                                readonly property bool holds: !!(modelData.opens
                                    || modelData.nested || modelData.verb === "more"
                                    || modelData.figure === true && modelData.targets)
                                readonly property string sort: holds ? "holds"
                                    : modelData.figure === true && !modelData.verbs ? "figure"
                                    : "act"

                                width: list.width
                                height: root.rowTall + (modelData.gap ? root.apart : 0)

                                // Its name's baseline, down from its top.
                                readonly property real nameBaseline: choice.y
                                    + choice.nameBaseline
                                // Its row, which the report reads its
                                // words' place from.
                                readonly property alias drawnRow: choice

                                Choice {
                                    id: choice
                                    anchors.bottom: parent.bottom
                                    selected: !entry.aside && entry.index === root.current
                                    flashed: !entry.aside && root.flashing[entry.index] === true
                                    tinted: root.tints(entry.index)
                                    // Seen until the closing card has faded, and
                                    // reading a long name on only while the keys
                                    // are on the rows: with them in the drawer,
                                    // only what they move moves.
                                    shown: card.opacity > 0 && !QueryKeys.paged(prompt.mode)
                                    fills: false
                                    groundFrom: list.x
                                    glyph: entry.own || entry.sort !== "act"
                                        ? entry.modelData.glyph ?? "" : root.rowsGlyph
                                    // A colour's swatch wears the colour itself.
                                    hue: entry.modelData.swatch
                                        ?? Theme[(entry.own ? entry.modelData.hue
                                            : root.rowsHue) ?? ""] ?? Theme.text
                                    swatch: !!entry.modelData.swatch
                                    // A step in under the row it belongs
                                    // to, as a session under its
                                    // orchestrator (Agents.qml).
                                    depth: entry.modelData.depth ?? 0
                                    sort: entry.sort
                                    graph: entry.modelData.graph ?? null
                                    warns: !!entry.modelData.warns
                                    marked: entry.modelData.marked === true
                                    alerts: !!entry.modelData.alerts
                                    figures: list.figures
                                    sides: list.sides
                                    // A quiet row's name is plain, so its
                                    // marks would read as letters, and its
                                    // letters take no wash.
                                    readonly property var matched: quiet ? []
                                        : root.matchedIn(entry.modelData)
                                    // A row the ask covers wears the tint
                                    // alone, the find's wash giving way, as
                                    // it gives way to the flash (VKCF3Z).
                                    readonly property bool covered: root.asked !== null
                                        && root.askedOver.includes(entry.index)
                                    // A flashed row's name is plain in the
                                    // flash's ink, as a selected one is.
                                    name: selected || quiet || flashed || covered
                                        ? entry.modelData.name
                                        : root.marked(entry.modelData, matched)
                                    plainName: entry.modelData.name
                                    finds: covered ? [] : Fuzzy.runs(matched)
                                    chord: entry.modelData.chord
                                    side: typeof entry.modelData.counts === "function"
                                        ? entry.modelData.counts(root.tenth) : entry.modelData.side
                                    sideItems: entry.modelData.sideItems ?? []
                                    count: entry.modelData.count ?? ""
                                    countItems: entry.modelData.countItems ?? []
                                    sideWhole: !!entry.modelData.sideWhole
                                    names: entry.modelData.names === true
                                    countWords: !!entry.modelData.countWords
                                    kind: root.rowsAtHome ? root.kindOf(entry.modelData) : ""
                                    landed: selected && root.landedOn(entry.modelData)
                                        ? `\u21b5 ${root.verbOf(entry.modelData)}` : ""
                                    // What the cursor row has to say of how
                                    // it stands, on the row, where the head
                                    // said it before (VKCF3Z, 2026-11-06).
                                    say: selected ? foot.saidStands : ""
                                    // Its note, after its name and what it
                                    // holds, giving way first (VKCF3Z,
                                    // 2026-11-09 [6918348b]).
                                    note: selected ? foot.saidNote : ""
                                    also: entry.modelData.byWord ? root.saidAs(entry.modelData) : ""
                                    // Gone since it was shown, it stays dimmed
                                    // in its place until a key (Steady.js).
                                    // A miss is said as quietly as nothing found.
                                    quiet: !!entry.modelData.gone
                                        || !!entry.modelData.answered && !!entry.modelData.missed
                                    onClicked: root.run(entry.modelData, false)
                                }
                            }
                        }

                        // With nothing typed, the shell says what it can do, or an
                        // empty collection says it's empty in its own words, above
                        // its ways elsewhere if it holds them (Showing.js's
                        // saysNothing); with nothing found, it says so. A field
                        // asking for words has only space below it.
                        Choice {
                            id: nothingShown

                            readonly property var holder: root.held.length > 0
                                ? root.held[root.held.length - 1].row : null
                            readonly property var inside: holder?.from ?? null
                            visible: Showing.saysNothing(root.typed, root.matches,
                                holder?.empty) && root.answer === null
                                && root.asking === null && root.commanding === null
                            glyph: root.typed === "" ? Theme.glyph.greeting
                                : Theme.glyph.nothingFound
                            hue: Theme.voice
                            quiet: true
                            voiced: true
                            name: root.typed !== ""
                                ? `Nothing answers to \u201c${root.typed}\u201d.`
                                : holder?.empty ? holder.empty
                                : inside?.collection
                                    ? inside.collection.empty ?? `No ${inside.collection.noun} yet.`
                                : "A window, a program, an action, or what a key does."
                        }
                    }

                    // A picture, or a row's whole words, show in the drawer
                    // beside the card, never below the rows, so the card's
                    // height never changes with the selection.

                    Item {
                        width: parent.width
                        height: Theme.space.rung.tie
                    }

                    // The foot, one line, the one place that teaches, so the
                    // rows stay clean. On the right, what Return does here,
                    // Shift+Return where it can, and where the arrows step.
                    // How things stand, always, on the left, and the keys on
                    // the right. What the selection has to say (why it's
                    // first when learning put it there, where a held thing
                    // is, where an answer came from) is said at the field's
                    // right end instead, so the glance never gives way to
                    // it: Joel often didn't see the battery or the network.
                    // A region of its own, under a hairline as the head is
                    // over the rows, so no gap among the rows outranks it;
                    // inset to inset, as the head's is.
                    Rectangle {
                        id: footLine
                        x: card.inset
                        width: parent.width - 2 * x
                        height: Theme.border
                        color: Theme.edge
                    }

                    Item {
                        id: foot

                        readonly property var choice: root.selected ?? root.asking
                        // An operator's word, for the key that did it; an
                        // ask is the line's to say.
                        readonly property var operatorWord: root.operatorSaid !== null
                            && root.operatorSaidFor === prompt.keysTaken ? root.operatorSaid
                            : null
                        // A parting word stands where Return was, its
                        // mark in the success colour.
                        // A question asked once takes no key of the row's,
                        // its own (y/n) saying how it's answered, and nor
                        // does an operator waiting, since no key of the
                        // row's does what it says until it's done: what it
                        // will take has their room (modeWord's pending).
                        readonly property var toldNow: root.parting !== ""
                            ? [[Theme.glyph.done, root.parting, Theme.success]]
                            : operatorWord !== null ? [operatorWord]
                            : prompt.asking !== "" || root.pendingSaid !== "" ? []
                            : root.commanding !== null ? root.commandKeys()
                            : root.sayingNow ? root.saying : root.keysFor(choice)
                        // As the card last showed them, while it's closed
                        // (root.cardSeen).
                        property var told: []
                        // Whether what it says is loose words (say), its
                        // last at its own width rather than Return's place.
                        readonly property bool looseNow: root.sayingLoose && root.sayingNow
                            && toldNow === root.saying
                        property bool loose: false

                        Binding {
                            target: foot
                            property: "told"
                            value: foot.toldNow
                            when: root.cardSeen
                            restoreMode: Binding.RestoreNone
                        }

                        Binding {
                            target: foot
                            property: "loose"
                            value: foot.looseNow
                            when: root.cardSeen
                            restoreMode: Binding.RestoreNone
                        }

                        Binding {
                            target: foot
                            property: "said"
                            value: foot.saidNow.footer
                            when: root.cardSeen
                            restoreMode: Binding.RestoreNone
                        }
                        Binding {
                            target: foot
                            property: "saidStands"
                            value: foot.saidNow.stands
                            when: root.cardSeen
                            restoreMode: Binding.RestoreNone
                        }
                        Binding {
                            target: foot
                            property: "saidNote"
                            value: foot.saidNow.note
                            when: root.cardSeen
                            restoreMode: Binding.RestoreNone
                        }
                        // As the foot says them: Return last, at the right
                        // end, the rest before it. Return first, in a place
                        // kept for the widest set, left holes after the keys
                        // a row didn't have; at the end, the others come and
                        // go only to its left, as an action bar's main act
                        // does. Its place's room left over stands after its
                        // word, at the frame's side, so the gap before it is
                        // a hint's like every other (VKCF3Z, 2026-11-05:
                        // held room at the far end, never before Return).
                        readonly property var shown: kept.length === 0 ? []
                            : kept.slice(1).concat([kept[0]])
                        // The keys in the room the facts at their least,
                        // what waits and the mode's word leave them, those
                        // furthest from Return giving way first: drawn
                        // whatever their room, they ran over "1 new" and
                        // "2 chosen" in visual over a full list at 2. The
                        // find says no mode word, so its room is the keys'
                        // too, as it is the facts' (room, below): held for
                        // it, the find's Ctrl+] cut the opening's room short,
                        // and the first letter moved the keys.
                        readonly property real keysRoom: width - card.inset
                            - statusLine.factsAt - standing.least - Theme.space.rung.groups
                            - (prompt.mode === "find" ? 0 : modeWord.room)
                        readonly property var kept: Standing.keysFitted(told,
                            told => root.keysWide(told, foot.loose), keysRoom, root.footYielding)
                        readonly property real returnPlace: told.length === 0 ? 0
                            : loose ? statusLine.keys.advance(told[0][0], told[0][1])
                            : root.returnPlace(told[0])
                        // Where Return's place starts, which holds still.
                        readonly property real returnAt: width - card.inset - returnPlace
                        // How far in the keys may reach: as far as the
                        // widest of them has needed since the launcher
                        // opened, so the left's room holds still too. The
                        // keys are measured by their advances, as noteKeys
                        // measures them, not by the row's laid-out width,
                        // which lags to its next layout pass: opening from
                        // normal, it still held normal's keys as the facts
                        // were fitted, and the glance opened cut for good.
                        // Only the keys drawn are measured, and the room held
                        // for any never passes theirs, or visual's would run
                        // over "1 new" again.
                        readonly property real keysFrom: width - card.inset
                            - Math.max(Math.min(root.footKeysWidest, keysRoom),
                                root.keysWide(kept, foot.loose))
                        // The room the left has, short of the keys and the
                        // mode's word, which keeps its room in every mode it
                        // speaks in so the facts say the same as it comes and
                        // goes. The find never says one, so its facts take
                        // that room: held there, it sat empty while the facts
                        // gave way to the find's own keys, and a fact may
                        // change its form once as the mode changes (Y2YBTS,
                        // 2026-11-05).
                        readonly property real room: keysFrom - Theme.space.rung.groups
                            - statusLine.factsAt
                            - (prompt.mode === "find" ? 0 : modeWord.room)

                        // As the card last showed it, while it's closed.
                        // Strings, so a pass that says the same signals
                        // nothing: what the report's footer names, what
                        // the cursor row says at its right, and its note.
                        property string said: ""
                        property string saidStands: ""
                        property string saidNote: ""
                        // What the cursor row says beyond its name and its
                        // side: its note, after its name, and how it stands
                        // or where it came from, at its right
                        // (Reading.cursorSays). A row whose drawer says its
                        // note says it once, there, while the drawer is out;
                        // an answer's row says where the answer came from.
                        // The name it was once called by and a held thing's
                        // context are asked for only where nothing before
                        // them is said.
                        readonly property var saidNow: !choice ? Reading.cursorSays({})
                            : Reading.cursorSays({
                                state: choice.state ? choice.state() : undefined,
                                once: () => root.onceCalled(choice),
                                note: choice.note ?? "",
                                noteInDrawer: !!choice.noteInDrawer && drawerCard.out,
                                answer: choice.plain ? root.answer : null,
                                context: () => root.held[0]?.row.context ?? ""
                            })

                        width: parent.width
                        // Its baseline on the rows' rhythm, and the frame's
                        // bottom where the line's text centres under its
                        // hairline (docs/design.md: a frame's bottom is set).
                        height: statusLine.tall

                        // How things stand, the quietest thing here: the
                        // facts the margin's pieces hold that are always
                        // true (the time, the date, the battery, the
                        // network, the volume, the day's AI spend, the
                        // agents), each its glyph and the least that
                        // tells Joel something, as a key hint is its key
                        // and what it does, at the hints' size and step,
                        // all muted but a fact that wants him, which
                        // wears its colour. It's the same whatever's
                        // typed, held or asked, so it's always where he
                        // looks for it. As room runs short, each says its
                        // least, then by collapse rank, lowest first, its
                        // glyph alone, and a glyph never gives way
                        // (launcher/Standing.js). Each keeps the room it
                        // has needed since the launcher opened. Two
                        // facts side by side under one glyph (the day's
                        // AI spend and the agents, both AI's) wear it
                        // once.
                        // What the mode is doing, after the facts, in the
                        // accent, as his Neovim's status line puts its showcmd
                        // after what's already there (modules/neovim/status.lua);
                        // the mark at the foot's start names the mode itself.
                        // It comes and goes without moving anything else.
                        // In normal, a count typed before a move stands in
                        // its place until the move, as Neovim's showcmd
                        // shows it, and a drawer's search after its / or ?
                        // until its Return, as Neovim's command line shows
                        // one, each in the strong ink of what's typed, their
                        // last letters where they're longer than the room.
                        // The command line says its own, in the line's place.
                        // A search counts its match of how many after it, as
                        // his Neovim's search count does, while it's typed
                        // and while the cursor stands on one it found.
                        Text {
                            id: modeWord

                            readonly property var found: drawerCard.found
                            readonly property string ofAll: found === null ? ""
                                : " \u00b7 " + String(Words.field.matched ?? "")
                                    .replace("{at}", found.at).replace("{all}", found.of)
                            readonly property var sought: prompt.standing.seek
                                ?? (prompt.mode === "page" && found !== null
                                    ? drawerCard.lastSought : null)
                            // A pending operator's count is in what it says.
                            readonly property string counted: root.pendingSaid !== ""
                                && !prompt.standing.seek ? ""
                                : prompt.standing.count !== "" && !prompt.standing.seek
                                    ? prompt.standing.count
                                : sought ? `${sought.forward ? "/" : "?"}${sought.pattern}${ofAll}`
                                : ""
                            // A pending operator and what it will take, in
                            // the accent as a mode's word is, the row's name
                            // whole in the keys' room as well as its own
                            // (VKCF3Z and Y2YBTS, 2026-11-05: "d \u00b7 close
                            // Notes" read as a whole name where two windows
                            // began so), and given way at a whole word, an
                            // ellipsis after it, only where even that's too
                            // little, as everything cut is.
                            readonly property string pending: root.pendingSaid === "" ? ""
                                : Reading.wholeWords(root.pendingSaid, width,
                                    text => measure.advanceWidth(text))
                            // The mark names the mode, so this says only what
                            // it's doing. A line with no modes, as a secret's,
                            // says nothing, and nor does the command line, which
                            // says its own in the line's place. In normal the
                            // place stood in says how it stands, as the file
                            // picker's "2 gathered"; visual, on the rows or the
                            // drawer's page, how many it's chosen, "3 chosen".
                            readonly property string said: counted !== "" ? counted
                                : pending !== "" ? pending
                                : !prompt.modal || root.commanding !== null ? ""
                                : prompt.mode === "normal" && root.placeSays !== "" ? root.placeSays
                                : root.visual ? Operators.chosenSaid(
                                    root.visualSpan.to - root.visualSpan.from + 1,
                                    Words.field.visual ?? {})
                                : QueryKeys.chosen(prompt.mode)
                                    ? Operators.chosenSaid(drawerCard.chosenCount,
                                        Words.field.visual ?? {})
                                : ""
                            // The widest it says often and its gap, held
                            // whatever shows: visual's count in as many figures
                            // as the place has rows, which hold still while it
                            // chooses, or three while the drawer is out, its
                            // page counting letters too, so a count or an
                            // operator's words never move the facts beside it.
                            // Three figures always held "999 chosen" beside two
                            // rows, and cost visual its y (VKCF3Z, 2026-11-06).
                            // The drawer's out, not what it shows, since it
                            // holds a row's page while it's in. A binding
                            // doesn't see what a call reads, so it reads the
                            // face itself, to be measured again in the theme's
                            // face once that arrives.
                            // Each mode holds only what it says itself:
                            // normal a count typed and what the place says,
                            // the others their count chosen. Normal's held
                            // for visual's "N chosen", which it never says,
                            // and gave its : and - away to it (VKCF3Z,
                            // 2026-11-07: space that ranks nothing). The
                            // facts sit left of it, so they hold still all
                            // the same, and visual brings keys of its own.
                            readonly property real room: {
                                void measure.font;
                                const figures = "9".repeat(root.footFiguresWidest);
                                // What the place says widens it while it says
                                // it, so it's read whole.
                                const widest = prompt.mode !== "normal"
                                    ? measure.advanceWidth(Operators.chosenSaid(figures,
                                        Words.field.visual ?? {}))
                                    : Math.max(measure.advanceWidth(figures),
                                        root.placeSays === "" ? 0
                                            : measure.advanceWidth(root.placeSays));
                                return Theme.space.rung.groups + Math.ceil(widest);
                            }
                            // A group after the facts, since it says the line's
                            // state rather than one more fact, past the room
                            // the facts have held.
                            x: standing.after
                            width: Math.max(0, root.pendingSaid !== ""
                                ? foot.width - card.inset - x
                                : room - Theme.space.rung.groups)
                            anchors.baseline: parent.top
                            anchors.baselineOffset: statusLine.base
                            visible: said !== ""
                            text: said
                            textFormat: Text.PlainText
                            // A mode's word is never cut; a count or an ex
                            // line longer than the room keeps its last letters,
                            // and what the place says its first.
                            elide: counted !== "" ? Text.ElideLeft
                                : said === root.placeSays ? Text.ElideRight : Text.ElideNone
                            color: counted !== "" ? Theme.strong : Theme.accent
                            font: measure.font
                        }

                        // The status line held notices' foot is too (StatusLine.qml):
                        // the mode in three capitals first (Joel, 2026-11-05: "the
                        // mode should be 3 letters, in caps, and in a place where
                        // they are more obvious and not lost with other things"),
                        // then the facts and the mode's word, set here from where
                        // it says, then the selected row's keys at the right,
                        // joined by dots, Return last in its place
                        // (footReturnWidest), the rest close by. A line with no
                        // modes, as a secret's, keeps the mark's room with none
                        // shown.
                        StatusLine {
                            id: statusLine

                            anchors.fill: parent
                            // On the rows' rhythm, as #1954 set the foot's.
                            base: root.footBase
                            mark: ModeMarks.mark(prompt.mode, root.commanding !== null)
                            resting: mark === ModeMarks.mark("normal", false)
                            marked: prompt.modal
                            hints: foot.shown
                            widths: foot.shown.map((key, at) => at === foot.shown.length - 1
                                ? foot.returnPlace : 0)
                        }

                        Row {
                            id: standing

                            // Each as it is now, its colour's facts too, so
                            // one held while the card is closed says what
                            // it said rather than reading its piece.
                            readonly property var factsNow: Facts.all
                                .filter(fact => fact.entry.tier !== "when" && fact.piece
                                    && fact.piece.present && fact.piece.value !== "")
                                .map(fact => ({
                                    entry: fact.entry,
                                    sign: fact.piece.sign !== "" ? Theme.glyph[fact.piece.sign] ?? "" : "",
                                    glance: fact.piece.glance ?? "",
                                    least: fact.piece.least ?? "",
                                    wants: !!fact.piece.wants,
                                    role: fact.piece.role ?? "text",
                                    hue: fact.piece.hue ?? ""
                                }))
                                // The network's glyph alone says all is well.
                                .filter(fact => fact.sign !== "" || fact.glance !== "")
                            // As the card last showed them, while it's
                            // closed (root.cardSeen).
                            property var facts: []

                            Binding {
                                target: standing
                                property: "facts"
                                value: standing.factsNow
                                when: root.cardSeen
                                restoreMode: Binding.RestoreNone
                            }

                            // Read no longer than the shortest they've had to
                            // since the launcher opened (root.footStage), so
                            // they change their form once, not back and forth.
                            // The room a fact takes: what it says, or the
                            // most it's said that way since the launcher
                            // opened (root.factsWidest).
                            function place(fact) {
                                return Math.max(measure.advanceWidth(fact.said),
                                    root.factsWidest[`${fact.entry.name}:${fact.way}`] ?? 0);
                            }

                            // What waits behind rows held still, "2 new":
                            // the last fact, beside the mode holding them,
                            // which never gives way, so the rest fit what
                            // it leaves (VKCF3Z, 2026-11-05).
                            readonly property var waiting: root.waitingSaid === "" ? []
                                : [{ entry: { name: "waiting" }, way: "", said: root.waitingSaid }]
                            readonly property var fit: {
                                const wide = facts => facts.reduce((sum, fact) => sum + place(fact), 0)
                                    + Math.max(0, facts.length - 1) * spacing;
                                const left = foot.room - (waiting.length === 0 ? 0
                                    : place(waiting[0]) + spacing);
                                return Standing.fittedFrom(facts, told => wide(told) <= left,
                                    root.footStage);
                            }
                            readonly property var kept: fit.told.concat(waiting)
                            // The room they take at their least, the glyphs
                            // and the time, with what waits: what the keys
                            // leave them however many a row has.
                            readonly property real least: {
                                const floor = Standing.fittedFrom(facts, () => false, 0).told
                                    .concat(waiting);
                                return floor.reduce((sum, fact) => sum + place(fact), 0)
                                    + Math.max(0, floor.length - 1) * spacing;
                            }

                            // Noted once the binding has settled, since `fit`
                            // reads what this writes.
                            onFitChanged: Qt.callLater(standing.noteStage)

                            function noteStage() {
                                if (root.open && foot.room > 0 && fit.stage > root.footStage)
                                    root.footStage = fit.stage;
                                if (!root.open)
                                    return;
                                const widest = Object.assign({}, root.factsWidest);
                                let grew = false;
                                for (const fact of kept) {
                                    // By its name, which each has and
                                    // none shares: by its label, the time
                                    // and the date, which have none, kept
                                    // one room, and the time kept the
                                    // date's once the date gave way.
                                    const key = `${fact.entry.name}:${fact.way}`;
                                    const now = measure.advanceWidth(fact.said);
                                    if (now > (widest[key] ?? 0)) {
                                        widest[key] = now;
                                        grew = true;
                                    }
                                }
                                if (grew)
                                    root.factsWidest = widest;
                            }
                            // As it's drawn, for a test.
                            readonly property string said: kept.map(fact => fact.said).join("  ")

                            // The room the facts have held, after the last of
                            // them, so the gap between two is always one
                            // item's (docs/design.md, The Ladder: held room at
                            // the far end).
                            readonly property real held: Math.max(width, kept.reduce(
                                (sum, fact) => sum + place(fact), 0)
                                + Math.max(0, kept.length - 1) * spacing)
                            // Where a group after the facts starts, past the
                            // room they've held, as the facts start a group
                            // past the mark (StatusLine.qml's factsAt).
                            readonly property real after: x + held + Theme.space.rung.groups

                            // Where the status line says the facts start, a
                            // group past the mark.
                            x: statusLine.factsAt
                            // On the status line's baseline, as its letters
                            // are, its facts all in the one face.
                            y: statusLine.base - measure.ascent
                            spacing: Theme.space.rung.items

                            // The foot's face: Note, a step under the rows,
                            // its figures tabular.
                            FontMetrics {
                                id: measure
                                font.family: Theme.level.note.family
                                font.pointSize: Theme.level.note.pointSize
                                font.features: { "tnum": 1 }
                            }

                            // A row's name's face, to count the characters
                            // a row has room for.
                            FontMetrics {
                                id: rowFace
                                font.family: Theme.level.name.family
                                font.pointSize: Theme.level.name.pointSize
                                font.weight: Theme.level.name.weight
                            }

                            Repeater {
                                model: standing.kept

                                Text {
                                    required property var modelData

                                    // The room it has needed since the launcher
                                    // opened, so one reading shorter moves
                                    // nothing after it; the gap to the next
                                    // counts from that room (VKCF3Z, 2026-11-05).
                                    width: standing.place(modelData)
                                    text: modelData.said
                                    textFormat: Text.PlainText
                                    // Its figures in text, small enough at Note
                                    // to stay quiet; one that wants Joel as the
                                    // margin's mark wears it (Margin.qml).
                                    color: !modelData.wants ? Theme.text
                                        : modelData.role !== "text" ? Theme[modelData.role]
                                        : Theme[modelData.hue]
                                    font: measure.font
                                }
                            }
                        }

                    }
                }
            }
        }

        // Out of the card's right side, top aligned and as tall as it,
        // its lines set on the card's.
        Drawer {
            id: drawerCard
            x: card.reach + (root.drawerOver ? 0 : root.width + root.drawerGap)
            y: card.reach
            width: root.drawerWidth
            tall: root.drawerOver ? root.rowsEnd : card.height
            // Over the card it stands on the card's rows, its sides on the
            // card's, so it meets them square rather than in a notch.
            squareFoot: root.drawerOver
            fieldBase: root.fieldBase
            rowLine: root.rowLine
            rowDrop: root.rowCaps / 2
            out: root.drawerOut && (!root.drawerOver || root.drawerHolds)
            waits: root.drawerOut && root.drawerOver && !root.drawerHolds
            given: root.drawerShows
            // Its frame says the keys are here while they're in it, gone in
            // with Space and the right arrow.
            keys: QueryKeys.paged(prompt.mode)
            // The search being typed on the line, washed as it's typed.
            seeking: prompt.standing.seek ? prompt.standing.seek.pattern : ""
            seekingForward: !prompt.standing.seek || prompt.standing.seek.forward
            // Visual in it, by its kind, and y's flash, as long as the rows'.
            choosing: prompt.mode === "pageVisual" ? "letters"
                : prompt.mode === "pageVisualLine" ? "lines" : ""
            flashFor: Options.keyTiming["yank-flash"]
        }
    }
}
