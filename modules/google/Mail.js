// What the shell says of the mail that wants Joel, as the google program
// keeps it (src/mail.rs): when each came, what its row and its details
// say, where a reply to it is written, the lines that mark one read or
// archive it (src/ask.rs), and which windows are Mail's. Mail.qml asks
// these, and the checks run them in Qt's own engine (tests/mail.qml).
// Times are milliseconds since 1970, as the program gives them.
.pragma library
.import "Calendar.js" as Calendar

// In milliseconds.
const DAY = 86_400_000;
// Within how many days mail is said by its day of the week: a week.
const WEEK = 7;

const DAYS = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
const MONTHS = ["January", "February", "March", "April", "May", "June", "July", "August",
    "September", "October", "November", "December"];

// When mail came, as its row says it: its time today, "yesterday", its
// day within the week, else its date, "12 October".
function came(at, now) {
    const days = Math.round((Calendar.midnight(now) - Calendar.midnight(at)) / DAY);
    const date = new Date(at);
    if (days <= 0) {
        return Calendar.clock(date);
    }
    if (days === 1) {
        return "yesterday";
    }
    if (days < WEEK) {
        return DAYS[date.getDay()];
    }
    return `${date.getDate()} ${MONTHS[date.getMonth()]}`;
}

// What its row says beside it: who it's from and when it came, "Ada
// Lovelace · 14:02", or only when it came where the sender gives no name.
function beside(mail, now) {
    const when = came(mail.at, now);
    return mail.from === "" ? when : `${mail.from} · ${when}`;
}

// When it came, in full, as its details say it: "Monday 26 October, 14:02".
function dated(at) {
    const date = new Date(at);
    return `${DAYS[date.getDay()]} ${date.getDate()} ${MONTHS[date.getMonth()]}, `
        + Calendar.clock(date);
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
function details(mail, untitled) {
    const attachments = mail.attachments ?? [];
    const facts = [
        { label: "From", value: sender(mail) },
        { label: "To", value: mail.to ?? "" },
        // Copied to, as every mail program shows it, and only when someone was.
        { label: "Cc", value: mail.cc ?? "" },
        { label: "Date", value: dated(mail.at) },
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

// Where a new message to these addresses is written, with this subject,
// as Gmail's own link to compose takes them: each address once, joined
// as Gmail joins them, the subject left out when there's none to give.
// Nothing when there's nobody to write to.
function writing(to, subject) {
    const addresses = [...new Set(to.filter(address => (address ?? "") !== ""))];
    if (addresses.length === 0) {
        return "";
    }
    const asked = [["view", "cm"], ["fs", "1"], ["to", addresses.join(",")]]
        .concat((subject ?? "") === "" ? [] : [["su", subject]]);
    return `${COMPOSE}?${asked.map(([name, value]) => `${name}=${encodeURIComponent(value)}`)
        .join("&")}`;
}

// Where a reply to it is written: a new message in Mail to who sent it,
// its subject marked a reply once. Nothing when it gives no address to
// reply to.
function replying(mail) {
    const subject = REPLIED.test(mail.subject) ? mail.subject : `Re: ${mail.subject}`;
    return writing([mail.address], subject);
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
// or save as `how` says.
function fetch(mail, at, how) {
    return `${JSON.stringify({ fetch: { id: mail.id, file: at, how } })}\n`;
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
