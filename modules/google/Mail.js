// What the shell says of the mail that wants Joel, as the google program
// keeps it (src/mail.rs): when each came, what its row and its details
// say, where a reply to it, or to all, is written, the lines that mark one
// read or archive it (src/ask.rs), and which windows are Mail's. Mail.qml asks
// these, and the checks run them in Qt's own engine (tests/mail.qml).
// Times are milliseconds since 1970, as the program gives them.
.pragma library
.import "Calendar.js" as Calendar
.import "When.js" as When

// In milliseconds.
const MINUTE = 60_000;
const HOUR = 3_600_000;

function sameYear(at, now) {
    return new Date(at).getFullYear() === new Date(now).getFullYear();
}

// When mail came, as its row says it, in the launcher's one way to say
// when (Y2YBTS's): "just now" within a minute, "12 min ago" within the
// hour, its time earlier today, and before today its day as When.js says
// any: "yesterday", its weekday within the last six days, else "28 Oct",
// with the year only when it isn't this one. A moment ahead of now, from a
// wrong clock, reads as one behind does: its time today, then "tomorrow",
// its weekday within the next six days, else its date. The words are
// `said`'s, the vocabulary's, and the names the locale's, as `said.date`
// gives them, Qt's `formatDate` in the shell.
function came(at, now, said) {
    const since = now - at;
    if (since >= 0 && since < MINUTE) {
        return said.now;
    }
    if (since >= 0 && since < HOUR) {
        return String(said.minutes).replace("{minutes}", Math.floor(since / MINUTE));
    }
    const day = When.day(at, now, said);
    return day === "" ? Calendar.clock(new Date(at)) : day;
}

// When something was, as a drawer's fact says it beside a row that says
// when in its own side: its time today, "06:06", and before today its day
// as When.js says any, then its time, "yesterday 14:10", "Wednesday
// 09:30", "28 Oct 09:30", as a call's note says when it was
// (Transcripts.js). It's never "12 min ago", which the row's side says,
// so a drawer's moment never reads as the side said twice. Nor is it the
// long date `dated` gives, so a Drive file's drawer says when as the
// launcher does everywhere else (VKCF3Z, 2026-11-08).
function moment(at, now, said) {
    const day = When.day(at, now, said);
    const time = Calendar.clock(new Date(at));
    return day === "" ? time : `${day} ${time}`;
}

// When a meeting that's over was, as its row says it at its right in a
// Recent: when it started as `moment` says any moment, "15:00" today,
// "yesterday 15:00", "28 Oct 09:30"; a whole day's is its day alone, or
// "today" (VKCF3Z, ccccbe5f). It's here, beside `moment`, since Calendar.js
// says only what's to come.
function was(meeting, now, said) {
    if (!meeting.allDay) {
        return moment(meeting.start, now, said);
    }
    const day = When.day(meeting.start, now, said);
    return day === "" ? "today" : day;
}

// What a meeting's row notes after its name: its times and his answer
// while it's to come or on, "14:00 to 14:30, maybe", and nothing once
// it's over, where `was` alone says when it was at its right and its
// drawer keeps When and Your Answer (VKCF3Z, c7d9e2bd).
function noted(meeting, now) {
    return meeting.end <= now ? "" : Calendar.about(meeting);
}

// What its row says beside it: who it's from and when it came, "Ada
// Lovelace · 14:02", or only when it came where the sender gives no name.
// Mail Joel sent names who it went to, "to Ada Lovelace · 13:17", never
// him (VKCF3Z, c7d9e2bd).
function beside(mail, now, said) {
    const when = came(mail.at, now, said);
    const who = mail.sent === true && (mail.to ?? "") !== ""
        ? String(said.to).replace("{who}", mail.to)
        : mail.from;
    return who === "" ? when : `${who} · ${when}`;
}

// When it came, in full, as its details say it and a meeting's say when
// it is: "Monday 26 October, 14:02", with the year only when it isn't
// this one (VKCF3Z, c7d9e2bd: one form for both).
function dated(at, now, said) {
    const date = new Date(at);
    const day = said.date(date, sameYear(at, now) ? "dddd d MMMM" : "dddd d MMMM yyyy");
    return `${day}, ${Calendar.clock(date)}`;
}

// Who sent it, as its details say it: by name and address, as mail writes
// them, or whichever of the two it gives.
function sender(mail) {
    if (mail.address === "" || mail.address === mail.from) {
        return mail.from;
    }
    return mail.from === "" ? mail.address : `${mail.from} <${mail.address}>`;
}

// Its details, as the launcher's drawer shows them beside its row: its
// subject, who it's from and to, when it came, the files it carries by
// name, and what it says, every one plain text, since they're someone
// else's words.
function details(mail, untitled, now, said) {
    const attachments = mail.attachments ?? [];
    const facts = [
        { label: "From", value: sender(mail) },
        { label: "To", value: mail.to ?? "" },
        // Copied to, as every mail program shows it, and only when someone was.
        { label: "Cc", value: mail.cc ?? "" },
        { label: "Date", value: dated(mail.at, now, said) },
        {
            label: attachments.length === 1 ? "Attachment" : "Attachments",
            value: attachments.map(file => file.name).join(", ")
        }
    ];
    return {
        title: mail.subject === "" ? untitled : mail.subject,
        facts: facts.filter(fact => fact.value !== ""),
        body: mail.body ?? ""
    };
}

// A reply is said to be one by its subject, as mail clients write it.
const REPLIED = /^re:/i;
// Gmail, where a new message is written in a window of its own.
const COMPOSE = "https://mail.google.com/mail/u/0/";

// Where a new message to these addresses is written, with this subject
// and copied to those, as Gmail's own link to compose takes them: each
// address once, joined as Gmail joins them, the subject left out when
// there's none to give. Nothing when there's nobody to write to.
function writing(to, subject, copied) {
    const once = all => [...new Set(all.filter(address => (address ?? "") !== ""))];
    const addresses = once(to);
    if (addresses.length === 0) {
        return "";
    }
    const asked = [["view", "cm"], ["fs", "1"], ["to", addresses.join(",")]];
    const cc = once(copied ?? []).filter(address => !addresses.includes(address));
    if (cc.length > 0) {
        asked.push(["cc", cc.join(",")]);
    }
    if ((subject ?? "") !== "") {
        asked.push(["su", subject]);
    }
    return `${COMPOSE}?${asked.map(([name, value]) => `${name}=${encodeURIComponent(value)}`)
        .join("&")}`;
}

// Where a reply to it is written: a new message in Mail to who sent it,
// its subject marked a reply once. Nothing when it gives no address to
// reply to.
function replying(mail) {
    return writing([mail.address], answering(mail));
}

// Where a reply to all is written: to who sent it, copied to everyone
// else it went to or was copied to, but Joel (src/mail.rs). Nothing when
// it went to nobody else, since then it's only a reply.
function replyingAll(mail) {
    const others = mail.others ?? [];
    return others.length === 0 ? "" : writing([mail.address], answering(mail), others);
}

// A reply's subject, marked a reply once.
function answering(mail) {
    return REPLIED.test(mail.subject) ? mail.subject : `Re: ${mail.subject}`;
}

// Where a mail to everyone else a meeting names is written, its holder
// and each guest with an address, about the meeting by its title. Joel
// isn't among them (src/calendar.rs leaves him out), and nor is a room.
function toTheGuests(meeting) {
    const guests = (meeting.guests ?? []).map(guest => guest.email);
    return writing([meeting.organizer?.email].concat(guests), meeting.title ?? "");
}

// The line that asks the program to mark mail read or unread again, or to
// archive it.
function tidy(mail, how) {
    return `${JSON.stringify({ [how]: { id: mail.id } })}\n`;
}

// What's said of mail once it's been marked read or archived, and the
// program no longer holds it as wanting Joel, or marked unread again and
// held once more.
const TIDIED = { read: "read", unread: "unread", archive: "archived" };

function tidied(how) {
    return TIDIED[how] ?? "";
}

// What a file's row says while the program fetches it and once it's done,
// or why it couldn't, as a network's says it's connecting.
const ELLIPSIS = "\u2026";
const FETCHED = {
    fetching: `fetching${ELLIPSIS}`,
    saved: "saved in Downloads",
    savedThere: "saved",
    gone: "no longer in the mail",
    runs: "a program or a script, so only saved",
    unreachable: "Gmail couldn't be reached",
    unwritten: "couldn't be written down"
};

function fetchedSaid(stood) {
    return FETCHED[stood] ?? "";
}

// How the file `at` on `mail` stands, by what the program last said of
// the file fetched (src/files.rs), for an ask made at or after `since`:
// "fetching", "opened", "saved" or why not, and nothing for one not
// asked for since.
function fetchedAs(fetched, mail, at, since) {
    if (!fetched || fetched.mail !== mail.id || fetched.file !== at || fetched.asked < since) {
        return "";
    }
    if (fetched.failed) {
        return fetched.failed;
    }
    if (!fetched.path) {
        return "fetching";
    }
    return fetched.how === "save" ? "saved" : "opened";
}

// The line asking the program to fetch the file `at` on `mail`, to open
// or save as `how` says, a save going `to` the path Joel picked when he
// picked one.
function fetch(mail, at, how, to = "") {
    const asked = { id: mail.id, file: at, how };
    if (to !== "") {
        asked.to = to;
    }
    return `${JSON.stringify({ fetch: asked })}\n`;
}

// A file's kind as a file manager says it, by its ending ("PDF"), or
// `plain` for one with none.
function kind(file, plain) {
    const at = file.name.lastIndexOf(".");
    return at > 0 && at < file.name.length - 1 ? file.name.slice(at + 1).toUpperCase() : plain;
}

// Sizes in decimal units, as a picture's and a disk's are said elsewhere
// in the launcher (Copies.qml's weighed), with a figure after the point
// only below ten.
const GIGA = 1e9;
const MEGA = 1e6;
const KILO = 1e3;
const UNITS = [
    [GIGA, "GB"],
    [MEGA, "MB"],
    [KILO, "kB"]
];
const TENS = 10;

function weighed(bytes) {
    const unit = UNITS.find(each => bytes >= each[0]);
    if (!unit) {
        return `${Math.round(bytes)} B`;
    }
    const scaled = bytes / unit[0];
    return `${scaled < TENS ? scaled.toFixed(1) : Math.round(scaled)} ${unit[1]}`;
}

// What a file's row says beside it: its kind and size.
function sized(file, plain) {
    return `${kind(file, plain)} \u00b7 ${weighed(file.size ?? 0)}`;
}

// A file's details, as the drawer shows them beside its row.
function carried(file, plain) {
    return {
        title: file.name,
        facts: [
            { label: "Kind", value: kind(file, plain) },
            { label: "Size", value: weighed(file.size ?? 0) }
        ],
        body: ""
    };
}

// Chrome names a Google web app's window from the link it opened:
// "chrome-mail.google.com_", then its path, each slash an underscore, so
// Mail's own page and each piece of mail opened from the shell are both
// Mail's. Mail in a browser tab is the browser's, and no window is.
const MAIL_WINDOW = /^chrome-mail\.google\.com_/;

// The ids of the windows, as niri lists them, that are Mail's.
function windows(all) {
    return all.filter(window => MAIL_WINDOW.test(window.app_id ?? "")).map(window => window.id);
}

// Chrome opens each link in a window of its own and can't be told to take
// an open one to it, so mail opened while Mail is open shows in a new one,
// and those open before close once it's there, leaving one. Given the Mail
// windows open before and those open now: the ones to close, still open,
// once a new one is among them; none until then.
function superseded(before, now) {
    return now.some(id => !before.includes(id)) ? before.filter(id => now.includes(id)) : [];
}
