// What a notice's body says, read once as it arrives (Notices.qml): as plain
// text, for the launcher's rows, the find and a copy; as styled text a card
// and a drawer draw, carrying only the markup the notification spec allows
// (bold, italics, underline and links) with everything else escaped, so a
// sender can't draw what the desktop doesn't; and its links, each its words
// and where it goes, for the keys to open. A bare address in the words is a
// link too, since chat programs send their links that way. Apart from the
// shell, so tests/notice-body.qml holds it in Qt's own engine.
.pragma library

// The spec's tags a body may carry, and Qt's line break, which the styled
// text draws; any other `<` is the sender's own words.
const TAG = /<\s*(\/?)\s*(b|i|u|a|br|img)\b([^>]*)>/giu;
// Where a link may go: the web, or mail. Anything else, such as a script's
// `javascript:` or a file, is said as words and never opened.
const GOES = /^(https?:\/\/|mailto:)/iu;
// An address in the words themselves, and the punctuation that closes a
// sentence round it, which is taken off its end.
const BARE = /\bhttps?:\/\/[^\s<>"']+/giu;
const CLOSING = /[.,;:!?)\]]+$/u;
const ENTITY = /&(#x[0-9a-f]+|#[0-9]+|[a-z]+);/giu;
const MAIL = /^mailto:([^?]*)/iu;
const HOST = /^[a-z]+:\/\/([^/?#:]+)/iu;
const WWW = /^www\./iu;
const NEWLINE = /\n/gu;
const AMPERSAND = /&/gu;
const LESS = /</gu;
const GREATER = />/gu;
const QUOTE = /"/gu;

// The entities a sender may write, and any by its number up to Unicode's
// last.
const NAMED = { amp: "&", lt: "<", gt: ">", quot: '"', apos: "'", nbsp: "\u00a0" };
const LAST_CODE_POINT = 0x10_ff_ff;
const HEX = 16;
const DECIMAL = 10;

// Every match of a global pattern in a text, as matchAll would give them,
// which Qt's engine doesn't have. The pattern is global, so it's started
// from the text's beginning each time.
function matches(pattern, text) {
    const found = [];
    pattern.lastIndex = 0;
    for (let one = pattern.exec(text); one !== null; one = pattern.exec(text)) {
        found.push(one);
    }
    pattern.lastIndex = 0;
    return found;
}

function decoded(text) {
    return text.replace(ENTITY, (whole, name) => {
        if (name[0] !== "#") {
            return NAMED[name.toLowerCase()] ?? whole;
        }
        const hex = name[1] === "x" || name[1] === "X";
        const code = hex
            ? Number.parseInt(name.slice(2), HEX)
            : Number.parseInt(name.slice(1), DECIMAL);
        return code > 0 && code <= LAST_CODE_POINT ? String.fromCodePoint(code) : whole;
    });
}

function escaped(text) {
    return text
        .replace(AMPERSAND, "&amp;")
        .replace(LESS, "&lt;")
        .replace(GREATER, "&gt;")
        .replace(QUOTE, "&quot;");
}

// An attribute's value, quoted either way or bare.
function attribute(attributes, name) {
    const found = new RegExp(
        `\\b${name}\\s*=\\s*("([^"]*)"|'([^']*)'|([^\\s>]+))`,
        "iu",
    ).exec(attributes);
    return found ? decoded(found[2] ?? found[3] ?? found[4] ?? "") : "";
}

// Plain words as styled text, each bare address in them a link, which
// `links` gains.
function linked(text, links) {
    let out = "";
    let from = 0;
    for (const found of matches(BARE, text)) {
        const href = found[0].replace(CLOSING, "");
        out += escaped(text.slice(from, found.index));
        links.push({ text: href, href });
        out += `<a href="${escaped(href)}">${escaped(href)}</a>`;
        from = found.index + href.length;
    }
    return out + escaped(text.slice(from));
}

// What's read so far: the plain words, the styled text, the links, the tags
// open, and the link being read, its words gathered until it closes.
function reading() {
    return { plain: "", styled: "", links: [], open: [], link: null };
}

function wordsOf(state, text) {
    const said = decoded(text);
    state.plain += said;
    if (state.link) {
        state.link.text += said;
        state.styled += escaped(said);
    } else {
        state.styled += linked(said, state.links);
    }
}

// A link ends, kept where it goes somewhere it may.
function linkEnds(state) {
    const { link } = state;
    if (link.href !== "") {
        state.styled += "</a>";
        state.links.push({ text: link.text.trim() || link.href, href: link.href });
    }
    state.link = null;
}

function anchor(state, closing, attributes) {
    if (!closing && state.link === null) {
        const href = attribute(attributes, "href");
        state.link = { text: "", href: GOES.test(href) ? href : "" };
        if (state.link.href !== "") {
            state.styled += `<a href="${escaped(state.link.href)}">`;
        }
    } else if (closing && state.link !== null) {
        linkEnds(state);
    }
}

// Closed in order, as styled text needs, those opened inside it closing
// with it; one closed that never opened is dropped.
function closes(state, tag) {
    if (!state.open.includes(tag)) {
        return;
    }
    let last = "";
    while (last !== tag) {
        last = state.open.pop();
        state.styled += `</${last}>`;
    }
}

function tagged(state, found) {
    const closing = found[1] === "/";
    const tag = found[2].toLowerCase();
    if (tag === "br") {
        state.plain += "\n";
        state.styled += "<br>";
    } else if (tag === "img") {
        // A picture in the words is said by what it shows, never fetched,
        // since the desktop draws nothing it wasn't handed.
        wordsOf(state, attribute(found[3], "alt"));
    } else if (tag === "a") {
        anchor(state, closing, found[3]);
    } else if (closing) {
        closes(state, tag);
    } else {
        state.open.push(tag);
        state.styled += `<${tag}>`;
    }
}

// The body read: { plain, styled, links }, each link { text, href }, in the
// order they come. A tag left open is closed at the end, so the styled text
// is always whole.
function read(body) {
    const source = body ?? "";
    const state = reading();
    let from = 0;
    for (const found of matches(TAG, source)) {
        wordsOf(state, source.slice(from, found.index));
        from = found.index + found[0].length;
        tagged(state, found);
    }
    wordsOf(state, source.slice(from));
    if (state.link !== null) {
        linkEnds(state);
    }
    while (state.open.length > 0) {
        state.styled += `</${state.open.pop()}>`;
    }
    // Plain text keeps the sender's own line breaks; styled text says them
    // as breaks, which it draws.
    return {
        plain: state.plain,
        styled: state.styled.replace(NEWLINE, "<br>"),
        links: state.links,
    };
}

// Where a link goes, said short: its host, without a leading www, or the
// address of a mail.
function host(href) {
    const mail = MAIL.exec(href);
    if (mail) {
        return decodeURIComponent(mail[1]);
    }
    const found = HOST.exec(href);
    return found ? found[1].replace(WWW, "").toLowerCase() : href;
}
