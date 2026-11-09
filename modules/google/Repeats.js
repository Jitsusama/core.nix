// How a meeting's series repeats, in words, from the rule the google
// program reads (src/calendar.rs's Repeats), and the names of the days
// and months Calendar.js says a meeting's day by. Apart from Calendar.js,
// which imports it, so each stays a file one reads whole. The checks run
// it in Qt's own engine (tests/calendar.qml).
.pragma library

const DAYS = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
const MONTHS = ["January", "February", "March", "April", "May", "June", "July", "August",
    "September", "October", "November", "December"];

// What one of a series counts by, one and many.
const EVERY = {
    day: ["day", "days"],
    week: ["week", "weeks"],
    month: ["month", "months"],
    year: ["year", "years"]
};
// Which of a day in its month, from the first.
const WHICH = ["", "first", "second", "third", "fourth", "fifth"];
// Monday to Friday, as the days count from Sunday.
const WEEKDAYS = "1,2,3,4,5";
// An ordinal's ending goes by its last figure, but for 11th to 13th.
const TEN = 10;
const HUNDRED = 100;
const TEENS = [11, 12, 13];
const ENDINGS = ["th", "st", "nd", "rd"];

// 1st, 2nd, 3rd, 4th, 11th, 21st.
function ordinal(n) {
    const ending = TEENS.includes(n % HUNDRED) ? "th" : ENDINGS[n % TEN] ?? "th";
    return `${n}${ending}`;
}

// Which of a day in its month: first, or from the end, last and second
// last.
function which(n) {
    if (n > 0) {
        return WHICH[n] ?? ordinal(n);
    }
    return n === -1 ? "last" : `${WHICH[-n] ?? ordinal(-n)} last`;
}

// Words in a list, as one says them: "a", "a and b", "a, b and c".
function listed(words) {
    if (words.length < 2) {
        return words.join("");
    }
    // Qt's engine has no Array's at.
    return `${words.slice(0, -1).join(", ")} and ${words[words.length - 1]}`;
}

// How a meeting repeats, as its details say it: "Every weekday", "Every 2
// weeks on Monday and Wednesday", "Every month on the last Friday, until
// 31 December 2026"; or "" for one that doesn't, or whose series' rule
// wasn't heard (src/calendar.rs).
function repeated(repeats) {
    const [one, many] = EVERY[repeats?.every] ?? [];
    if (!one) {
        return "";
    }
    const days = repeats.days ?? [];
    const plain = days.every(day => day.nth === null);
    const weekdays = days.map(day => day.day).sort().join(",") === WEEKDAYS;
    let said = "";
    if (repeats.every === "week" && repeats.interval === 1 && plain && weekdays) {
        said = "Every weekday";
    } else {
        said = repeats.interval === 1 ? `Every ${one}` : `Every ${repeats.interval} ${many}`;
        const on = days.map(day => (day.nth === null
            ? DAYS[day.day] : `the ${which(day.nth)} ${DAYS[day.day]}`));
        const dates = (repeats.monthDays ?? []).map(day => (day > 0
            ? `the ${ordinal(day)}` : `the ${which(day)} day`));
        if (on.length + dates.length > 0) {
            said += ` on ${listed(on.concat(dates))}`;
        }
    }
    if (repeats.times) {
        return `${said}, ${repeats.times} times`;
    }
    if (repeats.until) {
        const last = new Date(repeats.until);
        return `${said}, until ${last.getDate()} ${MONTHS[last.getMonth()]} ${last.getFullYear()}`;
    }
    return said;
}
