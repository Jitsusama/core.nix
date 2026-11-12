# The launcher's vocabulary, joined from words/: `places`, each place's words
# by the place, a thing's place being the first part of its `in` and the
# launcher's own those with none; `frame`, what the launcher says beside its
# things (home, its categories, commands, field, meanings and terminal);
# `value`, all of it as one; and `file`, the one words.json the shell and
# every daemon read, so how they load it never changes. A place's words
# change in its file alone, so two changes to different places' words share
# no file, and one never makes the other's gate run again (WCJQSG,
# 2026-11-12). Builtins only, so a check and a package read it alike.
let
  read = file: builtins.fromJSON (builtins.readFile file);
  frame = ./words/frame.json;
  places = {
    "agents" = ./words/agents.json;
    "applications" = ./words/applications.json;
    "asks" = ./words/asks.json;
    "ask" = ./words/ask.json;
    "bluetooth" = ./words/bluetooth.json;
    "calculator" = ./words/calculator.json;
    "calendar" = ./words/calendar.json;
    "cameras" = ./words/cameras.json;
    "capture" = ./words/capture.json;
    "characters" = ./words/characters.json;
    "clipboard" = ./words/clipboard.json;
    "computer" = ./words/computer.json;
    "displays" = ./words/displays.json;
    "docs" = ./words/docs.json;
    "drive" = ./words/drive.json;
    "eero" = ./words/eero.json;
    "emoji" = ./words/emoji.json;
    "file-chooser" = ./words/file-chooser.json;
    "firmware" = ./words/firmware.json;
    "for-you" = ./words/for-you.json;
    "heat" = ./words/heat.json;
    "jobs" = ./words/jobs.json;
    "keyboard" = ./words/keyboard.json;
    "keyboards" = ./words/keyboards.json;
    "kitty" = ./words/kitty.json;
    "launcher" = ./words/launcher.json;
    "light" = ./words/light.json;
    "location" = ./words/location.json;
    "machine" = ./words/machine.json;
    "machines" = ./words/machines.json;
    "mail" = ./words/mail.json;
    "media" = ./words/media.json;
    "meet" = ./words/meet.json;
    "networks" = ./words/networks.json;
    "notices" = ./words/notices.json;
    "on" = ./words/on.json;
    "orchestrators" = ./words/orchestrators.json;
    "passwords" = ./words/passwords.json;
    "path" = ./words/path.json;
    "people" = ./words/people.json;
    "power" = ./words/power.json;
    "program" = ./words/program.json;
    "quests" = ./words/quests.json;
    "recent" = ./words/recent.json;
    "runs" = ./words/runs.json;
    "session" = ./words/session.json;
    "sheets" = ./words/sheets.json;
    "shortcuts" = ./words/shortcuts.json;
    "slack" = ./words/slack.json;
    "slides" = ./words/slides.json;
    "sound" = ./words/sound.json;
    "spend" = ./words/spend.json;
    "spotify" = ./words/spotify.json;
    "stopwatch" = ./words/stopwatch.json;
    "swayimg" = ./words/swayimg.json;
    "time" = ./words/time.json;
    "timer" = ./words/timer.json;
    "timers" = ./words/timers.json;
    "today" = ./words/today.json;
    "traffic" = ./words/traffic.json;
    "transcript" = ./words/transcript.json;
    "transcripts" = ./words/transcripts.json;
    "tuple" = ./words/tuple.json;
    "vendors" = ./words/vendors.json;
    "verb" = ./words/verb.json;
    "vm" = ./words/vm.json;
    "waits" = ./words/waits.json;
    "warp" = ./words/warp.json;
    "weather" = ./words/weather.json;
    "wifi" = ./words/wifi.json;
    "windows" = ./words/windows.json;
  };
  value = read frame // {
    things = builtins.concatLists (map (file: (read file).things) (builtins.attrValues places));
  };
in
{
  inherit frame places value;
  file = builtins.toFile "words.json" (builtins.toJSON value);
}
