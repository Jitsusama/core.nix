"""Build the launcher's time zones from tzdata.

The shell's JavaScript has no time zones but its own, so this prints one
line of JSON for each zone, [zone, names, changes]. The names are the city
and, for a country with only that zone, the country, lowercased. The
changes are [when, minutes east of UTC, abbreviation], the first standing
from the start of `since` and each after it from the second given, until
the end of `until`.

    python3 zones.py zoneinfo since until > out
"""

import json
import sys
import zoneinfo
from datetime import UTC, datetime, timedelta
from pathlib import Path

# How a country tzdata names with a parenthesis is said.
SPOKEN = {
    "Samoa (American)": ["american samoa"],
    "Cocos (Keeling) Islands": ["cocos islands", "keeling islands"],
    "Congo (Dem. Rep.)": ["democratic republic of the congo", "dr congo"],
    "Congo (Rep.)": ["republic of the congo"],
    "Britain (UK)": ["britain", "uk", "united kingdom", "great britain", "england"],
    "Korea (North)": ["north korea"],
    "Korea (South)": ["south korea", "korea"],
    "St Martin (French)": ["saint martin", "st martin"],
    "Myanmar (Burma)": ["myanmar", "burma"],
    "St Maarten (Dutch)": ["sint maarten", "st maarten"],
    "Eswatini (Swaziland)": ["eswatini", "swaziland"],
    "Virgin Islands (UK)": ["british virgin islands"],
    "Virgin Islands (US)": ["us virgin islands"],
    "Samoa (western)": ["samoa"],
}

# The names a zone goes by beyond its city and country, as they're said in
# North America and of Europe's and Asia's largest.
KNOWN = {
    "America/New_York": ["eastern", "est", "edt", "et"],
    "America/Chicago": ["central", "cst", "cdt", "ct"],
    "America/Denver": ["mountain", "mst", "mdt", "mt"],
    "America/Los_Angeles": ["pacific", "pst", "pdt", "pt"],
    "Europe/Paris": ["cet", "cest"],
    "Asia/Tokyo": ["jst"],
    "Asia/Kolkata": ["ist"],
    "Australia/Sydney": ["aest", "aedt"],
}

# A zone's state at a moment: minutes east of UTC and its abbreviation.
type State = list[int | str | None]


def table(directory: Path, name: str) -> list[list[str]]:
    """Read one of tzdata's tab-separated tables, its comments left out."""
    with (directory / name).open(encoding="utf-8") as file:
        return [line.rstrip("\n").split("\t") for line in file if not line.startswith("#")]


def state(zone: zoneinfo.ZoneInfo, moment: datetime) -> State:
    """Give the zone's offset and abbreviation at the moment."""
    local = moment.astimezone(zone)
    offset = local.utcoffset()
    minutes = 0 if offset is None else int(offset.total_seconds() // 60)
    return [minutes, local.tzname()]


def at(seconds: int) -> datetime:
    """Give the moment a Unix time names."""
    return datetime.fromtimestamp(seconds, UTC)


def changes(name: str, since: int, until: int) -> list[list[int | str | None]]:
    """List the zone's changes from the start of since to the end of until."""
    zone = zoneinfo.ZoneInfo(name)
    moment = datetime(since, 1, 1, tzinfo=UTC)
    end = datetime(until + 1, 1, 1, tzinfo=UTC)
    found = [[int(moment.timestamp()), *state(zone, moment)]]
    day = timedelta(days=1)
    while moment < end:
        later = moment + day
        if state(zone, later) != found[-1][1:]:
            # The second it changed, found by halving the day.
            low, high = int(moment.timestamp()), int(later.timestamp())
            while high - low > 1:
                middle = (low + high) // 2
                if state(zone, at(middle)) == found[-1][1:]:
                    low = middle
                else:
                    high = middle
            found.append([high, *state(zone, at(high))])
        moment = later
    return found


def main(argv: list[str]) -> None:
    """Print each zone's line, then UTC's."""
    directory, since, until = Path(argv[0]), int(argv[1]), int(argv[2])
    zoneinfo.reset_tzpath([str(directory)])
    countries = {
        fields[0]: SPOKEN.get(fields[1], [fields[1].lower()])
        for fields in table(directory, "iso3166.tab")
    }
    zones: dict[str, list[str]] = {}
    for fields in table(directory, "zone.tab"):
        zones.setdefault(fields[2], []).append(fields[0])
    held: dict[str, list[str]] = {}
    for zone, codes in zones.items():
        for code in codes:
            held.setdefault(code, []).append(zone)

    for zone in sorted(zones):
        city = zone.split("/")[-1].replace("_", " ").lower()
        alone = [name for code in zones[zone] if held[code] == [zone] for name in countries[code]]
        names = list(dict.fromkeys([city, *alone, *KNOWN.get(zone, [])]))
        print(json.dumps([zone, names, changes(zone, since, until)], ensure_ascii=False))
    print(json.dumps(["UTC", ["utc", "gmt", "zulu"], changes("UTC", since, until)]))


if __name__ == "__main__":
    main(sys.argv[1:])
