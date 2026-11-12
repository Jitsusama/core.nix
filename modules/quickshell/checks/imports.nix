# What each of the shell's files that the checks copy imports of the
# shell's own, by its path under modules/quickshell. helpers.nix's copied
# brings these with each file a check names, and a file whose imports differ
# from its row here fails every check copying it, so a new import goes here
# in the same change. A file importing nothing of the shell's has no row.
{
  "Choice.qml" = [
    "Columns.js"
    "Measures.js"
    "Reading.js"
  ];
  "CommandBinding.js" = [
    "Commands.js"
    "Ex.js"
    "Operators.js"
  ];
  "Commands.js" = [ "Ex.js" ];
  "ConnectedKeyboards.qml" = [ "launcher/Keyboards.js" ];
  "Copied.qml" = [ "Copied.js" ];
  "Daemon.qml" = [ "Hearing.js" ];
  "Devices.js" = [ "Crossing.js" ];
  "DrawerPage.js" = [ "Search.js" ];
  "DrawerValue.js" = [
    "Identifier.js"
    "Reading.js"
  ];
  "Editing.js" = [ "Typing.js" ];
  "FileFacts.js" = [ "When.js" ];
  "Files.qml" = [ "FileFacts.js" ];
  "Finding.js" = [ "LineEditing.js" ];
  "ForYouHolds.qml" = [
    "launcher/Looks.js"
    "pieces/Holding.js"
  ];
  "KeyboardLayout.qml" = [ "launcher/Keyboards.js" ];
  "Mark.qml" = [ "MarkSize.js" ];
  "ModeMarks.js" = [ "QueryKeys.js" ];
  "Niri.qml" = [ "NiriRecords.js" ];
  "NoiseCancelling.qml" = [ "launcher/ByEar.js" ];
  "NoticeKeys.js" = [ "LineEditing.js" ];
  "NoticeList.js" = [ "launcher/Order.js" ];
  "NoticeRow.qml" = [ "Columns.js" ];
  "PiSource.qml" = [ "PiSource.js" ];
  "PixelMark.qml" = [
    "MarkSize.js"
    "Pixels.js"
  ];
  "Prompt.qml" = [
    "Bands.js"
    "Confirm.js"
    "Editing.js"
    "KeyNames.js"
    "MarkSize.js"
    "Measures.js"
    "Operators.js"
    "QueryKeys.js"
    "Reading.js"
    "Typing.js"
  ];
  "PromptFoot.qml" = [ "Bands.js" ];
  "PromptHead.qml" = [ "Bands.js" ];
  "QueryKeys.js" = [
    "Finding.js"
    "Operators.js"
  ];
  "ReadsOn.qml" = [
    "Measures.js"
    "Reading.js"
  ];
  "ScreenPicture.qml" = [ "Wallpaper.js" ];
  "Sessions.qml" = [
    "PiSource.js"
    "Places.js"
    "SessionWindows.js"
    "Viewers.js"
    "Waiting.js"
    "launcher/Order.js"
  ];
  "SetWords.js" = [ "Commands.js" ];
  "SpendWatch.qml" = [ "SpendWatch.js" ];
  "StatusLine.qml" = [ "Bands.js" ];
  "Taken.qml" = [ "Fuzzy.js" ];
  "Trials.qml" = [ "launcher/Trials.js" ];
  "Waiting.js" = [ "PiSource.js" ];
  "launcher/Accessories.qml" = [
    "Devices.js"
    "Fuzzy.js"
    "launcher/Links.js"
    "launcher/Nearby.js"
    "launcher/Order.js"
  ];
  "launcher/AgentsFold.qml" = [ "launcher/AgentWindows.js" ];
  "launcher/Applications.qml" = [
    "FileFacts.js"
    "Fuzzy.js"
    "launcher/Captured.js"
    "launcher/HeldWindows.js"
    "launcher/Media.js"
    "launcher/Order.js"
    "launcher/Program.js"
    "launcher/Recent.js"
  ];
  "launcher/Ask.qml" = [
    "Fuzzy.js"
    "launcher/SessionRow.js"
  ];
  "launcher/Backlight.js" = [
    "launcher/Numbers.js"
    "launcher/Order.js"
  ];
  "launcher/ByEar.js" = [ "launcher/Numbers.js" ];
  "launcher/Calculator.qml" = [ "launcher/Numbers.js" ];
  "launcher/Capture.qml" = [
    "FileFacts.js"
    "Fuzzy.js"
    "launcher/Captured.js"
    "launcher/HeldWindows.js"
    "launcher/Order.js"
    "launcher/Recent.js"
  ];
  "launcher/Captured.js" = [
    "FileFacts.js"
    "When.js"
    "launcher/Recent.js"
  ];
  "launcher/Characters.qml" = [ "launcher/Characters.js" ];
  "launcher/Clocks.qml" = [
    "launcher/Ago.js"
    "launcher/Keeping.js"
    "launcher/Moment.js"
    "launcher/NowDrawers.js"
    "launcher/Numbers.js"
    "launcher/Zones.js"
  ];
  "launcher/Displays.qml" = [
    "Fuzzy.js"
    "Hearing.js"
    "launcher/Backlight.js"
    "launcher/Numbers.js"
    "launcher/Order.js"
    "launcher/Power.js"
    "launcher/ScreenAnswers.js"
    "launcher/Screens.js"
    "launcher/Seen.js"
    "launcher/Warmth.js"
  ];
  "launcher/Emoji.js" = [ "Fuzzy.js" ];
  "launcher/ForYou.js" = [ "launcher/Order.js" ];
  "launcher/ForYouFilter.js" = [ "launcher/Order.js" ];
  "launcher/Heat.qml" = [ "launcher/Seen.js" ];
  "launcher/HomeNetwork.js" = [ "launcher/Numbers.js" ];
  "launcher/Keeping.js" = [ "launcher/Numbers.js" ];
  "launcher/KeyMap.qml" = [
    "Fuzzy.js"
    "launcher/KeyGroups.js"
    "launcher/KeyLayers.js"
  ];
  "launcher/Keyboard.qml" = [
    "Fuzzy.js"
    "launcher/Keyboards.js"
  ];
  "launcher/Keyboards.qml" = [
    "launcher/KeyCounts.js"
    "launcher/Keyboards.js"
  ];
  "launcher/Lately.js" = [ "launcher/Order.js" ];
  "launcher/Light.qml" = [ "launcher/Numbers.js" ];
  "launcher/Looks.js" = [
    "launcher/ForYou.js"
    "launcher/Order.js"
  ];
  "launcher/Machine.js" = [ "launcher/Numbers.js" ];
  "launcher/Machine.qml" = [
    "launcher/Holders.js"
    "launcher/HomeNetwork.js"
    "launcher/Interfaces.js"
    "launcher/Order.js"
    "launcher/SessionRow.js"
  ];
  "launcher/Media.js" = [ "Music.js" ];
  "launcher/Moment.js" = [ "launcher/Numbers.js" ];
  "launcher/Nearby.js" = [ "launcher/Order.js" ];
  "launcher/NeedsYou.js" = [ "launcher/Ago.js" ];
  "launcher/Networks.qml" = [
    "launcher/Addresses.js"
    "launcher/Interfaces.js"
    "launcher/Networkd.js"
    "launcher/Numbers.js"
    "launcher/Order.js"
    "launcher/Subnets.js"
  ];
  "launcher/Now.qml" = [
    "Fuzzy.js"
    "Hearing.js"
    "Marks.js"
    "When.js"
    "launcher/Ago.js"
    "launcher/Choices.js"
    "launcher/ForYou.js"
    "launcher/ForYouFilter.js"
    "launcher/Here.js"
    "launcher/Lately.js"
    "launcher/Looks.js"
    "launcher/Moment.js"
    "launcher/NeedsYou.js"
    "launcher/NowGroups.js"
    "launcher/Order.js"
    "launcher/PlaceRow.js"
    "launcher/Showing.js"
    "pieces/Holding.js"
  ];
  "launcher/NowGroups.js" = [ "launcher/Order.js" ];
  "launcher/Orchestrators.qml" = [
    "launcher/Order.js"
  ];
  "launcher/PastNotices.qml" = [
    "Fuzzy.js"
    "launcher/Notice.js"
  ];
  "launcher/Power.qml" = [ "launcher/Seen.js" ];
  "launcher/Program.js" = [
    "FileFacts.js"
    "launcher/Numbers.js"
  ];
  "launcher/Provider.qml" = [
    "Fuzzy.js"
    "ProgramGlyphs.js"
    "launcher/Asking.js"
    "launcher/Member.js"
    "launcher/Numbers.js"
  ];
  "launcher/Quests.qml" = [
    "Fuzzy.js"
    "When.js"
    "launcher/Order.js"
    "launcher/RealmMachines.js"
  ];
  "launcher/Queue.qml" = [
    "Fuzzy.js"
    "launcher/Numbers.js"
    "launcher/Order.js"
  ];
  "launcher/ReadyRows.js" = [
    "When.js"
    "launcher/Ago.js"
    "launcher/Interfaces.js"
  ];
  "launcher/ReadyRows.qml" = [
    "Fuzzy.js"
    "launcher/Beneath.js"
    "launcher/Found.js"
    "launcher/Moved.js"
    "launcher/ReadyRows.js"
  ];
  "launcher/RealmMachines.js" = [
    "launcher/Machine.js"
    "launcher/Order.js"
  ];
  "launcher/Recent.js" = [ "launcher/Order.js" ];
  "launcher/Recents.qml" = [ "launcher/Recents.js" ];
  "launcher/Runs.qml" = [
    "Fuzzy.js"
    "PiSource.js"
    "launcher/Order.js"
  ];
  "launcher/Showing.js" = [ "launcher/Order.js" ];
  "launcher/Sound.qml" = [
    "Fuzzy.js"
    "launcher/Ago.js"
    "launcher/ByEar.js"
    "launcher/ForYou.js"
    "launcher/Order.js"
    "launcher/Sounds.js"
    "launcher/Targets.js"
  ];
  "launcher/Spend.qml" = [ "Fuzzy.js" ];
  "launcher/Subnets.js" = [
    "launcher/Addresses.js"
    "launcher/Numbers.js"
  ];
  "launcher/Today.qml" = [
    "launcher/Moment.js"
    "launcher/NowDrawers.js"
    "launcher/Numbers.js"
  ];
  "launcher/Traffic.qml" = [
    "launcher/Interfaces.js"
    "launcher/Machine.js"
    "launcher/Numbers.js"
    "launcher/Order.js"
    "launcher/Traffic.js"
  ];
  "launcher/Vendors.qml" = [ "launcher/Addresses.js" ];
  "launcher/Waits.js" = [ "launcher/Numbers.js" ];
  "launcher/Waits.qml" = [
    "launcher/Numbers.js"
    "launcher/Waits.js"
  ];
  "launcher/Weather.qml" = [
    "launcher/Moment.js"
    "launcher/Numbers.js"
  ];
  "launcher/Windows.qml" = [
    "Fuzzy.js"
    "launcher/HeldWindows.js"
    "launcher/KeyLayers.js"
    "launcher/Order.js"
  ];
  "pieces/Disk.qml" = [
    "FileFacts.js"
    "launcher/ReadyRows.js"
    "pieces/Drawers.js"
  ];
  "pieces/Drawers.js" = [ "FileFacts.js" ];
}
