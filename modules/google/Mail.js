// What the shell says of the mail that wants Joel, as the google program
// keeps it (src/mail.rs): when each came, the lines that mark one read
// or archive it (src/ask.rs), and which windows are Mail's. Mail.qml asks
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

// The line that asks the program to mark mail read, or to archive it.
function tidy(mail, how) {
    return `${JSON.stringify({ [how]: { id: mail.id } })}\n`;
}

// What's said of mail once it's been marked read or archived, and the
// program no longer holds it as wanting Joel.
const TIDIED = { read: "read", archive: "archived" };

function tidied(how) {
    return TIDIED[how] ?? "";
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
