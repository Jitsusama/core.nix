// The cases of measures for the launcher's answers, which ../answers.qml runs
// with every other place's; its opening comment says how.
.import "common.js" as Common
.import "../Units.js" as Units

// What's typed, and the conversion's headline.
var conversions = ({
    "5 km in mi": "3.10686 mi",
    "1 mi to km": "1.60934 km",
    "6 ft in cm": "182.88 cm",
    "3 in in cm": "7.62 cm",
    "3 inches to cm": "7.62 cm",
    "5 nautical miles in km": "9.26 km",
    "1e3 m in km": "1 km",
    "1 lb in kg": "0.453592 kg",
    "1,500 g as kg": "1.5 kg",
    "2 cups in ml": "473.176 mL",
    "100 km/h in mph": "62.1371 mph",
    "100 c in f": "212 \u00b0F",
    "350f to c": "176.667 \u00b0C",
    "0 kelvin in celsius": "-273.15 \u00b0C",
    "1 gib in mb": "1,073.74 MB",
    "10 gbit in gb": "1.25 GB",
    "1 year in days": "365.25 d",
    "90 deg in rad": "1.5708 rad",
    "180 degrees in turns": "0.5 turn",
    "1 kwh in kcal": "860.421 kcal",
    "1 atm in psi": "14.6959 psi",
    "25ms in s": "0.025 s",
    "25' in m": "7.62 m",
    // A measure alone is said in its kind's base unit, and a
    // temperature in the other scale people use.
    "25ms": "0.025 s",
    "5 km": "5,000 m",
    "3 ft": "0.9144 m",
    "2 hours": "7,200 s",
    "100\u00b0f": "37.7778 \u00b0C",
    // What the table can't, the maths works out.
    "5 parsecs in km": "154,283,879,074,568 km",
    "5 ft + 3 in in cm": "160.02 cm",
    "60 mph * 2 h": "120 mi",
    "1 gib / (10 mbit/s) in min": "14.3166 min",
    "3 cups + 2 tbsp in ml": "739.338 mL",
    "2 pints + 1 quart in l": "1.89271 L"
});

// A letter after a number is a word's end as often as a unit ("4k",
// "3d", "1st"), so a measure alone needs a longer name.
var notConversions = ["5 km in kg", "5 km in km", "100 usd in cad",
    "100 USD in CAD", "255 in hex", "km in mi", "kitty", "4k", "3d", "1st", "100 c",
    "ws 3", "2fa", "5 km + 3 kg", "1 m / 1 cm", "12*7", "5 m"];

function run(failures, fromRoot) {
    let asked = 0;
    const Maths = fromRoot.maths;
    const quoted = Common.quoted;
    for (const typed in conversions) {
        asked++;
        const got = Units.answer(typed, undefined, Maths);
        if (!got || got.headline !== conversions[typed])
            failures.push(`"${typed}" converted to ${quoted(got && got.headline)}, `
                + `not "${conversions[typed]}"`);
    }
    for (const typed of notConversions) {
        asked++;
        const got = Units.answer(typed, undefined, Maths);
        if (got)
            failures.push(`"${typed}" converted to ${quoted(got.headline)}, `
                + "but nothing should");
    }
    const rated = Units.answer("5 km in mi", undefined, Maths);
    asked++;
    if (!rated || quoted(rated.rows.map(r => `${r.text} ${r.side}`))
            !== quoted(["1 km 0.621371 mi", "1 mi 1.60934 km"]))
        failures.push(`"5 km in mi" gave the rows ${quoted(rated && rated.rows)}`);
    // A temperature's third scale is named first, as a part is.
    const scaled = Units.answer("100 c in f", undefined, Maths);
    asked++;
    if (!scaled || quoted(scaled.rows.map(r => `${r.text} ${r.side}`))
            !== quoted(["Kelvin 373.15 K"]))
        failures.push(`"100 c in f" gave the rows ${quoted(scaled && scaled.rows)}`);
    return asked;
}
