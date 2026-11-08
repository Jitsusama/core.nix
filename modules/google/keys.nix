# The keys of Google's programs, each a page in the browser, for the
# launcher's Shortcuts to list over its window, before the browser's own
# (modules/keys/home.nix, launcher/KeyLayers.js). Google takes no settings
# for them, so these are its own, the ones used most from each program's
# list on its help pages, for Windows, ChromeOS and Linux:
#
#   Mail      support.google.com/mail/answer/6594
#   Calendar  support.google.com/calendar/answer/37034
#   Meet      support.google.com/a/users/answer/9896256
#   Docs      support.google.com/docs/answer/179738
#   Sheets    support.google.com/docs/answer/181110
#   Slides    support.google.com/docs/answer/1696717
#   Drive     support.google.com/drive/answer/2563044
#
# The browser hears a key before the page, so none of its own are here:
# Ctrl+h is Chrome's history, never Docs' find and replace, and Alt+1 its
# first tab, never Sheets' renaming. Mail's single keys work once Gmail's
# keyboard shortcuts are on in its settings, and Meet's in a call.
let
  grouped = group: map (key: key // { inherit group; });
  where = when: map (key: key // { inherit when; });
in
{
  google-mail =
    grouped "going" [
      {
        chord = "g i";
        does = "The inbox";
      }
      {
        chord = "g s";
        does = "Starred conversations";
      }
      {
        chord = "g t";
        does = "Sent mail";
      }
      {
        chord = "g d";
        does = "Drafts";
      }
      {
        chord = "j";
        does = "The older conversation";
      }
      {
        chord = "k";
        does = "The newer conversation";
      }
      {
        chord = "u";
        does = "Back to the list";
        when = "in a conversation";
      }
      {
        chord = "/";
        does = "Search the mail";
      }
    ]
    ++ grouped "conversations" [
      {
        chord = "c";
        does = "A new email";
      }
      {
        chord = "r";
        does = "Reply";
        when = "in a conversation";
      }
      {
        chord = "a";
        does = "Reply to all";
        when = "in a conversation";
      }
      {
        chord = "f";
        does = "Forward the email";
        when = "in a conversation";
      }
      {
        chord = "e";
        does = "Archive the conversation";
      }
      {
        chord = "#";
        does = "Delete the conversation";
      }
      {
        chord = "Shift+u";
        does = "Mark it unread";
      }
      {
        chord = "z";
        does = "Undo the last action";
      }
    ]
    ++ grouped "writing" (
      where "in a draft" [
        {
          chord = "Ctrl+Enter";
          does = "Send the email";
        }
        {
          chord = "Ctrl+Shift+c";
          does = "Add Cc recipients";
        }
        {
          chord = "Ctrl+Shift+b";
          does = "Add Bcc recipients";
        }
        {
          chord = "Ctrl+k";
          does = "A link";
        }
        {
          chord = "Ctrl+b";
          does = "Bold";
        }
        {
          chord = "Ctrl+i";
          does = "Italic";
        }
        {
          chord = "Ctrl+u";
          does = "Underline";
        }
        {
          chord = "Ctrl+Shift+8";
          does = "A bulleted list";
        }
      ]
    );

  google-calendar =
    grouped "views" [
      {
        chord = "d";
        does = "Day view";
      }
      {
        chord = "w";
        does = "Week view";
      }
      {
        chord = "m";
        does = "Month view";
      }
      {
        chord = "a";
        does = "Agenda view";
      }
      {
        chord = "x";
        does = "Custom view";
      }
      {
        chord = "t";
        does = "Today";
      }
      {
        chord = "j";
        does = "The next days";
      }
      {
        chord = "g";
        does = "Go to a date";
      }
    ]
    ++ grouped "events" [
      {
        chord = "c";
        does = "A new event";
      }
      {
        chord = "e";
        does = "The event's details";
        when = "with an event selected";
      }
      {
        chord = "Delete";
        does = "Delete the event";
        when = "with an event selected";
      }
      {
        chord = "Ctrl+s";
        does = "Save the event";
        when = "in an event's details";
      }
      {
        chord = "Escape";
        does = "Back to the calendar";
        when = "in an event's details";
      }
      {
        chord = "z";
        does = "Undo the last change";
      }
    ]
    ++ grouped "anywhere" [
      {
        chord = "/";
        does = "Search the calendar";
      }
      {
        chord = "r";
        does = "Refresh the calendar";
      }
      {
        chord = "+";
        does = "Add a calendar";
      }
      {
        chord = "s";
        does = "Calendar settings";
      }
      {
        chord = "?";
        does = "Every Calendar shortcut";
      }
    ];

  google-meet = where "in a call" (
    grouped "you" [
      {
        chord = "Ctrl+d";
        does = "Mute or unmute the microphone";
      }
      {
        chord = "Ctrl+e";
        does = "The camera on or off";
      }
      {
        chord = "Ctrl+Alt+h";
        does = "Raise or lower your hand";
      }
      {
        chord = "Ctrl+Alt+t";
        does = "Start presenting";
      }
      {
        chord = "Ctrl+Alt+m";
        does = "Your own video smaller or larger";
      }
    ]
    ++ grouped "seeing" [
      {
        chord = "Ctrl+Alt+c";
        does = "Show or hide the chat";
      }
      {
        chord = "Ctrl+Alt+p";
        does = "Show or hide everyone in the call";
      }
      {
        chord = "c";
        does = "Captions on or off";
      }
      {
        chord = "Shift+m";
        does = "The call in a picture of its own";
      }
      {
        chord = "Ctrl+Alt+k";
        does = "More people's tiles";
      }
      {
        chord = "Ctrl+Alt+j";
        does = "Fewer people's tiles";
      }
    ]
  );

  google-docs =
    grouped "text" [
      {
        chord = "Ctrl+b";
        does = "Bold";
      }
      {
        chord = "Ctrl+i";
        does = "Italic";
      }
      {
        chord = "Ctrl+u";
        does = "Underline";
      }
      {
        chord = "Alt+Shift+5";
        does = "Strikethrough";
      }
      {
        chord = "Ctrl+k";
        does = "A link";
      }
      {
        chord = "Ctrl+\\";
        does = "Clear the formatting";
        when = "with text selected";
      }
      {
        chord = "Ctrl+Shift+v";
        does = "Paste without formatting";
      }
    ]
    ++ grouped "paragraphs" [
      {
        chord = "Ctrl+Alt+0";
        does = "Normal text";
      }
      {
        chord = "Ctrl+Alt+1";
        does = "Heading 1";
      }
      {
        chord = "Ctrl+Alt+2";
        does = "Heading 2";
      }
      {
        chord = "Ctrl+Alt+3";
        does = "Heading 3";
      }
      {
        chord = "Ctrl+Shift+7";
        does = "A numbered list";
      }
      {
        chord = "Ctrl+Shift+8";
        does = "A bulleted list";
      }
      {
        chord = "Ctrl+Shift+9";
        does = "A checklist";
      }
      {
        chord = "Ctrl+]";
        does = "Indent more";
      }
      {
        chord = "Ctrl+[";
        does = "Indent less";
      }
    ]
    ++ grouped "reviewing" [
      {
        chord = "Ctrl+Alt+m";
        does = "A comment";
        when = "with text selected";
      }
      {
        chord = "Ctrl+Alt+Shift+x";
        does = "Suggest edits";
      }
      {
        chord = "Ctrl+Alt+Shift+z";
        does = "Edit directly";
      }
      {
        chord = "Ctrl+Shift+c";
        does = "Count the words";
      }
      {
        chord = "Ctrl+Alt+Shift+h";
        does = "The version history";
      }
      {
        chord = "Ctrl+Alt+f";
        does = "A footnote";
      }
      {
        chord = "Ctrl+Enter";
        does = "A page break";
      }
      {
        chord = "Alt+/";
        does = "Search the menus";
      }
    ];

  google-sheets =
    grouped "rows" [
      {
        chord = "Shift+Space";
        does = "Select the row";
      }
      {
        chord = "Ctrl+Space";
        does = "Select the column";
      }
      {
        chord = "Ctrl+Alt+=";
        does = "Insert rows or columns";
        when = "with rows or columns selected";
      }
      {
        chord = "Ctrl+Alt+-";
        does = "Delete rows or columns";
        when = "with rows or columns selected";
      }
      {
        chord = "Ctrl+Alt+9";
        does = "Hide the row";
      }
      {
        chord = "Ctrl+Alt+0";
        does = "Hide the column";
      }
      {
        chord = "Alt+Shift+Right";
        does = "Group rows or columns";
      }
      {
        chord = "Alt+Shift+Left";
        does = "Ungroup rows or columns";
      }
    ]
    ++ grouped "cells" [
      {
        chord = "Ctrl+d";
        does = "Fill down";
      }
      {
        chord = "Ctrl+Enter";
        does = "Fill the selection";
      }
      {
        chord = "Ctrl+;";
        does = "Today's date";
      }
      {
        chord = "Ctrl+Shift+;";
        does = "The time now";
      }
      {
        chord = "Ctrl+Shift+v";
        does = "Paste only the values";
      }
      {
        chord = "Ctrl+Shift+1";
        does = "As a decimal";
      }
      {
        chord = "Ctrl+Shift+4";
        does = "As currency";
      }
      {
        chord = "Ctrl+Shift+5";
        does = "As a percentage";
      }
      {
        chord = "Ctrl+\\";
        does = "Clear the formatting";
      }
      {
        chord = "Ctrl+Alt+m";
        does = "A comment";
      }
    ]
    ++ grouped "sheets" [
      {
        chord = "Alt+Down";
        does = "The next sheet";
      }
      {
        chord = "Alt+Up";
        does = "The sheet before";
      }
      {
        chord = "Alt+Shift+k";
        does = "Every sheet, listed";
      }
      {
        chord = "Shift+F11";
        does = "A new sheet";
      }
      {
        chord = "Ctrl+Home";
        does = "The sheet's start";
      }
      {
        chord = "Ctrl+End";
        does = "The sheet's end";
      }
    ];

  google-slides =
    grouped "slides" [
      {
        chord = "Ctrl+m";
        does = "A new slide";
      }
      {
        chord = "Ctrl+Alt+Shift+s";
        does = "The speaker notes";
      }
      {
        chord = "Ctrl+F5";
        does = "Present the slides";
      }
    ]
    ++ grouped "slides" (
      where "in the filmstrip" [
        {
          chord = "Ctrl+d";
          does = "A copy of the slide";
        }
        {
          chord = "Ctrl+Up";
          does = "Move the slide up";
        }
        {
          chord = "Ctrl+Down";
          does = "Move the slide down";
        }
        {
          chord = "Ctrl+Shift+Up";
          does = "Move the slide to the start";
        }
        {
          chord = "Ctrl+Shift+Down";
          does = "Move the slide to the end";
        }
      ]
    )
    ++ grouped "objects" [
      {
        chord = "Ctrl+Alt+g";
        does = "Group the objects";
      }
      {
        chord = "Ctrl+Alt+Shift+g";
        does = "Ungroup the objects";
      }
      {
        chord = "Ctrl+Alt+m";
        does = "A comment";
      }
    ]
    ++ grouped "objects" (
      where "with an object selected" [
        {
          chord = "Ctrl+Up";
          does = "Bring it forward";
        }
        {
          chord = "Ctrl+Down";
          does = "Send it backward";
        }
        {
          chord = "Ctrl+Shift+Up";
          does = "Bring it to the front";
        }
        {
          chord = "Ctrl+Shift+Down";
          does = "Send it to the back";
        }
      ]
    )
    ++ grouped "presenting" (
      where "while presenting" [
        {
          chord = "Right";
          does = "The next slide";
        }
        {
          chord = "Left";
          does = "The slide before";
        }
        {
          chord = "Home";
          does = "The first slide";
        }
        {
          chord = "End";
          does = "The last slide";
        }
        {
          chord = "b";
          does = "A black screen";
        }
        {
          chord = "s";
          does = "The speaker notes";
        }
        {
          chord = "l";
          does = "The laser pointer on or off";
        }
        {
          chord = "Escape";
          does = "Stop presenting";
        }
      ]
    );

  google-drive =
    grouped "making" [
      {
        chord = "/";
        does = "Search Drive";
      }
      {
        chord = "Alt+c t";
        does = "A new document";
      }
      {
        chord = "Alt+c s";
        does = "A new spreadsheet";
      }
      {
        chord = "Alt+c f";
        does = "A new folder";
      }
      {
        chord = "Alt+c u";
        does = "Upload a file";
      }
      {
        chord = "Alt+v l";
        does = "A grid or a list";
      }
      {
        chord = "Alt+v d";
        does = "Show or hide the details";
      }
      {
        chord = "Ctrl+/";
        does = "Every Drive shortcut";
      }
    ]
    ++ grouped "files" (
      where "with a file selected" [
        {
          chord = "Enter";
          does = "Open it";
        }
        {
          chord = "Ctrl+Enter";
          does = "Open it in a new tab";
        }
        {
          chord = "Ctrl+Alt+p";
          does = "Preview it";
        }
        {
          chord = "F2";
          does = "Rename it";
        }
        {
          chord = "Ctrl+Alt+a";
          does = "Share it";
        }
        {
          chord = "Ctrl+Alt+s";
          does = "Star it, or not";
        }
        {
          chord = "Ctrl+Alt+m";
          does = "Move it to a folder";
        }
        {
          chord = "Ctrl+Alt+l";
          does = "Copy its link";
        }
        {
          chord = "Delete";
          does = "Remove it";
        }
      ]
    );
}
