# GitHub's keys, a page in the browser, for the launcher's Shortcuts to
# list over it, before the browser's own (modules/keys/home.nix,
# launcher/KeyLayers.js). GitHub takes no settings for them, so these are
# its own, the ones used most from its list for Windows and Linux,
# docs.github.com/en/get-started/accessibility/keyboard-shortcuts. Its
# single keys are on unless they're turned off in its accessibility
# settings, and ? lists every key the page shown takes.
#
# The browser hears a key before the page, so none of its own are here,
# and Vimium is off on GitHub (home.nix), so its keys leave GitHub's: a
# list's c makes an issue, never Vimium's.
let
  grouped = group: map (key: key // { inherit group; });
  where = when: map (key: key // { inherit when; });
in
grouped "going" [
  {
    chord = "/";
    does = "Search GitHub";
  }
  {
    chord = "g n";
    does = "The notifications";
  }
  {
    chord = "g c";
    does = "The repository's code";
  }
  {
    chord = "g i";
    does = "The repository's issues";
  }
  {
    chord = "g p";
    does = "The repository's pull requests";
  }
  {
    chord = "g a";
    does = "The repository's actions";
  }
  {
    chord = "Shift+/";
    does = "Every key this page takes";
  }
]
++ grouped "code" (
  where "in a repository's code" [
    {
      chord = "t";
      does = "Find a file";
    }
    {
      chord = "w";
      does = "Another branch or tag";
    }
    {
      chord = "l";
      does = "Go to a line";
    }
    {
      chord = "b";
      does = "Who changed each line";
    }
    {
      chord = "y";
      does = "The page's lasting address, at its commit";
    }
    {
      chord = ".";
      does = "Open it in the editor on the web";
    }
  ]
)
++ grouped "lists" (
  where "in a list of issues or pull requests" [
    {
      chord = "c";
      does = "A new issue";
    }
    {
      chord = "o";
      does = "Open the one chosen";
    }
    {
      chord = "u";
      does = "By its author";
    }
    {
      chord = "l";
      does = "By its labels";
    }
    {
      chord = "m";
      does = "By its milestone";
    }
    {
      chord = "a";
      does = "By whom it's assigned to";
    }
  ]
)
++ grouped "changes" (
  where "on an issue or pull request" [
    {
      chord = "q";
      does = "Ask for a review";
    }
    {
      chord = "a";
      does = "Assign someone";
    }
    {
      chord = "l";
      does = "Label it";
    }
    {
      chord = "m";
      does = "Set its milestone";
    }
    {
      chord = "x";
      does = "Link another issue or pull request";
    }
  ]
  ++ [
    {
      chord = "r";
      does = "Quote the words in a reply";
      when = "with words selected";
    }
    {
      chord = "Ctrl+Shift+p";
      does = "Write or preview the comment";
      when = "in a comment";
    }
    {
      chord = "Ctrl+Enter";
      does = "Send the comment";
      when = "in a comment";
    }
  ]
)
