// What the shell says of the mail that wants Joel, as the google program
// keeps it (src/mail.rs): when each came, and the lines that mark one read
// or archive it (src/ask.rs). Mail.qml asks these, and the checks run them
// in Qt's own engine (tests/mail.qml). Times are milliseconds since 1970,
// as the program gives them.
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
