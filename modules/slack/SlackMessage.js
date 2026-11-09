// A Slack message's one row, wherever it's listed (SlackMessage.qml):
// For You's Slack row, its History, and what Search Slack finds (VKCF3Z,
// 2026-11-08: "a Slack message is one thing, so it gets one row
// everywhere"). Kept apart from the QML so Qt's own engine holds it
// (tests/slack-message.qml). `one` is a message as Slack's daemon says
// it: one it holds wanting Joel, { channel, ts, place, who, mine, first,
// text, at, link, team } (src/messages.rs), or one a search found, which
// says its conversation and timestamp in its key instead,
// "slack-message:<channel>:<ts>" (src/search.rs).
.pragma library

const SPACES = /\s+/g;
const LINES = /\r?\n/;

// Its conversation and timestamp, as Slack names them.
function channelOf(one) {
    return String(one.channel ?? String(one.key ?? "").split(":")[1] ?? "");
}

function tsOf(one) {
    return String(one.ts ?? String(one.key ?? "").split(":")[2] ?? "");
}

// Its key, the same wherever it's listed, so it's one thing.
function key(one) {
    return `slack:message:${channelOf(one)}:${tsOf(one)}`;
}

// Whether it's in a direct message, whose conversation Slack names with a
// D.
function direct(one) {
    return channelOf(one).startsWith("D");
}

// Who said it, by which its row is named, since a message has no title:
// Joel's own as `you`.
function who(one, you) {
    return one.mine === true ? you : String(one.who ?? "");
}

// Its note: its first line; or, for one found by `words` matching only
// past it, the message from the first matched word on, so the match still
// shows (VKCF3Z, 2026-11-08).
function note(one, words) {
    const text = String(one.text ?? "");
    const first = String(one.first ?? text.split(LINES)[0]);
    const asked = String(words ?? "").toLowerCase().split(SPACES).filter(word => word !== "");
    const lowered = first.toLowerCase();
    if (asked.length === 0 || asked.some(word => lowered.includes(word))) {
        return first;
    }
    const flat = text.replace(SPACES, " ").trim();
    const found = asked.map(word => flat.toLowerCase().indexOf(word))
        .filter(index => index >= 0);
    if (found.length === 0) {
        return first;
    }
    const at = Math.min(...found);
    return flat.slice(flat.lastIndexOf(" ", at) + 1);
}

// Its drawer's eyebrow, where it is: "in #deploys · message", or "direct
// message" (`said`, words.json's slack:message).
function eyebrow(one, said) {
    return direct(one) ? String(said.direct ?? "")
        : String(said.where ?? "").replace("{place}", String(one.place ?? ""));
}

// Where Return takes Joel: the message itself, where Slack shows it, else
// its conversation in Slack's app.
function opens(one) {
    return one.link ? String(one.link)
        : `slack://channel?team=${one.team ?? ""}&id=${channelOf(one)}`;
}
