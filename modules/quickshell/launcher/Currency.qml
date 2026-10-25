// Money from one currency to another, answered in the field: "100 usd in
// cad", "usd to eur", "5,000 jpy as cad". The rates are the European
// Central Bank's, which it sets once each working day, asked of Frankfurter
// only when a conversion is typed and kept for an hour; the footer says
// which day's they are. With `lookups` set false it answers nothing.
import qs
import QtQuick
import "Numbers.js" as Numbers

Provider {
    id: provider

    examples: ["100 usd in cad", "eur to jpy"]

    // The currencies the bank publishes a rate for.
    readonly property var codes: ["AUD", "BGN", "BRL", "CAD", "CHF", "CNY", "CZK", "DKK", "EUR",
        "GBP", "HKD", "HUF", "IDR", "ILS", "INR", "ISK", "JPY", "KRW", "MXN", "MYR", "NOK", "NZD",
        "PHP", "PLN", "RON", "SEK", "SGD", "THB", "TRY", "USD", "ZAR"]
    readonly property var months: ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep",
        "Oct", "Nov", "Dec"]

    function money(value, places) {
        return Numbers.format(value, places, settings.group, settings.point);
    }

    // Enough places to show a rate's first four figures.
    function rate(value) {
        return money(value, Math.max(2, 4 - Math.floor(Math.log10(Math.abs(value) || 1))));
    }

    function answer(typed) {
        if (settings.lookups === false)
            return null;
        const asked = /^(\d[\d,_]*(?:\.\d+)?\s*)?([a-z]{3})\s+(?:in|to|as)\s+([a-z]{3})$/i
            .exec(typed.trim());
        if (!asked)
            return null;
        const amount = asked[1] ? parseFloat(asked[1].replace(/[,_\s]/g, "")) : 1;
        const from = asked[2].toUpperCase();
        const to = asked[3].toUpperCase();
        if (!codes.includes(from) || !codes.includes(to) || from === to)
            return null;
        const said = ask(`https://api.frankfurter.dev/v1/latest?base=${from}&symbols=${to}`,
            60 * 60 * 1000);
        if (!said.value || !said.value.rates || said.value.rates[to] === undefined)
            return {
                headline: said.failed ? "no rate" : "\u2026",
                copy: "",
                rows: [],
                note: said.failed ? "Frankfurter didn't answer" : "asking Frankfurter",
                asking: !said.failed
            };
        const each = said.value.rates[to];
        const total = amount * each;
        const [year, month, day] = (said.value.date ?? "").split("-").map(Number);
        const when = month ? `${day} ${months[month - 1]}` : "";
        const cash = Theme.glyph.currency;
        return {
            headline: `${money(total, 2)} ${to}`,
            copy: money(total, 2).replace(/[^\d.-]/g, ""),
            rows: [
                copyRow({ name: `1 ${from} = ${rate(each)} ${to}`, side: "rate", glyph: cash,
                    copy: String(each) }),
                copyRow({ name: `1 ${to} = ${rate(1 / each)} ${from}`, side: "inverse", glyph: cash,
                    copy: String(1 / each) })
            ],
            note: `European Central Bank${when === "" ? "" : ` \u00b7 ${when}`}`
        };
    }
}
