# Builds the launcher's time zones from tzdata, since the shell's JavaScript
# has no time zones but its own: one line of JSON for each zone,
# [zone, names, changes], where names are the city and, for a country with
# only that zone, the country, lowercased; and changes are [when, minutes
# east of UTC, abbreviation], the first standing from the start of `since`
# and each after it from the second given, until the end of `until`.
#
#     python3 zones.py zoneinfo since until > out
import json
import sys
import zoneinfo
from datetime import datetime, timedelta, timezone

directory, since, until = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
zoneinfo.reset_tzpath([directory])


def table(name):
    with open(f"{directory}/{name}", encoding="utf-8") as file:
        return [line.rstrip("\n").split("\t") for line in file if not line.startswith("#")]


# How a country tzdata names with a parenthesis is said.
spoken = {
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
known = {
    "America/New_York": ["eastern", "est", "edt", "et"],
    "America/Chicago": ["central", "cst", "cdt", "ct"],
    "America/Denver": ["mountain", "mst", "mdt", "mt"],
    "America/Los_Angeles": ["pacific", "pst", "pdt", "pt"],
    "Europe/Paris": ["cet", "cest"],
    "Asia/Tokyo": ["jst"],
    "Asia/Kolkata": ["ist"],
    "Australia/Sydney": ["aest", "aedt"],
}

countries = {
    fields[0]: spoken.get(fields[1], [fields[1].lower()]) for fields in table("iso3166.tab")
}
zones = {}
for fields in table("zone.tab"):
    zones.setdefault(fields[2], []).append(fields[0])
held = {}
for zone, codes in zones.items():
    for code in codes:
        held.setdefault(code, []).append(zone)


def state(zone, moment):
    local = moment.astimezone(zone)
    return [int(local.utcoffset().total_seconds() // 60), local.tzname()]


def at(seconds):
    return datetime.fromtimestamp(seconds, timezone.utc)


def changes(name):
    zone = zoneinfo.ZoneInfo(name)
    moment = datetime(since, 1, 1, tzinfo=timezone.utc)
    end = datetime(until + 1, 1, 1, tzinfo=timezone.utc)
    found = [[int(moment.timestamp())] + state(zone, moment)]
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
            found.append([high] + state(zone, at(high)))
        moment = later
    return found


def names(zone, codes):
    city = zone.split("/")[-1].replace("_", " ").lower()
    alone = [name for code in codes if held[code] == [zone] for name in countries[code]]
    return list(dict.fromkeys([city] + alone + known.get(zone, [])))


for zone in sorted(zones):
    print(json.dumps([zone, names(zone, zones[zone]), changes(zone)], ensure_ascii=False))
print(json.dumps(["UTC", ["utc", "gmt", "zulu"], changes("UTC")]))
