// The notices while Super+N holds them: Vim's quickfix list (VKCF3Z's
// vi-paradigm spec, Notices), a line a card, the newest on line 1, and the
// cursor card whole under its line. It's the stack's other face
// (Notifications.qml), drawn in the stack's place while the keys are held,
// and every key does what Vim's does (NoticeKeys.js holds them, `pressed`
// does them, NoticeList.js works out where they land).
//
// Normal stands on a card: j and k move, Return goes to its sender, l
// steps into its acts, d is done with it (marked read where its sender
// offers Read, which is sent at once, and then forgotten), y copies what it
// said, u brings cards back. V chooses several for d or y, / finds among
// them, narrowing the lines as it's typed, and : is the command line, with
// a menu of every command it takes (Ex.js reads it). :q, ZZ, Space w and
// Super+N let them go; Escape at rest does nothing, as Vim's, and q says
// it does nothing.
//
// It's the launcher's card in shape (VKCF3Z's family cut, 2026-11-05): a
// head reading Notices, where / types, as a place does there; a hairline;
// the lines; a hairline; the foot. Each line is the launcher's row: the
// sender's glyph, who or what at Name, its first words at
// Words and when at Note, after a gutter of relative numbers, as Joel's
// Neovim sets number and relativenumber. The foot says how things stand,
// the mode's word in a room of its own, and the cursor card's hints from
// the same table that does the keys, Space ? last.
pragma ComponentBehavior: Bound
import Quickshell
import Quickshell.Services.Notifications
import QtQuick
import KeyTimes
import "Bands.js" as Bands
import "Changes.js" as Changes
import "Columns.js" as Columns
import "Commands.js" as Commands
import "Ex.js" as Ex
import "NoticeKeys.js" as NoticeKeys
import "NoticeList.js" as NoticeList
import "Search.js" as Search
import "launcher/Notice.js" as Notice

Card {
    id: held

    // A head, its rows and its foot, each a band from the frame's inside
    // edge, as the launcher's card is laid.
    bands: true

    // The stack these are (Notifications.qml), for when each arrived, what
    // each opens, the history it hands on to, and whether it holds them.
    required property var stack
    // The notices on screen, as their notifications.
    property var live: []

    keys: true

    // How the keys stand: the mode, and what's begun in it.
    property string mode: "normal"
    property var standing: NoticeKeys.rest()
    readonly property bool pending: standing.operator !== ""

    // What u and Ctrl+R move between, and the cards it holds away from the
    // list, let go for good as the keys go back.
    property var changes: Changes.changes()
    readonly property var awayIds: NoticeList.away(changes.done)
    // The last change, for . to do again.
    property var lastChange: null

    // The lines, the newest first. While visual, an operator or the command
    // line is at work they hold their order, and what arrives waits behind
    // a line saying how many came.
    property var frozen: null
    readonly property var all: NoticeList.ordered(live.map(one => one.id)
        .filter(id => !awayIds.includes(id)), stack.arrived)
    readonly property var rows: frozen === null ? all : frozen.filter(id => all.includes(id))
    readonly property int arrivals: frozen === null ? 0
        : all.filter(id => !frozen.includes(id)).length

    // The find: what was found, kept once its line is left, and the line
    // while it's typed. The lines narrow to what holds it.
    property string found: ""
    property var findLine: NoticeKeys.line("")
    readonly property bool finding: mode === "find" || mode === "command" && commandFrom === "find"
    readonly property string pattern: finding ? findLine.text : found
    readonly property var listed: pattern === "" ? rows : rows.filter(id => held.holds(id, pattern))

    // The cursor, bound to its card, and where it was, for when its card
    // goes.
    property var cursorId: null
    property int cursorAt: 0
    readonly property int cursor: {
        const at = listed.indexOf(cursorId);
        return at >= 0 ? at : Math.max(Math.min(cursorAt, listed.length - 1), 0);
    }
    readonly property var current: notice(listed[cursor])
    // Where it was is kept once the binding's settled, since the cursor
    // reads it, and the cursor's bound to the card it came to, as a find
    // narrowing the lines leaves it on what was found, as Vim's / does.
    onCursorChanged: Qt.callLater(remember)
    function remember() {
        cursorAt = cursor;
        if (listed[cursor] !== undefined)
            cursorId = listed[cursor];
        follow();
    }
    onCurrentChanged: {
        if (stack.holding && current)
            Report.say(`notice cursor ${current.summary}`);
    }

    // Visual's other end, and the last selection, for gv and '< '>.
    property var anchorId: null
    property var lastSelection: null
    readonly property var span: mode === "visual"
        ? NoticeList.chosen(Math.max(listed.indexOf(anchorId), 0), cursor) : null
    readonly property var selected: span === null ? [] : NoticeList.between(listed, span)
    onSelectedChanged: if (mode === "visual") Report.say(`notice selects ${selected.length}`)

    // The view: the first line shown, kept around the cursor, and how many
    // show, as many as fit the screen's height up to eight, and never more
    // than the frame has room for inside the screen's rung.
    property int firstShown: 0
    readonly property int room: Math.max(Math.min(NoticeList.room,
        Math.floor((screenHeight * linesShare) / rowHeight),
        Math.floor(listTall / rowHeight)), 1)
    // How much of the screen's height the lines take at most, leaving the
    // cursor card, the menu and the foot room beneath them. Half, so eight
    // of the launcher's rows fit a screen 768 tall: seven would keep the
    // cursor on the middle line, scrolloff held to half the view.
    readonly property real linesShare: 0.5
    property real screenHeight: 768
    // At rest the cursor's open card takes the lines' room, so they give
    // way round the cursor, down to one, and the frame never passes the
    // screen's rung on a short screen.
    readonly property int shownRoom: Math.max(1, Math.min(room,
        Math.floor((listTall - cardShown) / rowHeight)))
    // How tall the cursor's card stands under its row, as the card says it
    // with the notice it opens, so a card going away never clears the one
    // that replaced it.
    property var cardOf: null
    property real cardTall: 0
    // Taken a moment after the card says it, never while the rows it
    // shapes are being laid, and only when it changes, so a card laid
    // again with the same height leaves them be.
    function cardTold(of, tall) {
        if (cardOf !== of)
            cardOf = of;
        if (cardTall !== tall)
            cardTall = tall;
    }
    readonly property real cardShown: cardOf !== null && cardOf === listed[cursor]
        && mode !== "command" ? cardTall : 0
    readonly property int shownTop: Math.max(firstShown, cursor - shownRoom + 1)
    readonly property var inView: listed.slice(shownTop, shownTop + shownRoom)
    function follow() {
        firstShown = NoticeList.viewTop(firstShown, cursor, listed.length, room);
    }
    onListedChanged: Qt.callLater(follow)

    // Whether the gutter counts from the cursor, as Space u l switches.
    property bool relative: true

    // The cursor card's own acts as the commands that do them, as the card
    // lists them under its words and : puts them first in the menu: a
    // card's acts are commands, never a row stepped into (Joel, 2026-11-05:
    // "where in vi does that step into shit?"). Return goes to its sender.
    readonly property var cardCommands: {
        const one = current;
        if (!one)
            return [];
        const replies = one.hasInlineReply || replyOffer(one) ? ["reply"] : [];
        return replies.concat(offered.map(offer => offer.command)).map(command => `:${command}`);
    }
    readonly property string cardSays: cardCommands.join(" \u00b7 ")
    onCardSaysChanged: if (stack.holding)
        sayOffers()
    function sayOffers() {
        Report.say(`notice offers ${cardSays === "" ? "nothing" : cardSays}`);
    }

    // Space ? lists the keys in the lines' place, scrolling there.
    property bool listing: false
    property int listTop: 0
    readonly property var keyList: NoticeKeys.keyList()
    readonly property var listShown: keyList.slice(listTop, listTop + listRoom)
    onListingChanged: {
        listTop = 0;
        Report.say(listing ? "notice lists keys" : "notice hides keys");
    }
    onListTopChanged: if (listing) Report.say(`notice lists keys from ${listTop + 1}`)

    // The command line, the launcher's own (CommandLine.qml), made while :
    // is open: where it was opened from, the range it opens on, and the
    // line itself, which holds what's typed, its menu narrowed and the line
    // chosen in it.
    property string commandFrom: "normal"
    property string commandRange: ""
    readonly property CommandLine commandItem: commandLoader.item as CommandLine
    readonly property string commandText: commandItem ? commandItem.text : ""
    readonly property var menuShown: commandItem ? commandItem.shown : []
    readonly property var menuChosen: commandItem ? commandItem.chosenLine : null
    onMenuChosenChanged: if (menuChosen !== null)
        Report.say(`notice menu chose ${menuChosen.command}`)
    // Where the keys go back to once the line is let go, which took them.
    property Item keysTo: null

    // A reply's prompt on the card it answers, and its words before the
    // last change in normal over them, for u.
    property var replyTo: null
    property var promptLine: NoticeKeys.line("")
    property string promptBefore: ""

    // What the foot says until the next key: what was just done, or Vim's
    // error for what can't be.
    property string said: ""
    property bool saidWrong: false
    onSaidChanged: if (said !== "") Report.say(`notice says ${said}`)

    onModeChanged: {
        refreeze();
        if (mode !== "command" && keysTo !== null)
            keysTo.forceActiveFocus();
        Qt.callLater(placeNumbers);
        Report.say(`notice mode ${mode}`);
    }
    onPendingChanged: refreeze()
    onPatternChanged: if (pattern !== "") Report.say(`notice finds ${pattern} in ${listed.length}`)

    function refreeze() {
        const holds = mode === "visual" || mode === "command" || pending;
        frozen = holds ? frozen ?? all.slice() : null;
    }

    // The keys taken, and the list as it was before: the cursor on the
    // card in front, which is the one Joel was looking at. Let go, the
    // cards done are let go for good.
    // Taken on the card it was on when let go, the cursor's card changes
    // nothing, so what it offers is said once the cursor has settled.
    function take(front) {
        traces.opened({ cards: live.length });
        reset();
        gutterDigits = 1;
        noteDigits();
        cursorId = front;
        Qt.callLater(sayOffers);
        Qt.callLater(sayFoot);
        Qt.callLater(sayColumns);
    }

    // Whether the cursor's row starts where the rule has it, read from
    // where its parts are drawn, for a test to hold: its number on the
    // inset, only as wide as the most it numbers, its mark in the mark
    // column, and its Name and its card's lines on the text column.
    function sayColumns() {
        const row = lines.itemAt(cursor - shownTop) as Line;
        if (!row)
            return;
        const near = (a, b) => Math.abs(a - b) < 0.01;
        const drawn = numbers.said();
        const ends = drawn.length > 0 ? drawn[0].ends : -1;
        // How far each band stands from where the one before ends, the
        // first from a tie under the head's hairline: none, as Vim's lines
        // tile (Joel, 2026-11-06).
        const gaps = [];
        let end = headRule.y + headRule.height + Theme.space.rung.tie;
        for (let at = 0; at < lines.count; at++) {
            const band = lines.itemAt(at) as Line;
            gaps.push(Math.round(band.mapToItem(sheet, 0, 0).y - end));
            end = band.mapToItem(sheet, 0, 0).y + band.height;
        }
        const ok = near(numbers.x, 0)
            && near(ends, Math.ceil(noteWide("9".repeat(String(listed.length).length))))
            && near(row.markX, columns.mark) && near(row.nameX, columns.text)
            && near(row.cardX, columns.text) && gaps.every(gap => gap === 0);
        Report.say(ok ? "notice rows start on the inset"
            : `notice rows start ${Math.round(numbers.x)}, numbers end ${Math.round(ends)}, `
                + `mark ${Math.round(row.markX)}, name ${Math.round(row.nameX)}, `
                + `card ${Math.round(row.cardX)}, bands apart ${gaps.join(" ")}`);
    }

    // Whether the command line stands in the head as the launcher's does,
    // read from where its parts are drawn, for a test to hold: its : in
    // the head mark's cell and on the head band, what's typed on the text
    // column, and its menu's lines on the columns, tiling from a tie under
    // the head's hairline.
    function sayCommand() {
        const line = commandItem;
        if (line === null)
            return;
        const near = (a, b) => Math.abs(a - b) < 0.5;
        const drawn = Array.from(line.children);
        const colon = drawn.find(one => one.text === ":" && !("cursorPosition" in one));
        const typed = drawn.find(one => "cursorPosition" in one);
        if (!colon || !typed)
            return;
        const at = colon.mapToItem(sheet, 0, 0);
        const centre = at.x + colonInk.tightBoundingRect.x + colonInk.tightBoundingRect.width / 2;
        const inHead = at.y >= 0 && at.y + colon.height <= headRule.y;
        const typedX = typed.mapToItem(sheet, 0, 0).x;
        // Only the rows drawn count: those wholly inside the menu's view,
        // which must hold as many as it has room for.
        const rows = [];
        const view = menuRows.mapToItem(sheet, 0, 0).y;
        let end = headRule.y + headRule.height + Theme.space.rung.tie;
        for (let row = 0; row < menuRows.count; row++) {
            const band = menuRows.itemAtIndex(row) as MenuRow;
            if (!band)
                continue;
            const top = band.mapToItem(sheet, 0, 0);
            if (top.y < view - 0.5 || top.y + band.height > view + menuRows.height + 0.5)
                continue;
            rows.push({ gap: Math.round(top.y - end),
                name: Math.round(top.x + band.nameX - columns.text) });
            end = top.y + band.height;
        }
        const room = Math.min(menuShown.length, menuRoom);
        const ok = near(centre, cellMiddle) && inHead && near(typedX, columns.text)
            && rows.length === room && room > 0
            && rows.every(one => one.gap === 0 && one.name === 0);
        Report.say(ok ? "notice command stands in the head"
            : `notice command : at ${Math.round(centre)} of ${Math.round(cellMiddle)}, `
                + `in the head ${inHead}, typed at ${Math.round(typedX)} of `
                + `${Math.round(columns.text)}, ${rows.length} rows drawn of ${room} `
                + `(${menuRows.count} listed, room ${menuRoom}, view ${menuRows.height} `
                + `tall), rows ${JSON.stringify(rows)}`);
    }

    // The :'s ink, so sayCommand reads where its middle is drawn.
    TextMetrics {
        id: colonInk
        text: ":"
        font: figure.font
    }

    // Whether the status line reaches the frame, so its text centres in
    // the band the hairline and the frame close, for a test to hold.
    function sayFoot() {
        const short = Math.round(sheet.height - foot.y - statusLine.height);
        Report.say(short === 0 ? "notice foot ends at the frame"
            : `notice foot ends ${short} short of the frame`);
    }

    function reset() {
        mode = "normal";
        standing = NoticeKeys.rest();
        frozen = null;
        found = "";
        findLine = NoticeKeys.line("");
        replyTo = null;
        listing = false;
        lastChange = null;
        lastSelection = null;
        anchorId = null;
        said = "";
        firstShown = 0;
    }

    function letGo() {
        traces.closed({ done: awayIds.length, left: live.length - awayIds.length });
        stack.dismiss(awayIds);
        changes = Changes.changes();
        reset();
    }

    // The live notice an id names, or null.
    function notice(id) {
        return live.find(one => one.id === id) ?? null;
    }

    function senderOf(id) {
        const one = notice(id);
        return one ? one.from.name : "";
    }

    // Whether what a card says, or the name it's signed with, holds what's
    // found.
    function holds(id, sought) {
        const one = notice(id);
        return one !== null && Search.found(sought,
            `${stack.said(one)}\n${one.from.name}`).length > 0;
    }

    // The sender's own Read, where it offers one, which done sends at once,
    // as the launcher's d does (Notice.js).
    function readOffer(one) {
        return Notice.readOffer(one.actions);
    }

    // The sender's own Reply, where it offers one rather than a reply typed
    // here.
    readonly property var replyAct: /reply/i
    function replyOffer(one) {
        return one.actions.find(act => replyAct.test(act.identifier) || replyAct.test(act.text))
            ?? null;
    }

    // What d says it does to the cards it would take: read, where each
    // sender offers it, or else forget.
    function doneWord(ids) {
        return ids.length > 0 && ids.every(id => notice(id) && readOffer(notice(id)) !== null)
            ? "read" : "forget";
    }

    function plural(count, one, many) {
        return count === 1 ? one : many;
    }

    // Done with these: each read at once, by its sender where it offers
    // Read, and forgotten, kept for u until the keys go back; or forgotten
    // unread, as :forget does.
    function finish(ids, kind) {
        if (ids.length === 0) {
            said = "not here";
            return;
        }
        if (kind === "done") {
            for (const id of ids) {
                const one = notice(id);
                Notices.mark(id, true);
                const offer = one ? readOffer(one) : null;
                if (offer)
                    offer.invoke();
            }
        }
        const left = listed.filter(id => !ids.includes(id));
        const from = Math.min(...ids.map(id => listed.indexOf(id)).filter(at => at >= 0));
        cursorId = left[Math.min(from, left.length - 1)] ?? null;
        cursorAt = Math.max(Math.min(from, left.length - 1), 0);
        changes = Changes.changed(changes, { ids, kind });
        const count = ids.length;
        said = `${count} ${kind === "done" ? "done" : "forgotten"} \u00b7 u brings `
            + `${plural(count, "it", "them")} back`;
        Report.say(`notice ${kind === "done" ? "done" : "forgets"} ${count}`);
    }

    // What the cards said onto the clipboard, a blank line apart.
    function copy(ids) {
        const said = ids.map(id => notice(id)).filter(one => one !== null)
            .map(one => stack.said(one));
        if (said.length === 0)
            return;
        Quickshell.clipboardText = said.join("\n\n");
        held.said = `${said.length} copied`;
        Report.say(`copied card ${notice(ids[0])?.summary ?? ""}`);
    }

    // An operator on what its motion takes from the cursor, or on the
    // selection, as Vim's linewise d and y; done in visual, it ends it.
    function operate(operator, motion, count) {
        const list = { count: listed.length, top: firstShown,
            senders: listed.map(id => senderOf(id)) };
        const took = motion === "selection" ? span : NoticeList.taken(motion, cursor, count, list);
        const ids = NoticeList.between(listed, took);
        if (mode === "visual") {
            lastSelection = { anchorId, cursorId };
            mode = "normal";
        }
        if (operator === "copy") {
            cursorId = listed[took.from] ?? cursorId;
            copy(ids);
            return;
        }
        lastChange = motion === "selection"
            ? { operator, motion: "line", count: ids.length } : { operator, motion, count };
        finish(ids, "done");
    }

    function undo(count) {
        const undone = Changes.undo(changes, count);
        if (undone.moved === null) {
            said = "Already at oldest change";
            return;
        }
        changes = undone.history;
        const ids = NoticeList.away(undone.moved);
        cursorId = ids[0];
        said = `${ids.length} brought back`;
        Report.say(`notice brought back ${ids.length}`);
    }

    function redo(count) {
        const redone = Changes.redo(changes, count);
        if (redone.moved === null) {
            said = "Already at newest change";
            return;
        }
        const ids = NoticeList.away(redone.moved);
        const left = listed.filter(id => !ids.includes(id));
        cursorId = left[Math.min(cursor, left.length - 1)] ?? null;
        changes = redone.history;
        said = `${ids.length} taken again`;
    }

    // Whether the cursor's card has somewhere to go: what it offers, or
    // its sender's window.
    function opensCurrent() {
        const one = current;
        return one !== null && (stack.opens(one) !== null
            || Notices.senderWindow(one.appName) !== null);
    }

    // Goes to whoever sent the cursor card: what it's about, by its
    // sender's own act, or else the sender's window, giving the keys back
    // so what's typed next reaches it.
    function open() {
        const one = current;
        const offer = one ? stack.opens(one) : null;
        const window = one ? Notices.senderWindow(one.appName) : null;
        if (!one || !offer && !window) {
            said = "not here";
            return;
        }
        Notices.mark(one.id, true);
        if (offer)
            Notices.open(offer, one.appName);
        else
            Niri.act({ FocusWindow: { id: window.id } });
        stack.holding = false;
    }

    // gx: the cursor card's link opened, its first or, given a count, its
    // {n}th, as Vim's opens the one under the cursor; it's read by opening
    // it, and the keys go back so the page opening takes them.
    function followLink(count) {
        const one = current;
        const links = one ? one.words.links : [];
        const link = links[Math.max(count, 1) - 1] ?? null;
        if (!link) {
            const words = Words.of("notices").said ?? {};
            said = links.length === 0 ? words["no-link"]
                : links.length === 1 ? words["one-link"]
                : String(words.links).replace("{count}", links.length);
            return;
        }
        Notices.mark(one.id, true);
        stack.follow(link.href);
        stack.holding = false;
    }

    // :reply: words given are sent at once where the card takes a reply
    // typed here, or else a prompt opens on it; a sender whose own Reply
    // opens it elsewhere is gone to.
    function reply(words) {
        const one = current;
        if (!one) {
            said = "not here";
            return;
        }
        if (one.hasInlineReply) {
            if (words !== "") {
                send(one, words);
                return;
            }
            replyTo = one.id;
            promptLine = NoticeKeys.line("");
            mode = "insert";
            return;
        }
        const offer = replyOffer(one);
        if (!offer) {
            said = `${one.summary} takes no reply`;
            return;
        }
        Notices.mark(one.id, true);
        offer.invoke();
        stack.holding = false;
    }

    function send(one, words) {
        one.sendInlineReply(words);
        Notices.mark(one.id, true);
        replyTo = null;
        mode = "normal";
        said = `replied to ${one.summary}`;
        Report.say(`notice replied ${one.summary}`);
    }

    // The commands the : line takes here, each its full name, its fixed
    // short form (checked against Neovim's own) and whether it takes a
    // range; then what the cursor card's sender offers, each by its first
    // word, where that word isn't already one of these.
    readonly property var commands: [
        { command: "delete", short: "d", ranged: true },
        { command: "forget", short: "forg", ranged: true },
        { command: "yank", short: "y", ranged: true },
        { command: "reply", short: "rep", ranged: false },
        { command: "open", short: "ope", ranged: false },
        { command: "edit", short: "e", ranged: false },
        { command: "quit", short: "q", ranged: false },
        { command: "xit", short: "x", ranged: false },
        { command: "close", short: "clo", ranged: false },
        { command: "help", short: "h", ranged: false },
        { command: "undo", short: "u", ranged: false },
        { command: "redo", short: "red", ranged: false },
        { command: "set", short: "se", ranged: false }
    ]
    readonly property var offered: {
        const one = current;
        if (!one)
            return [];
        return Notice.offers(one.actions)
            .filter(offer => !replyAct.test(offer.text))
            .map(offer => ({
                command: offer.text.split(" ")[0].toLowerCase(),
                identifier: offer.identifier,
                text: offer.text
            }))
            .filter(offer => /^[a-z]+$/.test(offer.command)
                && !commands.some(ours => ours.command.startsWith(offer.command)
                    || offer.command.startsWith(ours.short)));
    }
    readonly property var commandNames: commands.concat(offered.map(offer => ({
        command: offer.command, short: offer.command, ranged: false
    })))

    // The menu: the cursor card's or the selection's own acts, Return's
    // first; then the list's; then what every surface has.
    readonly property var entries: {
        const one = current;
        const out = [];
        // A line as the launcher's menu reads one (Commands.js): its
        // command and short form, its title, its key, the argument it
        // writes, and whether it takes a range or what's typed after it;
        // `after` is shown after it, as what :g takes.
        const entry = (command, short, title, key, more) => Object.assign({
            command, short, title, key, kind: "surface", argument: "", after: "",
            lowered: ` ${title.toLowerCase()}`, bang: false, takes: false, ranged: false,
            undoable: true
        }, more ?? {});
        if (one && (stack.opens(one) || Notices.senderWindow(one.appName)))
            out.push(entry("open", "ope", Words.name("verb:notice:open"), "\u21b5"));
        if (one && (one.hasInlineReply || replyOffer(one)))
            out.push(entry("reply", "rep", "Reply", ""));
        for (const offer of offered)
            out.push(entry(offer.command, offer.command, offer.text, ""));
        out.push(entry("delete", "d", "Done", "dd", { ranged: true }),
            entry("forget", "forg", Words.name("verb:notice:forget"), "", { ranged: true }),
            // :%d, done with every card at once, as Super+Shift+N puts them
            // all away from anywhere (Joel, 2026-10-21: "a quick way to
            // dismiss them all").
            entry("delete", "d", Words.name("verb:notices:done"), "", { every: true }),
            entry("yank", "y", Words.name("verb:notice:copy"), "yy", { ranged: true }),
            entry("edit", "e", Words.name("verb:notices:all"), "",
                { argument: "Notices", takes: true }),
            entry("global", "g", "On Each Found", "", { after: "/pat/cmd", takes: true,
                ranged: true }),
            entry("undo", "u", "Bring Them Back", "u"),
            entry("redo", "red", "Take Them Again", "Ctrl+R"),
            entry("quit", "q", "Let Them Go", "ZZ"),
            entry("help", "h", "These Keys", "Space ?"),
            entry("set", "se", "Relative Numbers", "Space u l",
                { argument: "rnu!", takes: true }),
            entry("{n}", "{n}", "Go to Line n", "nG", { kind: "row", row: cursor + 1 }));
        return out;
    }

    // What a chosen menu line acts on, as a drawer would name it.
    readonly property string actsOn: {
        const chosenEntry = menuChosen;
        if (chosenEntry === null)
            return "";
        const ids = commandFrom === "visual" || commandText.startsWith(Ex.selected)
            ? (lastSelection ? selectedOf(lastSelection) : [listed[cursor]]) : [listed[cursor]];
        const names = ids.map(id => senderOf(id)).filter(name => name !== "");
        return ids.length === 1 ? `on ${notice(ids[0])?.summary ?? ""}`
            : `on ${ids.length} notices: ${names.join(", ")}`;
    }

    function selectedOf(selection) {
        const from = listed.indexOf(selection.anchorId);
        const to = listed.indexOf(selection.cursorId);
        return from < 0 || to < 0 ? [] : NoticeList.between(listed, NoticeList.chosen(from, to));
    }

    // Runs what's typed on the command line, exactly, as Vim's Return does.
    function run(text) {
        const from = commandFrom;
        mode = from === "prompt" ? "prompt" : "normal";
        const asked = Ex.parse(text, commandNames);
        if (asked.error) {
            wrong(asked.error);
            return;
        }
        const marks = {};
        if (lastSelection) {
            const chosenIds = selectedOf(lastSelection);
            if (chosenIds.length > 0) {
                marks["<"] = listed.indexOf(chosenIds[0]);
                marks[">"] = listed.indexOf(chosenIds[chosenIds.length - 1]);
            }
        }
        const at = { cursor, count: listed.length, marks };
        if (asked.global) {
            everywhere(asked, at);
            return;
        }
        const rows = Ex.resolve(asked.range, at);
        if (rows.error && (asked.range !== null || listed.length > 0)) {
            wrong(rows.error);
            return;
        }
        const ids = (rows.rows ?? []).map(row => listed[row]);
        does(asked, ids, from);
    }

    function wrong(error) {
        said = error;
        saidWrong = true;
    }

    // :g/pat/cmd and :v: the inner command on every line in the range that
    // holds the pattern, or doesn't; with none, the lines narrow to it, as
    // Vim lists them.
    function everywhere(asked, at) {
        const rows = Ex.resolve(asked.range, at);
        if (rows.error) {
            wrong(rows.error);
            return;
        }
        const ids = rows.rows.map(row => listed[row])
            .filter(id => held.holds(id, asked.global.pattern) !== asked.global.invert);
        const inner = asked.global.command;
        if (inner.name === "") {
            found = asked.global.pattern;
            return;
        }
        if (!["delete", "forget", "yank"].includes(inner.name)) {
            wrong("E481: No range allowed");
            return;
        }
        if (ids.length === 0) {
            wrong(`E486: Pattern not found: ${asked.global.pattern}`);
            return;
        }
        does(inner, ids, "normal");
    }

    function does(asked, ids, from) {
        switch (asked.name) {
        case "":
            if (ids.length > 0)
                cursorId = ids[ids.length - 1];
            break;
        case "delete":
            finish(ids, "done");
            break;
        case "forget":
            finish(ids, "forget");
            break;
        case "yank":
            copy(ids);
            break;
        case "reply":
            reply(asked.args);
            break;
        case "open":
            open();
            break;
        case "edit":
            if (asked.args === "")
                wrong("E32: No file name");
            else if (asked.args.toLowerCase() === "notices")
                stack.history();
            else
                wrong(`E447: Can't find file "${asked.args}" in path`);
            break;
        case "quit":
        case "xit":
        case "close":
            if (from === "prompt") {
                replyTo = null;
                mode = "normal";
            } else {
                stack.holding = false;
            }
            break;
        case "help":
            listing = true;
            listTop = NoticeKeys.helpAt(keyList, asked.args);
            break;
        case "undo":
            undo(0);
            break;
        case "redo":
            redo(0);
            break;
        case "set":
            setting(asked.args);
            break;
        default: {
            const offer = offered.find(one => one.command === asked.name);
            const one = current;
            const action = one && offer
                ? one.actions.find(act => act.identifier === offer.identifier) : null;
            if (action) {
                Notices.mark(one.id, true);
                action.invoke();
                stack.holding = false;
            }
        }
        }
    }

    // :set's one option here, Vim's relativenumber.
    function setting(option) {
        const names = { rnu: true, relativenumber: true, nornu: false, norelativenumber: false };
        if (option === "" || option === "rnu?" || option === "relativenumber?") {
            said = relative ? "  relativenumber" : "norelativenumber";
        } else if (option in names) {
            relative = names[option];
        } else if (["rnu!", "relativenumber!", "invrnu", "invrelativenumber"].includes(option)) {
            relative = !relative;
        } else {
            wrong(`E518: Unknown option: ${option}`);
        }
    }

    // A key as the table names it (NoticeKeys.js): a modifier, then the
    // letter it typed, or else its name, Shift unnamed beside a letter.
    readonly property var keyNames: ({
        [Qt.Key_Return]: "Return",
        [Qt.Key_Enter]: "Enter",
        [Qt.Key_Delete]: "Delete",
        [Qt.Key_Backspace]: "Backspace",
        [Qt.Key_Escape]: "Escape",
        [Qt.Key_Space]: "Space",
        [Qt.Key_Tab]: "Tab",
        [Qt.Key_Backtab]: "Tab",
        [Qt.Key_Home]: "Home",
        [Qt.Key_End]: "End",
        [Qt.Key_F1]: "F1",
        [Qt.Key_Left]: "Left",
        [Qt.Key_Up]: "Up",
        [Qt.Key_Right]: "Right",
        [Qt.Key_Down]: "Down"
    })
    function named(event) {
        const control = (event.modifiers & Qt.ControlModifier) !== 0;
        const shift = (event.modifiers & Qt.ShiftModifier) !== 0;
        // With Ctrl a letter types a control character, so it goes by name.
        const letter = !control && /^[!-~]$/.test(event.text) ? event.text : "";
        const byKey = control && event.key >= Qt.Key_A && event.key <= Qt.Key_Z
            ? String.fromCharCode(event.key) : "";
        const name = letter || (keyNames[event.key] ?? byKey);
        if (name === "")
            return "";
        return (control ? "Ctrl+" : "") + (shift && !letter ? "Shift+" : "") + name;
    }

    // Every key while the notices are held, in the mode it's pressed in.
    function pressed(event) {
        event.accepted = true;
        const key = named(event);
        if (key === "")
            return;
        said = "";
        saidWrong = false;
        const repeated = event.isAutoRepeat;
        switch (mode) {
        case "find":
            finds(key, repeated);
            break;
        case "insert":
            inserted(key, repeated);
            break;
        case "prompt":
            prompted(key);
            break;
        default:
            normal(key, repeated);
        }
    }

    // A key on the find line, which narrows the lines as it's typed; Return
    // goes to the chosen card's sender, and Escape gives normal on the
    // lines as found.
    function finds(key, repeated) {
        const typed = NoticeKeys.typing(findLine, key, "find", repeated);
        findLine = typed.line;
        switch (typed.act) {
        case "open":
            found = findLine.text;
            mode = "normal";
            // Where what it found opens nothing, it lands on it, as Vim's
            // / lands on its match (VKCF3Z, 2026-11-06).
            if (opensCurrent())
                open();
            else
                Report.say(`notice lands on ${found}`);
            break;
        case "choose":
        case "chooseBack":
            cursorId = listed[Math.max(0, Math.min(cursor + (typed.act === "choose" ? 1 : -1),
                listed.length - 1))] ?? cursorId;
            break;
        case "take":
            if (current)
                findLine = NoticeKeys.line(current.summary);
            break;
        case "paste":
            findLine = pasted(findLine);
            break;
        case "command":
            opensCommand("find", "");
            break;
        case "normal":
        case "leave":
            found = findLine.text;
            mode = "normal";
            break;
        }
    }

    function pasted(line) {
        const words = (Quickshell.clipboardText ?? "").split("\n")[0];
        return NoticeKeys.line(line.text.slice(0, line.caret) + words
            + line.text.slice(line.caret), line.caret + words.length);
    }

    // Opens the command line on the range given, as the launcher's : opens
    // (CommandLine.qml), which takes the keys until it's let go.
    function opensCommand(from, range) {
        commandFrom = from;
        commandRange = range;
        // Its gutter holds the menu's numbers, as the launcher's does.
        gutterDigits = Math.max(gutterDigits, String(entries.length).length);
        mode = "command";
    }

    // The line let go, by Escape or by Backspace on it empty, gives back
    // the mode it was opened from, putting back what it covered.
    function commandLeft() {
        mode = commandFrom === "find" ? "find" : commandFrom === "prompt" ? "prompt"
            : "normal";
    }

    // A key in a reply's prompt, typing its words; Return sends them and
    // Escape gives normal over them, the caret one left.
    function inserted(key, repeated) {
        const typed = NoticeKeys.typing(promptLine, key, "insert", repeated);
        promptLine = typed.line;
        const one = notice(replyTo);
        switch (typed.act) {
        case "send":
            if (one)
                send(one, promptLine.text);
            break;
        case "normal":
            promptBefore = promptLine.text;
            promptLine = NoticeKeys.line(promptLine.text, Math.max(promptLine.caret - 1, 0));
            mode = "prompt";
            break;
        case "paste":
            promptLine = pasted(promptLine);
            break;
        }
    }

    // A key in normal over a reply's words: Vim's editing, : for :q to drop
    // it, and Return to send it.
    function prompted(key) {
        const typed = NoticeKeys.prompting(standing, promptLine, key);
        standing = typed.standing;
        if (typed.line.text !== promptLine.text)
            promptBefore = promptLine.text;
        promptLine = typed.line;
        const one = notice(replyTo);
        switch (typed.act) {
        case "insert":
            mode = "insert";
            break;
        case "send":
            if (one)
                send(one, promptLine.text);
            break;
        case "command":
            opensCommand("prompt", "");
            break;
        case "undo":
            promptLine = NoticeKeys.line(promptBefore, Math.min(promptLine.caret,
                Math.max(promptBefore.length - 1, 0)));
            break;
        case "notHere":
            said = "not here";
            break;
        }
    }

    // A key in normal or visual, from the one table.
    function normal(key, repeated) {
        const read = NoticeKeys.read(standing, key, mode, { repeated, now: Date.now() });
        standing = read.standing;
        // Escape puts the key list away, as Vim's cancels which-key; at
        // rest it does nothing, so the table reads it as nothing.
        if (listing && key === "Escape") {
            listing = false;
            return;
        }
        if (read.act === "" || read.act === "pending") {
            if (read.act === "pending")
                Report.say(`notice pending ${standing.operator}`);
            return;
        }
        if (listing) {
            scrollsKeys(read);
            return;
        }
        const startedAt = Date.now();
        const started = KeyTimes.now();
        const from = mode;
        act(read);
        traces.did("act", startedAt, { act: read.act, count: read.count ?? 1, mode: from,
            cards: listed.length, tookMs: KeyTimes.now() - started });
        standing = NoticeKeys.settled(standing, Date.now());
    }

    // Each hold a trace (Traces.qml) of the service "notices": the keys
    // taken its root, ended as they're let go with how many cards were
    // done and how many left, and each act a key does in normal a span
    // under it, saying what it did, how many times, to how many cards and
    // how long it took by the shell's clock (Y2YBTS, 2026-11-07).
    Traces {
        id: traces
        service: "notices"
        rootName: "hold"
    }

    // Keys while the key list shows: the moves scroll it, and Escape, q
    // and Space ? put it away.
    function scrollsKeys(read) {
        const last = listLast;
        const by = Math.max(read.count, 1);
        const half = Math.floor(listRoom / 2);
        const scrolls = {
            down: listTop + by, up: listTop - by, first: 0, last,
            halfDown: listTop + half * by, halfUp: listTop - half * by,
            viewDown: listTop + listRoom * by, viewUp: listTop - listRoom * by
        };
        if (read.act in scrolls)
            listTop = Math.max(0, Math.min(scrolls[read.act], last));
        else if (["back", "keys", "quiet", "close"].includes(read.act))
            listing = false;
        else
            said = "not here";
    }

    // The view scrolled as Vim's is, the cursor kept in it (NoticeList's
    // scrolled): Ctrl+E Ctrl+Y, zz zt zb and the wheel.
    readonly property var scrolls: ["lineDown", "lineUp", "viewCentre", "viewAtTop",
        "viewAtBottom"]
    function scroll(motion, count) {
        const to = NoticeList.scrolled(motion, cursor, count,
            { count: listed.length, top: firstShown, shown: room });
        firstShown = to.top;
        cursorId = listed[to.cursor] ?? cursorId;
    }
    onFirstShownChanged: if (stack.holding) Report.say(`notice view from ${firstShown + 1}`)

    function act(read) {
        const count = read.count;
        const motions = ["down", "up", "first", "last", "viewTop", "viewMiddle", "viewBottom",
            "runOn", "runBack", "halfDown", "halfUp", "viewDown", "viewUp"];
        if (motions.includes(read.act)) {
            const to = NoticeList.moved(read.act, cursor, count,
                { count: listed.length, top: firstShown, senders: listed.map(id => senderOf(id)) });
            cursorId = listed[to] ?? cursorId;
            return;
        }
        if (scrolls.includes(read.act)) {
            scroll(read.act, count);
            return;
        }
        switch (read.act) {
        case "operate":
            operate(read.operator, read.motion, count);
            break;
        case "inRun": {
            const run = NoticeList.run(listed.map(id => senderOf(id)), cursor);
            anchorId = listed[run.from];
            cursorId = listed[run.to];
            break;
        }
        case "open":
            open();
            break;
        case "link":
            followLink(count);
            break;
        case "repeat":
            if (lastChange)
                operate(lastChange.operator, lastChange.motion,
                    count > 0 ? count : lastChange.count);
            break;
        case "undo":
            undo(count);
            break;
        case "redo":
            redo(count);
            break;
        case "select":
            if (mode === "visual") {
                lastSelection = { anchorId, cursorId };
                mode = "normal";
            } else if (current) {
                anchorId = current.id;
                cursorId = current.id;
                mode = "visual";
            }
            break;
        case "otherEnd": {
            const other = anchorId;
            anchorId = cursorId ?? listed[cursor];
            cursorId = other;
            break;
        }
        case "reselect":
            if (lastSelection && selectedOf(lastSelection).length > 0) {
                anchorId = lastSelection.anchorId;
                cursorId = lastSelection.cursorId;
                mode = "visual";
            }
            break;
        case "find":
            findLine = NoticeKeys.line("");
            found = "";
            mode = "find";
            break;
        case "findStart":
        case "findEnd":
            findLine = NoticeKeys.line(found, read.act === "findStart" ? 0 : found.length);
            mode = "find";
            break;
        case "command":
            if (mode === "visual") {
                lastSelection = { anchorId, cursorId };
                mode = "normal";
                opensCommand("visual", Ex.selected);
            } else {
                opensCommand("normal", count > 0 ? Ex.counted(count) : "");
            }
            break;
        case "close":
            stack.holding = false;
            break;
        case "quiet":
            said = "q does nothing \u00b7 :q closes";
            break;
        case "keys":
            listing = true;
            break;
        case "numbers":
            relative = !relative;
            Report.say(`notice numbers ${relative ? "relative" : "absolute"}`);
            break;
        // Escape ends visual, and never lets the notices go (Joel,
        // 2026-11-05: "fuck escape quitting").
        case "back":
            if (mode === "visual") {
                lastSelection = { anchorId, cursorId };
                mode = "normal";
            }
            break;
        default:
            said = "not here";
        }
    }

    // The mode, at the foot's start in three capitals (NoticeKeys.mark).
    readonly property string modeMark: NoticeKeys.mark(mode)
    onModeMarkChanged: Report.say(`notice shows ${held.modeMark}`)

    // What the mode is doing, in the room kept for its widest beside the
    // facts: how many visual has chosen, a count being typed, or an
    // operator waiting with what it will take. The mode itself is named
    // at the foot's start, so this never says it again.
    readonly property string modeWord: {
        if (mode === "visual")
            return `${selected.length} chosen`;
        if (pending) {
            const word = standing.operatorAct === "done" ? doneWord([listed[cursor]])
                : "copy";
            return `${standing.operator} \u00b7 ${word} ${current?.summary ?? ""}`;
        }
        return standing.count;
    }

    // How things stand: which line of how many are listed. What a find
    // found is counted beside it in the head, as Vim's search count sits
    // beside the search, so the foot keeps only the list's own facts
    // (VKCF3Z, 2026-11-05).
    readonly property string facts: actsOn !== "" && mode === "command" ? actsOn
        : `${listed.length === 0 ? 0 : cursor + 1} of ${listed.length}`
    // The find's count says what it found, never "of" beside the
    // foot's position, which means something else (VKCF3Z, 2026-11-05).
    readonly property string foundCount: `${listed.length} found`

    // The hints for the cursor card in this mode, from the table.
    readonly property var hints: {
        // The command line names its own keys: Tab chooses until a line is
        // chosen or typed, and then Return runs it.
        if (mode === "command") {
            if (commandText === "" && menuChosen === null)
                return [{ act: "choose", key: "Tab", word: "choose" }];
            const asked = Ex.parse(commandText, commandNames);
            const name = asked.global ? "global" : asked.name ?? "";
            return [{ act: "run", key: "\u21b5", word: name === "" ? "run" : name }];
        }
        const hintMode = listing ? "listing" : pending ? "pending" : mode;
        const ids = mode === "visual" ? selected : [listed[cursor]];
        return NoticeKeys.hints(hintMode, { open: opensCurrent() }, { done: doneWord(ids) });
    }

    // A held notice is a notice row of the launcher's (VKCF3Z,
    // 2026-11-05), so it takes the launcher's row (Choice.qml): a band four
    // beats tall, its Name's capitals centred in it, the bands meeting with
    // no gap and the cursor's ground filling its band edge to edge, as
    // Vim's cursorline (Joel, 2026-11-06). The cursor's card makes its band
    // taller, keeping as much room under its last line as over its first.
    readonly property real rowHeight: 4 * Theme.space.beat
    readonly property real rowCaps: nameCaps.tightBoundingRect.height
    // Where a row's Name has its baseline, down from the row's top.
    readonly property real rowBase: (rowHeight + rowCaps) / 2
    // What a row leaves under its baseline, which the last of anything
    // leaves too, so the hairline under it always stands as far off.
    readonly property real rowLeaves: (rowHeight - rowCaps) / 2
    // Where rich text sets the Words' first baseline, down from its top. Qt
    // lays a rich line of fixed height with its baseline four fifths down it
    // (QTextDocumentLayout's lineAdjustment), while it reports the face's
    // ascent as the baseline, as it is for styled text; placed by that, a
    // washed card's Words dropped 3 device pixels at 2 under a find.
    readonly property real richBase: Theme.level.words.pitch * 4 / 5
    // The columns, a held row being the pop-up's notice in a list and a
    // launcher row in its place (VKCF3Z, 2026-11-06): its number on the
    // inset, the gutter only as wide as the most it numbers, the sender's
    // mark in the mark column and every line of the row, its Name, words,
    // signature and acts, on the text column, so it differs from the
    // pop-up's notice by the gutter alone and from a launcher row by the
    // gutters' widths alone (Joel, 2026-11-06: "they need to share more
    // code and UI/UX design to stop them from constantly drifting").
    readonly property var columns: Columns.across(sheet.width, gutter,
        Columns.ofMark(Theme.level.name.pitch), Theme.space.rung)
    // How much wider than a pop-up notice the list stands, so a notice
    // breaks its words held as it does popped (VKCF3Z, 2026-11-06): its
    // gutter, the numbers' room and the mark rung after them.
    readonly property real gutter: Columns.gutterOf(numbers.room, Theme.space.rung)
    // Where the head's mark centres its ink across, which the mode's marks
    // in its cell, the find's / and the :, centre on, as Vim keeps one cell
    // for its mode (Y2YBTS, 2026-11-06).
    readonly property real cellMiddle: headMark.inkCentre
    // Space ?'s pitch, the drawer's key list's: Figure's, at Note.
    readonly property real keyPitch: Theme.level.figure.pitch
    // How many of its lines show: as many as stand within the rows' room,
    // the first on the first row's baseline and the last a row's leaves
    // over where the last row's box would end.
    readonly property real listSpare: (room - 1) * rowHeight
    readonly property var listSteps: ({ pitch: keyPitch, beat: Theme.space.beat })
    readonly property int listRoom: NoticeKeys.shownFrom(keyList, listTop, listSpare, listSteps)
    // The furthest it scrolls, its last line shown.
    readonly property int listLast: NoticeKeys.lastTop(keyList, listSpare, listSteps)

    // The faces, measured by TextMetrics where a binding reads what's
    // measured, since a FontMetrics call in a binding is asked once, before
    // the face has loaded; the calls below read the face first, so they're
    // asked again as it does.
    FontMetrics {
        id: note
        font.family: Theme.level.note.family
        font.pointSize: Theme.level.note.pointSize
        font.weight: Theme.level.note.weight
    }

    TextMetrics {
        id: nameCaps
        text: "H"
        font.family: Theme.level.name.family
        font.pointSize: Theme.level.name.pointSize
        font.weight: Theme.level.name.weight
    }

    // A line in view, its band, saying where its parts are drawn so
    // sayColumns can read them by type, and which it is, for its number.
    component Line: Item {
        property int at
        property real markX
        property real nameX
        property real cardX
    }

    // A line of the command line's menu, its band, saying where its name
    // stands across and its baseline down, for sayCommand and its number.
    component MenuRow: Item {
        property real nameX
        property real nameBaseline
    }

    // The gutter's room, by the widest it holds: as many figures as the
    // most cards held since the keys were taken, so a card arriving or
    // going never moves the rows, and ten widen it only once. Its numbers
    // take as many figures as the lines listed need, on the inset.
    property int gutterDigits: 1
    onRowsChanged: noteDigits()
    function noteDigits() {
        gutterDigits = Math.max(gutterDigits, String(Math.max(1, rows.length)).length);
    }

    // Where each line in view has its Name's baseline, down from the first
    // line's top, for its number: set as the lines are laid, since a
    // binding can't follow the lines a Repeater makes.
    property var numbered: []
    function placeNumbers() {
        const out = [];
        if (mode === "command") {
            for (let at = 0; at < menuRows.count; at += 1) {
                const row = menuRows.itemAtIndex(at) as MenuRow;
                if (row)
                    out.push({ at, base: row.y - menuRows.contentY + row.nameBaseline });
            }
            numbered = out;
            return;
        }
        for (let at = 0; at < lines.count; at += 1) {
            const line = lines.itemAt(at) as Line;
            if (line)
                out.push({ at: line.at, base: line.y + rowBase });
        }
        numbered = out;
    }

    // How wide a run of Note's is, asked again as the face loads.
    function noteWide(text: string): real {
        void note.font;
        return note.advanceWidth(text);
    }

    FontMetrics {
        id: wordsFace
        font.family: Theme.level.words.family
        font.pointSize: Theme.level.words.pointSize
        font.weight: Theme.level.words.weight
        font.variableAxes: Theme.level.words.variableAxes
    }

    // How wide a run of a card's words is, the same way; asked, not set
    // on a TextMetrics, whose width the asking binding would then hang on.
    function wordsWide(text: string): real {
        void wordsFace.font;
        return wordsFace.advanceWidth(text);
    }

    // How wide a key list's chords are drawn: each key of a chord a tie
    // from the next, each chord apart from the next.
    function chordsWide(chords): real {
        return chords.reduce((sum, chord, at) => sum + (at > 0 ? Theme.space.rung.apart : 0)
            + chord.split(" ").reduce((wide, key, part) => wide
                + (part > 0 ? Theme.space.rung.tie : 0) + held.noteWide(key), 0), 0);
    }
    // The keys' column, as wide as the widest line's chords.
    readonly property real keysWide: keyList.reduce((most, one) =>
        Math.max(most, chordsWide(one.chords)), 0)
    // Where what the keys do starts, a group past the widest chords.
    readonly property real doesAt: keysWide + Theme.space.rung.groups

    FontMetrics {
        id: figure
        font.family: Theme.level.figure.family
        font.pointSize: Theme.level.figure.pointSize
        font.weight: Theme.level.figure.weight
    }

    TextMetrics {
        id: titleCaps
        text: "H"
        font.family: Theme.level.title.family
        font.pointSize: Theme.level.title.pointSize
        font.weight: Theme.level.title.weight
        font.variableAxes: Theme.level.title.variableAxes
        font.letterSpacing: Theme.level.title.letterSpacing
    }

    TextMetrics {
        id: titleSmall
        text: "x"
        font: titleCaps.font
    }

    // The head is a band, the frame's inside to its hairline, its one line
    // centred in it by eye (docs/design.md: text in a band centres in it):
    // a row's band, as the launcher's head is now its Title is the line's
    // size, so the card keeps one rhythm (VKCF3Z, 2026-11-06). Its largest
    // level is always the Title, which centres it.
    readonly property real headTall: rowHeight
    readonly property real headBase: Bands.baseline(headTall, titleCaps.tightBoundingRect.height,
        titleSmall.tightBoundingRect.height)

    // The caret's blink, as the launcher's and his kitty's: a key shows it
    // whole, it fades out and back in, and with no key for a while it holds
    // steady, so a line left still draws nothing more (Prompt.qml).
    readonly property bool typing: mode === "find" || mode === "insert"
    readonly property string lineText: mode === "find" ? findLine.text
        : mode === "insert" ? promptLine.text : ""
    property real caretLight: 1
    readonly property bool blinks: typing && visible && Options.caret.fade > 0
    onLineTextChanged: wakes()
    onBlinksChanged: wakes()
    function wakes() {
        blinking.stop();
        caretLight = 1;
        if (blinks) {
            blinking.start();
            stilled.restart();
        }
    }
    SequentialAnimation {
        id: blinking
        loops: Animation.Infinite
        NumberAnimation {
            target: held
            property: "caretLight"
            from: 1
            to: 0
            duration: Options.caret.fade
        }
        NumberAnimation {
            target: held
            property: "caretLight"
            from: 0
            to: 1
            duration: Options.caret.fade
        }
    }
    Timer {
        id: stilled
        interval: Options.caret.stopsAfter
        onTriggered: {
            blinking.stop();
            held.caretLight = 1;
        }
    }

    // A match's ink and ground, as Joel's Neovim draws a search's.
    // On a chosen line, which lies on the selection, its ground is the
    // theme's for a selected row, so the match still shows.
    function wash(chosen) {
        return {
            ink: `${Theme.wash.ink}`,
            ground: `${chosen ? Theme.wash.selected : Theme.wash.ground}`
        };
    }

    // The card's content, the inset in on either side and from the frame's
    // top to its foot, a band at a time: the head, its hairline, the lines
    // a tie under it, a tie over the foot's hairline, and the foot, its
    // baseline the inset over the frame. What reaches the frame, a ground
    // or the status line, reaches out past the content by the card's
    // inset.
    Column {
        id: sheet
        width: parent.width
        // The foot's room under its row, so its baseline, where the row's
        // box puts it, stands the inset over the frame (Launcher.qml's).
        bottomPadding: Math.round(held.inset
            - (held.rowHeight - (held.rowHeight - note.height) / 2 - note.ascent))

        // The wheel scrolls three lines a notch, as his Neovim's mousescroll
        // does, the cursor kept in view; over the key list, the list.
        WheelHandler {
            acceptedDevices: PointerDevice.Mouse | PointerDevice.TouchPad
            onWheel: event => {
                const notches = Math.round(-event.angleDelta.y / 120);
                if (notches === 0)
                    return;
                if (held.listing) {
                    held.listTop = Math.max(0, Math.min(held.listTop + 3 * notches,
                        held.listLast));
                    return;
                }
                held.scroll(notches > 0 ? "lineDown" : "lineUp", 3 * Math.abs(notches));
            }
        }

        // The head: where Joel is, the notices' mark and their Title, as a
        // place in the launcher reads, on the rows' columns. The mark
        // stands in the mark column, where the find's / takes its cell
        // while it lasts, as Vim keeps one cell for its mode, so nothing
        // after it moves between normal and / (Y2YBTS, 2026-11-06, as the
        // launcher's head). The Title stands on the text column, the
        // line's size and set apart by its weight (Joel, 2026-11-06: "use
        // weights and not size for that"), and what's typed follows it at
        // Figure, on its baseline (VKCF3Z's family mocks, 2026-11-05).
        Item {
            id: head

            width: sheet.width
            height: held.headTall

            // How far in the caret stands.
            TextMetrics {
                id: soughtBefore
                font: figure.font
                text: held.findLine.text.slice(0, held.findLine.caret)
            }

            // The /'s ink, so its middle stands on the cell's.
            TextMetrics {
                id: slashInk
                font: figure.font
                text: "/"
            }

            Mark {
                id: headMark
                x: held.columns.mark
                line: held.headBase
                visible: held.mode !== "find" && held.mode !== "command"
                text: Theme.glyph.notifications
                color: Theme.accent
            }

            Text {
                x: held.cellMiddle - slashInk.tightBoundingRect.x
                    - slashInk.tightBoundingRect.width / 2
                y: held.headBase - baselineOffset
                visible: held.mode === "find"
                text: "/"
                color: Theme.accent
                font: figure.font
            }

            // The Title, then what's sought, typed or kept once its line is
            // left, and how many it found, a mark each side of the dot, as
            // the hints are joined; the count stays whole, the sought words
            // giving way before it.
            Row {
                id: placed
                x: held.columns.text
                y: held.headBase - place.baselineOffset
                visible: held.mode !== "command"
                spacing: Theme.space.rung.items

                Text {
                    id: place
                    text: "Notices"
                    // The Title level whole, its weight and colour as the
                    // launcher's place draws it (VKCF3Z, 2026-11-05).
                    color: Theme.level.title.colour
                    font: titleCaps.font
                }

                // Shown while a find types, so its caret shows before a
                // letter has.
                Row {
                    visible: held.pattern !== "" || held.mode === "find"
                    spacing: Theme.space.rung.mark

                    Text {
                        id: sought
                        y: place.baselineOffset - baselineOffset
                        width: Math.max(0, Math.min(implicitWidth, head.width - placed.x
                            - place.width - Theme.space.rung.items - 2 * Theme.space.rung.mark
                            - dot.implicitWidth - count.implicitWidth))
                        text: held.pattern
                        textFormat: Text.PlainText
                        elide: Text.ElideLeft
                        // Strong while it types, as every line being typed
                        // is; the text colour once kept, a fact like the
                        // count.
                        color: held.mode === "find" ? Theme.strong : Theme.text
                        font: figure.font

                        Rectangle {
                            visible: held.mode === "find"
                            opacity: held.caretLight
                            x: soughtBefore.advanceWidth
                            y: sought.baselineOffset - figure.ascent
                            width: Math.max(1, Math.round(figure.averageCharacterWidth / 4))
                            height: figure.ascent + figure.descent
                            color: Theme.strong
                        }
                    }

                    Text {
                        id: dot
                        visible: held.pattern !== ""
                        y: place.baselineOffset - baselineOffset
                        text: "\u00b7"
                        color: Theme.muted
                        font: note.font
                    }

                    Text {
                        id: count
                        visible: held.pattern !== ""
                        y: place.baselineOffset - baselineOffset
                        text: held.foundCount
                        color: Theme.muted
                        font: note.font
                    }
                }
            }

            // The command line, the launcher's own, in the head while : is
            // open, as Vim's : and / share one line (#2042): its : in the
            // head mark's cell, where the find's / stands, and what's typed
            // on the text column, over the names it completes. From the
            // frame's inside edge to its other, as the launcher's stands.
            Loader {
                id: commandLoader
                x: -held.inset
                width: head.width + 2 * held.inset
                height: head.height
                active: held.mode === "command"
                onLoaded: {
                    const line = item as CommandLine;
                    line.take(held.commandRange, "");
                    Report.say(`notice menu ${line.shown.length}`);
                    Qt.callLater(held.sayCommand);
                }

                sourceComponent: CommandLine {
                    markMiddle: held.cellMiddle + held.inset
                    typedAt: held.columns.text + held.inset
                    menu: held.entries
                    onRan: read => held.run(read.typed)
                    onLetGo: () => held.commandLeft()
                }
            }
        }

        // The head's hairline, inset to inset, spanning the ink: a rule
        // meeting the frame would cut the card into boxes (VKCF3Z's family
        // cut, 2026-11-05).
        Rectangle {
            id: headRule
            width: sheet.width
            height: Theme.border
            color: Theme.edge
        }

        // A tie between the hairline and the first band, so a ground never
        // touches a rule, as the launcher's rows stand (VKCF3Z, 2026-11-06).
        Item {
            width: sheet.width
            height: Theme.space.rung.tie
        }

        // The lines in view, numbered, or in their place Space ?'s list or
        // the command line's menu.
        Item {
            width: sheet.width
            height: held.mode === "command" ? Math.max(menuRows.height, held.rowHeight)
                : inViewLines.height

            Column {
                id: inViewLines
                width: parent.width

                Repeater {
                    id: lines
                    model: held.listing || held.mode === "command" ? [] : held.inView
                    onItemAdded: Qt.callLater(held.placeNumbers)
                    onItemRemoved: Qt.callLater(held.placeNumbers)
                    delegate: lineBand
                }
            }

            // The launcher's gutter, over the bands' grounds, which reach
            // under it: the cursor line's own number in text, the rest how
            // far from it, muted, on the inset.
            //
            // On the command line, its menu's lines, each a launcher row
            // (Choice.qml) on the columns: its command in Vim's help
            // notation on the text column under what's typed, what it does
            // in its words and its key at the far end, never a menu
            // indented under the line (Joel, 2026-11-06: "i don't like how
            // auto completions are indented, they signal the wrong
            // organizational concept"). From the frame's inside edge to its
            // other, so the chosen line's ground fills its band.
            ListView {
                id: menuRows
                x: -held.inset
                width: parent.width + 2 * held.inset
                // From the lines it's given, not its own count: read from
                // that, it stood no taller than nothing with eleven listed.
                height: Math.min(held.menuShown.length, held.menuRoom) * held.rowHeight
                visible: held.mode === "command"
                clip: true
                interactive: false
                boundsBehavior: Flickable.StopAtBounds
                model: held.menuShown
                currentIndex: held.commandItem?.choosing.at ?? -1
                onCurrentIndexChanged: {
                    if (currentIndex >= 0)
                        positionViewAtIndex(currentIndex, ListView.Contain);
                    else
                        positionViewAtBeginning();
                }
                onContentYChanged: Qt.callLater(held.placeNumbers)
                onCountChanged: {
                    Qt.callLater(held.placeNumbers);
                    Qt.callLater(held.sayCommand);
                }

                delegate: MenuRow {
                    id: menuRow

                    required property int index
                    required property var modelData
                    readonly property string named: Commands.called(modelData)
                        + (modelData.after ?? "")
                    nameBaseline: menuChoice.nameBaseline
                    nameX: menuChoice.x + held.inset + menuChoice.glyphColumn
                    // The view lays its rows out after it opens, so where
                    // the line stands is said again once each is made.
                    Component.onCompleted: {
                        Qt.callLater(held.placeNumbers);
                        Qt.callLater(held.sayCommand);
                    }

                    width: menuRows.width
                    height: held.rowHeight

                    Choice {
                        id: menuChoice
                        x: held.columns.mark
                        width: parent.width - x
                        height: parent.height
                        groundFrom: x
                        selected: menuRow.index === menuRows.currentIndex
                        shown: held.visible
                        name: menuRow.named
                        plainName: menuRow.named
                        count: menuRow.modelData.title ?? ""
                        side: menuRow.modelData.key ?? ""
                    }
                }
            }

            Gutter {
                id: numbers
                z: 1
                height: parent.height
                clip: true
                slots: held.mode === "command" ? held.menuRoom + 1 : held.inView.length
                rows: held.numbered
                current: held.mode === "command" ? menuRows.currentIndex : held.cursor
                relative: held.relative
                digits: held.gutterDigits
                figures: String(Math.max(1, held.mode === "command" ? menuRows.count
                    : held.listed.length)).length
                shown: true
            }
        }

        // A line in view, its band: one row's, or taller under the
        // cursor, holding its card whole.
        Component {
            id: lineBand

            Line {
                id: noticeLine

                at: held.shownTop + index
                markX: mark.x
                nameX: named.x
                cardX: card.x

                required property var modelData
                required property int index
                readonly property var one: held.notice(modelData)
                readonly property bool atCursor: at === held.cursor
                readonly property bool chosenLine: held.selected.includes(modelData)
                    || atCursor && (held.mode === "normal" || held.pending)
                readonly property var from: one ? one.from : null
                // Sent at low urgency, it's quieter by two cues, as its
                // pop-up is (NoticeRow.qml).
                readonly property bool quiet: one !== null
                    && one.urgency === NotificationUrgency.Low
                readonly property bool washed: held.pattern !== ""
                // When it came, said at the far end of its only line, or of
                // its signing line under the cursor.
                readonly property real now: clock.date.getTime()
                readonly property string when: one
                    ? held.stack.since(held.stack.arrived[one.id] ?? now, now) : ""

                width: sheet.width
                height: card.visible ? held.rowHeight + card.height : held.rowHeight
                onYChanged: Qt.callLater(held.placeNumbers)

                // Chosen in visual, under the cursor in normal, or waiting
                // on an operator, its band wears the selection's ground,
                // as the launcher's tinted rows do.
                Ground {
                    from: held.inset
                    width: parent.width + 2 * held.inset
                    height: parent.height
                    ink: noticeLine.chosenLine ? "selection" : ""
                }

                // Its sender's mark, centred on its Name's capitals.
                Mark {
                    id: mark
                    x: held.columns.mark
                    line: held.rowBase
                    text: noticeLine.from ? noticeLine.from.glyph : ""
                    color: noticeLine.from && !noticeLine.quiet
                        ? Theme[noticeLine.from.hue] : Theme.muted
                }

                // Its Name, its capitals centred in the row, and its first
                // words after it. Washed by the find, they're rich text, set
                // on the same baseline.
                Row {
                    id: named
                    x: held.columns.text
                    y: held.rowBase - who.baselineOffset
                    spacing: Theme.space.rung.items

                    Text {
                        id: who
                        // Giving way a group before the time, or reaching the
                        // far end where its card signs it.
                        width: Math.min(implicitWidth, (time.visible
                            ? time.x - named.x - Theme.space.rung.groups
                            : noticeLine.width - named.x) * (first.text === "" ? 1 : 0.6))
                        text: !noticeLine.one ? "" : noticeLine.washed
                            ? Search.washed(noticeLine.one.summary, held.pattern,
                                held.wash(noticeLine.chosenLine))
                            : noticeLine.one.summary
                        textFormat: noticeLine.washed ? Text.RichText : Text.PlainText
                        elide: Text.ElideRight
                        maximumLineCount: 1
                        color: noticeLine.quiet ? Theme.text : Theme.strong
                        font.family: Theme.level.name.family
                        font.pointSize: Theme.level.name.pointSize
                        font.weight: Theme.level.name.weight
                    }

                    // Its first words, unless it's the cursor card, whose
                    // words show whole beneath.
                    Text {
                        id: first
                        // Giving way a group before the time.
                        width: Math.max(time.x - named.x - who.width - Theme.space.rung.items
                            - Theme.space.rung.groups, 0)
                        anchors.baseline: who.baseline
                        visible: text !== ""
                        text: !noticeLine.one || noticeLine.atCursor ? ""
                            : noticeLine.washed
                                ? Search.washed(noticeLine.one.words.plain
                                    .split("\n")[0], held.pattern,
                                    held.wash(noticeLine.chosenLine))
                            : noticeLine.one.words.plain.split("\n")[0]
                        textFormat: noticeLine.washed ? Text.RichText : Text.PlainText
                        elide: Text.ElideRight
                        maximumLineCount: 1
                        color: Theme.level.words.colour
                        font.family: Theme.level.words.family
                        font.pointSize: Theme.level.words.pointSize
                        font.weight: Theme.level.words.weight
                        font.variableAxes: Theme.level.words.variableAxes
                    }
                }

                // At the far end, on the inset, unless its card signs it.
                Text {
                    id: time
                    x: parent.width - implicitWidth
                    y: held.rowBase - baselineOffset
                    visible: !card.visible
                    text: noticeLine.when
                    color: Theme.level.note.colour
                    font: note.font
                }

                MouseArea {
                    width: parent.width
                    height: held.rowHeight
                    onClicked: {
                        if (noticeLine.atCursor)
                            held.open();
                        else
                            held.cursorId = noticeLine.modelData;
                    }
                }

                // The cursor card whole under its line, in its band, on the
                // text column, each line set by its baseline, down in beats
                // (VKCF3Z, 2026-11-05): what it says a pitch under the
                // Name, as lines of one saying; who sent it Note's pitch and
                // a beat on, as the pop-up signs it, when it came at the far
                // end; its acts as commands a pitch under that, and a
                // reply's field two beats under them.
                Item {
                    id: card

                    x: held.columns.text
                    y: held.rowHeight

                    // Its row's Name's baseline, down from its top, which
                    // is the row's bottom.
                    readonly property real nameBase: held.rowBase - held.rowHeight
                    readonly property real wordsBase: nameBase + Theme.level.words.pitch
                    readonly property real wordsEnd: body.visible
                        ? wordsBase + (body.lineCount - 1) * Theme.level.words.pitch
                        : nameBase
                    readonly property real senderBase: wordsEnd + Theme.level.note.pitch
                        + Theme.space.beat
                    readonly property real lastBase: says.visible
                        ? senderBase + Theme.level.note.pitch : senderBase
                    // How far the card runs on under its last line of words,
                    // the same however many it has.
                    readonly property real afterWords: Theme.level.note.pitch + Theme.space.beat
                        + (says.visible ? Theme.level.note.pitch : 0)
                        + (field.visible ? 2 * Theme.space.beat + held.rowHeight : held.rowLeaves)
                    // How many lines of words it has room for: six, or on a
                    // short screen those that leave its row and the rest of
                    // it whole inside the screen's rung (VKCF3Z, 2026-11-05).
                    readonly property int wordsFit: Math.max(1, Math.min(6, 1 + Math.floor(
                        (held.listTall - held.rowHeight - wordsBase - afterWords)
                        / Theme.level.words.pitch)))
                    // Its words as plain text, and where they're too many,
                    // the whole words that fit.
                    readonly property string plainWords: noticeLine.one && noticeLine.atCursor
                        ? noticeLine.one.words.plain : ""
                    readonly property var shortened: plainWords === "" ? null
                        : NoticeList.wholeWords(plainWords, wordsFit, width,
                            run => held.wordsWide(run))

                    // Folded while the command line's menu is up, which
                    // takes its room.
                    visible: noticeLine.atCursor && noticeLine.one !== null
                        && held.mode !== "command"
                    width: parent.width - x
                    // A field ends it as a row does. Else it leaves what a
                    // row leaves under its last baseline, so its band keeps
                    // as much room under its last line as over its first,
                    // and the next band meets it (Joel, 2026-11-06: rows
                    // tile like Vim's cursorline).
                    height: tall
                    readonly property real tall: field.visible ? field.y + field.height
                        : lastBase + held.rowLeaves
                    // Told to the list, which gives it room on a short
                    // screen; set rather than bound, as the rows shown
                    // follow from it.
                    function told() {
                        if (card.visible)
                            Qt.callLater(held.cardTold, noticeLine.modelData, card.tall);
                    }
                    onTallChanged: told()
                    onVisibleChanged: told()
                    Component.onCompleted: told()

                    Text {
                        id: body
                        y: card.wordsBase - (noticeLine.washed ? held.richBase : baselineOffset)
                        width: parent.width
                        visible: text !== ""
                        text: !noticeLine.one || !noticeLine.atCursor ? "" : noticeLine.washed
                            ? Search.washed(card.shortened ?? card.plainWords, held.pattern,
                                held.wash(noticeLine.chosenLine && held.mode === "visual"))
                            : card.shortened ?? noticeLine.one.words.styled
                        textFormat: noticeLine.washed ? Text.RichText
                            : card.shortened !== null ? Text.PlainText : Text.StyledText
                        // A link underlined in the words' own colour, as
                        // its pop-up draws it; gx opens it.
                        linkColor: color
                        onLinkActivated: href => {
                            Notices.mark(noticeLine.one.id, true);
                            held.stack.follow(href);
                            held.stack.holding = false;
                        }
                        wrapMode: Text.Wrap
                        // Qt's own cut stays behind the whole words', for a
                        // word too wide for a line alone.
                        maximumLineCount: card.wordsFit
                        elide: Text.ElideRight
                        lineHeight: Theme.level.words.pitch
                        lineHeightMode: Text.FixedHeight
                        color: Theme.text
                        font.family: Theme.level.words.family
                        font.pointSize: Theme.level.words.pointSize
                        font.weight: Theme.level.words.weight
                        font.variableAxes: Theme.level.words.variableAxes
                    }

                    // Who sent it, as the pop-up signs it, its mark already
                    // in the mark column, and when at the far end; its
                    // bottom is the line's baseline.
                    Signature {
                        id: sentBy
                        y: sentBy.onPixel(card.senderBase - sentBy.implicitHeight)
                        width: parent.width
                        name: noticeLine.from ? noticeLine.from.name : ""
                        side: noticeLine.when
                    }

                    // Its own acts as the commands that do them, ":reply" and
                    // ":play", in the text's colour since they're typed; Return
                    // goes to its sender, as the foot says.
                    Text {
                        id: says
                        y: card.senderBase + Theme.level.note.pitch - baselineOffset
                        width: parent.width
                        // Not while a reply is typed: :reply is already
                        // done, and a short screen needs its line.
                        visible: text !== "" && !field.visible
                        text: held.cardSays
                        textFormat: Text.PlainText
                        elide: Text.ElideRight
                        color: Theme.text
                        font: note.font
                    }

                    // A reply's words, typed on the card it answers, in a
                    // prompt's field: a row, its words on the card's column
                    // and ending at its inset, as every line of the card,
                    // and its ground a band's, raised, from the frame's
                    // inside edge to its other (VKCF3Z, 2026-11-05).
                    Item {
                        id: field
                        visible: noticeLine.atCursor && held.replyTo === noticeLine.modelData
                        y: card.lastBase + 2 * Theme.space.beat
                        width: parent.width
                        height: held.rowHeight

                        Ground {
                            from: card.x + held.inset
                            width: sheet.width + 2 * held.inset
                            height: parent.height
                            ink: "raised"
                        }

                        // How far in the caret stands.
                        TextMetrics {
                            id: replyBefore
                            font: figure.font
                            text: held.promptLine.text.slice(0, held.promptLine.caret)
                        }

                        // Over the caret, so the block in normal sits
                        // behind the letter it's on.
                        Text {
                            id: words
                            z: 1
                            width: parent.width
                            anchors.verticalCenter: parent.verticalCenter
                            text: held.promptLine.text === "" && held.mode === "insert"
                                ? noticeLine.one?.inlineReplyPlaceholder ?? ""
                                : held.promptLine.text
                            textFormat: Text.PlainText
                            elide: Text.ElideLeft
                            color: held.promptLine.text === "" ? Theme.muted : Theme.strong
                            font: figure.font
                        }

                        // The bar in insert, the block over a letter in
                        // normal, as Joel's guicursor.
                        Rectangle {
                            readonly property real before: replyBefore.advanceWidth
                            visible: held.mode === "insert" || held.mode === "prompt"
                            opacity: held.mode === "insert" ? held.caretLight : 1
                            x: words.x + before
                            y: words.y + words.baselineOffset - figure.ascent
                            width: held.mode === "insert"
                                ? Math.max(1, Math.round(figure.averageCharacterWidth / 4))
                                : figure.averageCharacterWidth
                            height: figure.ascent + figure.descent
                            color: held.mode === "insert" ? Theme.strong : Theme.selection
                        }
                    }
                }
            }
        }

        // Nothing left: said, on the text column, so the list never looks
        // broken.
        Text {
            visible: !held.listing && held.listed.length === 0
            x: held.columns.text
            height: held.rowHeight
            verticalAlignment: Text.AlignVCenter
            text: held.pattern !== "" ? `nothing holds ${held.pattern}` : "no notices"
            color: Theme.muted
            font.family: Theme.level.words.family
            font.pointSize: Theme.level.words.pointSize
            font.variableAxes: Theme.level.words.variableAxes
        }

        // What arrived while the lines held their order.
        Text {
            visible: held.arrivals > 0
            x: held.columns.text
            height: held.rowHeight
            verticalAlignment: Text.AlignVCenter
            text: `${held.arrivals} new`
            color: Theme.accent
            font: note.font
        }

        // Space ?: every key, by mode, each group under its heading, as the
        // drawer's key list sets them (VKCF3Z, 2026-11-05): a line a key
        // pitch, a heading a pitch and a beat over its first line and an
        // empty line under the one before; each chord apart from the next
        // and its keys a tie, and what they do a group past the keys.
        Item {
            id: keyLines

            // Each line's baseline, down from the list's top: the first's
            // on the first row's, a row under the head's, whatever its
            // level, so the key list keeps the rows' baselines.
            readonly property var bases: {
                const out = [];
                held.listShown.forEach((one, at) => {
                    out.push(at === 0 ? held.rowBase : out[at - 1]
                        + NoticeKeys.stepTo(held.listShown, at, held.listSteps));
                });
                return out;
            }

            visible: held.listing
            width: sheet.width
            height: bases.length === 0 ? 0 : bases[bases.length - 1] + held.rowLeaves

            Repeater {
                model: held.listing ? held.listShown : []

                Item {
                    id: listedKey

                    required property var modelData
                    required property int index
                    readonly property real base: keyLines.bases[index] ?? 0

                    width: sheet.width

                    Text {
                        y: listedKey.base - baselineOffset
                        visible: listedKey.modelData.heading !== ""
                        text: listedKey.modelData.heading
                        color: Theme.level.label.colour
                        font.family: Theme.level.label.family
                        font.pointSize: Theme.level.label.pointSize
                        font.weight: Theme.level.label.weight
                        font.capitalization: Theme.level.label.capitalization
                        font.letterSpacing: Theme.level.label.letterSpacing
                        font.features: Theme.level.label.features
                    }

                    Row {
                        y: listedKey.base - baselineOffset
                        spacing: Theme.space.rung.apart
                        visible: listedKey.modelData.heading === ""
                        baselineOffset: note.ascent

                        Repeater {
                            model: listedKey.modelData.chords

                            Row {
                                id: chord
                                required property string modelData
                                spacing: Theme.space.rung.tie

                                Repeater {
                                    model: chord.modelData.split(" ")

                                    Text {
                                        required property string modelData
                                        text: modelData
                                        textFormat: Text.PlainText
                                        color: Theme.text
                                        font: note.font
                                    }
                                }
                            }
                        }
                    }

                    Text {
                        x: held.doesAt
                        width: parent.width - x
                        y: listedKey.base - baselineOffset
                        visible: listedKey.modelData.heading === ""
                        text: listedKey.modelData.does
                        elide: Text.ElideRight
                        color: Theme.muted
                        font: note.font
                    }
                }
            }
        }

        // A tie between the last band and the foot's hairline, as under
        // the head's.
        Item {
            width: sheet.width
            height: Theme.space.rung.tie
        }

        // The foot a region of its own, as the launcher's is: under one
        // hairline, inset to inset as the head's.
        Rectangle {
            width: sheet.width
            height: Theme.border
            color: Theme.edge
        }

        // The foot: how things stand, the mode's word in its room, and the
        // hints, or what was just done until the next key. From the frame's
        // inside edge to its other, as the status line stands its own ink
        // at the card's inset.
        Item {
            id: foot

            x: -held.inset
            width: sheet.width + 2 * held.inset
            height: held.rowHeight

            // The mode word's room, kept for the widest it says often, so
            // the hints never shift as the mode changes; an operator's
            // word, naming its card, may take more where the hints leave
            // it. TextMetrics, since a binding follows a property and never
            // what a call measured before the face loaded.
            readonly property real wordRoom: widestWord.advanceWidth

            TextMetrics {
                id: widestWord
                font: note.font
                text: "999 chosen"
            }

            // The status line the launcher's foot is too (StatusLine.qml):
            // the mode in three capitals first (Joel, 2026-11-05: "in a
            // place where they are more obvious and not lost with other
            // things"), then which line of how many and the mode's word,
            // set here from where it says, then the hints at the right,
            // joined by dots, or what was just done until the next key.
            StatusLine {
                id: statusLine

                // Down to the frame, through the sheet's room under the
                // foot, so its text centres in the band the hairline and
                // the frame close (docs/design.md: text in a band centres
                // in it), not in the foot's row above that room.
                width: parent.width
                height: parent.height + sheet.bottomPadding
                inset: held.inset
                mark: held.modeMark
                resting: held.modeMark === NoticeKeys.mark("normal")
                hints: hinted.shown
                said: held.said
                saidHue: held.saidWrong ? Theme.alert : Theme.text
                saidRoom: hinted.fits
            }

            // Where the facts start, as the status line says.
            readonly property real factsAt: statusLine.factsAt

            // Whole, as the hints leave it its room as it says it now. It
            // gives way whole, never half, only while the hints that stay
            // leave it none, as normal's and visual's at 2 do: the cursor's
            // ground and number say where it stands, and in visual the count
            // chosen is what d acts on (VKCF3Z, 2026-11-06). It hides rather than narrowing
            // to nothing, which would lay it out again inside its own width.
            Text {
                id: facts
                x: foot.factsAt
                width: Math.min(implicitWidth, foot.width - foot.factsAt
                    - held.inset - Theme.space.rung.groups - hinted.wide)
                visible: !hinted.crowded
                anchors.baseline: parent.top
                anchors.baselineOffset: statusLine.base
                text: held.facts
                elide: Text.ElideRight
                color: Theme.muted
                font: note.font
            }

            // What the facts take of the line, a group after them, or
            // nothing while they're hidden or empty.
            readonly property real factsTake: facts.visible && facts.width > 0
                ? facts.width + Theme.space.rung.groups : 0

            Text {
                id: modeWord
                x: foot.factsAt + foot.factsTake
                width: Math.max(foot.wordRoom, Math.min(implicitWidth, foot.width
                    - foot.factsAt - held.inset - foot.factsTake
                    - hinted.wide - Theme.space.rung.groups))
                anchors.baseline: parent.top
                anchors.baselineOffset: statusLine.base
                text: held.modeWord
                elide: Text.ElideRight
                color: held.standing.count !== "" && !held.pending ? Theme.strong : Theme.accent
                font: note.font
            }

            // The hints the status line draws, as many as fit.
            Item {
                id: hinted

                // The room the facts and the mode word leave as they say
                // it now, a group from each, so at rest, with no mode word,
                // the hints have it all, and none gives way while there's
                // room for it (VKCF3Z, 2026-11-05).
                readonly property real fits: foot.width - foot.factsAt
                    - held.inset - factsNow.advanceWidth - Theme.space.rung.groups
                    - (held.modeWord === "" ? 0
                        : wordNow.advanceWidth + Theme.space.rung.groups)

                TextMetrics {
                    id: factsNow
                    font: note.font
                    text: held.facts
                }

                TextMetrics {
                    id: wordNow
                    font: note.font
                    text: held.modeWord
                }

                // The hints that fit: the one furthest from Return gives
                // way first, and the keys list, done and Return never do
                // (NoticeList.js). The dots' gap is read, so they're
                // measured again once the theme's face arrives.
                readonly property var shown: {
                    const keys = statusLine.keys;
                    const join = keys.spacing;
                    const wide = hint => keys.advance(hint.key, hint.word);
                    return NoticeList.footHints(held.hints, wide, join, fits)
                        .map(one => [one.key, one.word]);
                }

                // How wide the hints that stay are, by the same advances
                // `fits` was measured against, so the facts and the mode
                // word leave them their room before the keys have laid out
                // (Y2YBTS, 2026-11-06).
                readonly property real wide: shown.reduce((sum, hint, at) => sum
                    + (at > 0 ? statusLine.keys.spacing : 0)
                    + statusLine.keys.advance(hint[0], hint[1]), 0)

                // Whether the hints that stay leave the facts no room, as
                // drawn: what was just done, said in their place, takes
                // only `fits`.
                readonly property bool crowded: held.said === "" && wide > fits
            }
        }
    }

    // How many of the menu's lines show in the lines' place: ten at most,
    // as his pumheight, and no more than the lines' room; one at least.
    readonly property int menuRoom: Math.max(1, Math.min(10, Math.floor(listTall / rowHeight)))
    // How tall the lines may be: the screen less its rung each side, the
    // frame's hairlines, the head and its rule, the ties inside the rules,
    // and the foot and its rule.
    readonly property real listTall: screenHeight - 2 * Theme.space.rung.screen
        - 4 * Theme.border - headTall - 2 * Theme.space.rung.tie - rowHeight - sheet.bottomPadding

    SystemClock {
        id: clock
        precision: SystemClock.Minutes
    }
}
