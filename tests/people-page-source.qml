// A stand-in People source that keeps a page for each person it knows,
// as the machine's own repository's sources may (Vault's), for
// tests/people.qml: it knows Ada by her address, her team and her time
// zone, and keeps a page for her alone, so People lists Open in Pages
// for her, and is still finding out for Cal, so his stands dimmed.
import qs
import QtQuick

PersonSource {
    called: "Pages"
    people: [{
        emails: ["ada@example.com"],
        facts: [{ label: "Team", value: "Shell" },
            { label: "Time Zone", value: "Asia/Kolkata", zone: true }]
    }]

    function page(person) {
        return person.emails.includes("ada@example.com") ? "https://pages.example.com/ada" : "";
    }

    function looking(person) {
        return person.emails.includes("cal@example.com");
    }
}
