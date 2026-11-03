// What the shell works out of each call call-transcripts wrote down, apart
// from drawing it: its name, what it was, when and how long, the drawer's
// words, and which calls are with someone or of a meeting. Plain
// JavaScript, held by tests/transcripts.qml in Qt's own engine.
.pragma library

const MINUTE = 60_000;
const HOUR_MINUTES = 60;
const DAY = 86_400_000;
const WEEK_DAYS = 7;
const DIGITS = 2;
const DAYS = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
const MONTHS = ["January", "February", "March", "April", "May", "June", "July", "August",
    "September", "October", "November", "December"];

// Markdown's marks, as a summary or a transcript is written with them.
const RETURNS = /\r/g;
const BETWEEN_PARAGRAPHS = /\n\s*\n/;
const AMPERSANDS = /&/g;
const OPENS = /</g;
const CLOSES = />/g;
const BOLD = /\*\*(.+?)\*\*/g;
const STARRED = /(^|[^\w*])\*(?!\s)(.+?)\*(?!\w)/g;
const UNDERSCORED = /(^|\W)_(?!\s)(.+?)_(?!\w)/g;
const CODE = /`([^`]+)`/g;
const HEADING = /^#{1,6}\s+(.*)$/;
const ITEM = /^(?:[-*+]|\d+[.)])\s+(.*)$/;
const QUOTE = /^>\s?(.*)$/;

// The calls the daemon holds, newest first, any record not a call's left
// out.
function newestFirst(records) {
    return records.filter(record => record && record.kind === "call"
        && typeof record.started === "number")
        .sort((a, b) => b.started - a.started);
}

// What it was, as its side says it in a word or two: a meeting's call, a
// call with someone, or a call that was neither.
function kind(record) {
    if (record.event) {
        return "meeting";
    }
    return record.person ? "person" : "call";
}

function clock(time) {
    const date = new Date(time);
    const two = n => String(n).padStart(DIGITS, "0");
    return `${two(date.getHours())}:${two(date.getMinutes())}`;
}

function midnight(time) {
    const date = new Date(time);
    date.setHours(0, 0, 0, 0);
    return date.getTime();
}

// The day it was on, as its note says it before the time: "today",
// "yesterday", the day of the week within the week, else the date, with
// the year only when it isn't this one.
function day(time, now) {
    const days = Math.round((midnight(now) - midnight(time)) / DAY);
    const date = new Date(time);
    if (days <= 0) {
        return "today";
    }
    if (days === 1) {
        return "yesterday";
    }
    if (days < WEEK_DAYS) {
        return DAYS[date.getDay()];
    }
    const year = date.getFullYear() === new Date(now).getFullYear() ? ""
        : ` ${date.getFullYear()}`;
    return `${date.getDate()} ${MONTHS[date.getMonth()]}${year}`;
}

// How long it went on, from its start to its end: "under a minute",
// "32 min", "1 h", "1 h 5 min".
function length(started, ended) {
    const minutes = Math.floor(Math.max(0, ended - started) / MINUTE);
    if (minutes < 1) {
        return "under a minute";
    }
    if (minutes < HOUR_MINUTES) {
        return `${minutes} min`;
    }
    const hours = `${Math.floor(minutes / HOUR_MINUTES)} h`;
    return minutes % HOUR_MINUTES === 0 ? hours : `${hours} ${minutes % HOUR_MINUTES} min`;
}

// Its note: when it was and how long, "yesterday 14:00 \u00b7 32 min",
// or, while it's on, what `said` says of it: being written down, or on
// still with nothing being written, "today 14:00 \u00b7 not written down".
function note(record, now, said) {
    if (record.transcribing) {
        return said.writing;
    }
    const when = `${day(record.started, now)} ${clock(record.started)}`;
    if (typeof record.ended !== "number") {
        return `${when} \u00b7 ${said.on}`;
    }
    return `${when} \u00b7 ${length(record.started, record.ended)}`;
}

// The day and time it began, as the drawer says them over its title, as
// a notice's are: "Tuesday 3 November \u00b7 14:00". How long it went on
// is a fact of its own, so its end isn't said here too.
function dated(record) {
    const date = new Date(record.started);
    return `${DAYS[date.getDay()]} ${date.getDate()} ${MONTHS[date.getMonth()]} \u00b7 `
        + clock(record.started);
}

// Text made safe to show as styled text, so nothing a call said is read
// as markup.
function escaped(text) {
    return text.replace(AMPERSANDS, "&amp;").replace(OPENS, "&lt;").replace(CLOSES, "&gt;");
}

// A line of markdown's own marks, bold and italics, as styled text, and
// code as its plain words.
function inline(text) {
    return escaped(text)
        .replace(BOLD, "<b>$1</b>")
        .replace(STARRED, "$1<i>$2</i>")
        .replace(UNDERSCORED, "$1<i>$2</i>")
        .replace(CODE, "$1");
}

// Markdown, as a summary or a transcript is written, as the drawer's
// styled text: a heading in bold, a list's items each a line led by a
// bullet, a quote in italics, and a blank line between paragraphs, so it
// reads as the reader would set it, on the drawer's one pitch. A plain
// line right after another goes on the same paragraph, as markdown reads
// a line wrapped by hand.
function styled(markdown) {
    return String(markdown ?? "").replace(RETURNS, "").split(BETWEEN_PARAGRAPHS)
        .map(paragraph => paragraph.split("\n").map(line => line.trim())
            .filter(line => line !== ""))
        .filter(lines => lines.length > 0)
        .map(styledParagraph)
        .join("<br><br>");
}

// A paragraph's lines as styled text, a line apiece but for plain lines
// in a row, which run on as one.
function styledParagraph(lines) {
    const out = [];
    let plain = false;
    for (const line of lines) {
        const mark = marked(line);
        if (mark === null && plain) {
            out[out.length - 1] += ` ${inline(line)}`;
        } else {
            out.push(mark ?? inline(line));
        }
        plain = mark === null;
    }
    return out.join("<br>");
}

// A line markdown marks as a heading, a list's item or a quote, styled,
// or null for a plain one.
function marked(line) {
    const heading = HEADING.exec(line);
    if (heading) {
        return `<b>${inline(heading[1])}</b>`;
    }
    const item = ITEM.exec(line);
    if (item) {
        return `\u2022 ${inline(item[1])}`;
    }
    const quote = QUOTE.exec(line);
    return quote ? `<i>${inline(quote[1])}</i>` : null;
}

// Its drawer: when it was over its title, how long it went on and who it
// was with, then its summary when one's written, else how its transcript
// opens, both set as the markdown they are, in the plate's sans as a
// notice's words are. Who it was with is `who`, their name as People
// knows them, or else their address.
function details(record, name, said, who) {
    const facts = [];
    if (typeof record.ended === "number") {
        facts.push({ label: said.length, value: length(record.started, record.ended) });
    }
    if (record.person) {
        facts.push({ label: said.with, value: who || record.person });
    }
    const words = record.summary || record.opening || "";
    return {
        eyebrow: dated(record),
        title: name,
        facts,
        body: styled(words),
        styled: true,
        sans: true
    };
}

// Whether a call was with someone, by any of their addresses.
function withThem(record, emails) {
    const person = String(record.person ?? "").toLowerCase();
    return person !== "" && emails.some(email => String(email).toLowerCase() === person);
}

// The newest call of a meeting, by its calendar event, or null.
function ofEvent(records, event) {
    return records.find(record => event && record.event === event) ?? null;
}
