# Neovim's keys, one home for each: home.nix writes them as lua/bindings.lua,
# which keys.lua binds every one of, and they go into the key map (jitsusama.keys.neovim)
# where the launcher finds them by what they do. Space is Neovim's layer,
# its leader, and after it come the words every layer shares
# (docs/design.md, The Keyboard): w closes, Tab goes back to the one used
# last, Return opens a new one, Space finds, ? shows what's here, the arrows
# go that way and Shift brings the split along, and 1 to 9 pick by number.
# Each group is a letter (b the open files, c code, g git), and the group
# then Space finds among what it holds.
#
# A chord is written as the map spells it, keys pressed one after another
# with a space between, and a capital letter alone is that letter with
# Shift: Space b W. What a key does is one of:
#
#   lua      Lua run when the key is pressed, as a function's body
#   action   a function a plugin's file registers by this name, with
#            require('keys').act(name, function), for one too long to say here
#   command  an Ex command
#   keys     keys for Neovim to press in its place, through other maps
#            when remap is set
#   plugin   the name a plugin gives one of its actions, for a key in its
#            own window, with options the Lua for what it's given
#
# modes are Neovim's (n, x for a selection, o after an operator, i while
# typing, t in a terminal), normal mode unless said. A key with a scope is
# bound in a buffer only when that scope arrives: code when a language
# server attaches, git when gitsigns does, status in fugitive's window, and
# a plugin's window's when the plugin opens it.
let
  selected = [ "x" ];
  typing = [ "i" ];

  # A plugin window's keys, which the plugin binds in its window.
  inWindow = scope: map (key: key // { inherit scope; });

  # Neovim's own fold keys, in a window that folds what it lists, by the
  # names its plugin gives them.
  folds =
    names:
    map
      (fold: {
        inherit (fold) chord does;
        plugin = names.${fold.name};
      })
      [
        {
          chord = "z a";
          does = "Fold here, or unfold";
          name = "toggle";
        }
        {
          chord = "z A";
          does = "Fold here all the way down, or unfold";
          name = "toggleAll";
        }
        {
          chord = "z o";
          does = "Unfold here";
          name = "open";
        }
        {
          chord = "z O";
          does = "Unfold here all the way down";
          name = "openAll";
        }
        {
          chord = "z c";
          does = "Fold here";
          name = "close";
        }
        {
          chord = "z C";
          does = "Fold here all the way down";
          name = "closeAll";
        }
        {
          chord = "z r";
          does = "Unfold a level everywhere";
          name = "less";
        }
        {
          chord = "z R";
          does = "Unfold everything";
          name = "none";
        }
        {
          chord = "z m";
          does = "Fold a level more everywhere";
          name = "more";
        }
        {
          chord = "z M";
          does = "Fold everything";
          name = "all";
        }
      ];
  outlineFolds = {
    toggle = "actions.tree_toggle";
    toggleAll = "actions.tree_toggle_recursive";
    open = "actions.tree_open";
    openAll = "actions.tree_open_recursive";
    close = "actions.tree_close";
    closeAll = "actions.tree_close_recursive";
    less = "actions.tree_increase_fold_level";
    none = "actions.tree_open_all";
    more = "actions.tree_decrease_fold_level";
    all = "actions.tree_close_all";
  };
  problemFolds = {
    toggle = "fold_toggle";
    toggleAll = "fold_toggle_recursive";
    open = "fold_open";
    openAll = "fold_open_recursive";
    close = "fold_close";
    closeAll = "fold_close_recursive";
    less = "fold_reduce";
    none = "fold_open_all";
    more = "fold_more";
    all = "fold_close_all";
  };
  both = [
    "n"
    "x"
  ];
  around = [
    "x"
    "o"
  ];
  moving = [
    "n"
    "x"
    "o"
  ];

  # The four ways, for the keys that say "that way": where the split is,
  # and where it can be moved to.
  ways = {
    Left = "on the left";
    Down = "below";
    Up = "above";
    Right = "on the right";
  };
  edges = {
    Left = "far left";
    Down = "bottom";
    Up = "top";
    Right = "far right";
  };
  wincmd = {
    Left = "h";
    Down = "j";
    Up = "k";
    Right = "l";
  };

  # octo's keys in its own buffers, by what they act on, for the windows
  # that share them; each names the octo action it runs. Space g h holds
  # them, as it holds GitHub's keys everywhere else.
  octo = rec {
    key = chord: does: plugin: { inherit chord does plugin; };
    thing = thing: options: [
      (key "Return" "Choose what to do with the ${thing}" options)
      (key "Space g h b w" "Close the ${thing}" "close_issue")
      (key "Space g h b R" "Reopen the ${thing}" "reopen_issue")
      (key "Ctrl+L" "Read the ${thing} again" "reload")
      (key "Space g h b o" "Open the ${thing} in the browser" "open_in_browser")
      (key "Space g h b y" "Copy the ${thing}'s link" "copy_url")
    ];
    assignees = [
      (key "Space g h a Return" "Add someone to the assignees" "add_assignee")
      (key "Space g h a w" "Take someone off the assignees" "remove_assignee")
    ];
    reviewers = [
      (key "Space g h a Shift+Return" "Ask someone to review" "add_reviewer")
      (key "Space g h a W" "Take someone off the reviewers" "remove_reviewer")
    ];
    labels = [
      (key "Space g h l Return" "Make a label" "create_label")
      (key "Space g h l a" "Put a label on" "add_label")
      (key "Space g h l w" "Take a label off" "remove_label")
    ];
    comments = [
      (key "Space g h c Return" "Comment" "add_comment")
      (key "Space g h c r" "Reply to the comment here" "add_reply")
      (key "Space g h c w" "Delete the comment here" "delete_comment")
      (key "Space g h c h" "Show how the comment here was edited" "comment_edits")
      (key "Space g h c i" "Quote the comment here in a new issue" "reference_in_new_issue")
      (key "] C" "Go to the next comment" "next_comment")
      (key "[ C" "Go to the comment before" "prev_comment")
      (key "Space g h i g" "Go to the issue named here" "goto_issue")
    ];
    threads = [
      (key "Space g h T r" "Resolve the thread" "resolve_thread")
      (key "Space g h T u" "Open the thread again" "unresolve_thread")
    ];
    reactions =
      map
        (reaction: key "Space g h t ${reaction.key}" "React with ${reaction.word}" "react_${reaction.name}")
        [
          {
            key = "p";
            name = "hooray";
            word = "a party";
          }
          {
            key = "h";
            name = "heart";
            word = "a heart";
          }
          {
            key = "e";
            name = "eyes";
            word = "eyes";
          }
          {
            key = "+";
            name = "thumbs_up";
            word = "a thumbs up";
          }
          {
            key = "-";
            name = "thumbs_down";
            word = "a thumbs down";
          }
          {
            key = "r";
            name = "rocket";
            word = "a rocket";
          }
          {
            key = "l";
            name = "laugh";
            word = "a laugh";
          }
          {
            key = "c";
            name = "confused";
            word = "a confused face";
          }
        ];
    changes = [
      (key "] v" "Go to the next changed file" "select_next_entry")
      (key "[ v" "Go to the changed file before" "select_prev_entry")
      (key "[ V" "Go to the first changed file" "select_first_entry")
      (key "] V" "Go to the last changed file" "select_last_entry")
      (key "] u" "Go to the next file not yet viewed" "select_next_unviewed_entry")
      (key "[ u" "Go to the file not yet viewed before" "select_prev_unviewed_entry")
      (key "Ctrl+C" "Close the review" "close_review_tab")
    ];
    submitting = [
      (key "Space g h v S" "Submit the review" "submit_review")
      (key "Space g h v d" "Throw the review away" "discard_review")
    ];
    panel = [
      (key "Space g h v e" "Go to the changed files" "focus_files")
      (key "Space g h v b" "Show the changed files, or hide them" "toggle_files")
      (key "Space g h v t" "Mark the file viewed, or not" "toggle_viewed")
      (key "Space g h v c" "Review one of the pull request's commits" "review_commits")
    ];
  };
in
[
  # ─── What every layer shares ────────────────────────────────────────────
  {
    chord = "Space Space";
    does = "Find anything";
    lua = "require('fzf-lua').global()";
  }
  {
    chord = "Space ?";
    does = "Show every key here";
    lua = "require('which-key').show()";
  }
  {
    chord = "Space w";
    does = "Close the split, or the file when it's the last";
    action = "split.close";
  }
  {
    chord = "Space Tab";
    does = "Go back to the file used last";
    action = "file.last";
  }
  {
    chord = "Space Return";
    does = "Open a terminal over the editor, or hide it";
    lua = "Snacks.terminal()";
  }
]
++ map (way: {
  chord = "Space ${way}";
  does = "Go to the split ${ways.${way}}";
  keys = "<C-w>${wincmd.${way}}";
}) (builtins.attrNames ways)
++ map (way: {
  chord = "Space Shift+${way}";
  does = "Move the split to the ${edges.${way}}";
  keys = "<C-w>${builtins.replaceStrings [ "h" "j" "k" "l" ] [ "H" "J" "K" "L" ] wincmd.${way}}";
}) (builtins.attrNames ways)
++
  map
    (number: {
      chord = "Space ${toString number}";
      does = "Go to tab ${toString number}";
      lua = "pcall(vim.cmd.tabnext, ${toString number})";
    })
    [
      1
      2
      3
      4
      5
      6
      7
      8
      9
    ]
++ [
  {
    chord = "Space /";
    does = "Find a line in this file";
    lua = "require('fzf-lua').blines()";
  }
  {
    chord = "Space .";
    does = "Find again, where the last search left off";
    lua = "require('fzf-lua').resume()";
  }
  {
    chord = "Space Q";
    does = "Quit Neovim, asking about unsaved files";
    command = "confirm qall";
  }
  {
    chord = "Space U";
    does = "Show the undo history as a tree, or hide it";
    command = "UndotreeToggle";
  }

  # ─── b: the open files ──────────────────────────────────────────────────
  {
    chord = "Space b Space";
    does = "Find an open file";
    lua = "require('fzf-lua').buffers()";
  }
  {
    chord = "Space b w";
    does = "Close the file, keeping the split";
    lua = "Snacks.bufdelete()";
  }
  {
    chord = "Space b W";
    does = "Close the file, even unsaved";
    lua = "Snacks.bufdelete({ force = true })";
  }
  {
    chord = "Space b o";
    does = "Close every other file";
    lua = "Snacks.bufdelete.other()";
  }
  {
    chord = "Space b s";
    does = "Save the file";
    command = "write";
  }
  {
    chord = "Space b Return";
    does = "Open a scratch file";
    lua = "Snacks.scratch()";
  }
  {
    chord = "Space b S";
    does = "Find a scratch file";
    lua = "Snacks.scratch.select()";
  }

  # ─── c: code ────────────────────────────────────────────────────────────
  {
    chord = "Space c Space";
    does = "Find a symbol in this file";
    lua = "require('fzf-lua').lsp_document_symbols()";
  }
  {
    chord = "Space c u";
    does = "Find where this is defined and used";
    lua = "require('fzf-lua').lsp_finder()";
  }
  {
    chord = "Space c a";
    does = "Choose a fix or a change for the code here";
    lua = "require('fzf-lua').lsp_code_actions()";
    modes = both;
    scope = "code";
  }
  {
    chord = "Space c R";
    does = "Rename this everywhere";
    lua = "vim.lsp.buf.rename()";
    scope = "code";
  }
  {
    chord = "Space c h";
    does = "Show what this is";
    lua = "vim.lsp.buf.hover({ border = 'rounded', max_width = 80 })";
    scope = "code";
  }
  {
    chord = "Space c H";
    does = "Show what this call takes";
    lua = "vim.lsp.buf.signature_help({ border = 'rounded', max_width = 80 })";
    scope = "code";
  }
  {
    chord = "Space c f";
    does = "Format the file, or the selection";
    lua = "require('conform').format({ async = true, lsp_fallback = true })";
    modes = both;
  }
  {
    chord = "Space c F";
    does = "Format the code inside other code, as in Markdown";
    lua = "require('conform').format({ formatters = { 'injected' }, timeout_ms = 3000 })";
    modes = both;
  }
  {
    chord = "Space c n";
    does = "Line up text, over a motion";
    keys = "<Plug>(EasyAlign)";
    remap = true;
  }
  {
    chord = "Space c n";
    does = "Line up the selected text";
    keys = "<Plug>(EasyAlign)";
    remap = true;
    modes = selected;
  }
  {
    chord = "Space c c Space";
    does = "Find a key for calls, by what it does";
    lua = "require('keys').find('Space c c', 'calls')";
  }
  {
    chord = "Space c c i";
    does = "Find what calls this";
    lua = "require('fzf-lua').lsp_incoming_calls()";
    scope = "code";
  }
  {
    chord = "Space c c o";
    does = "Find what this calls";
    lua = "require('fzf-lua').lsp_outgoing_calls()";
    scope = "code";
  }
  {
    chord = "Space c l Space";
    does = "Find a key for code lenses, by what it does";
    lua = "require('keys').find('Space c l', 'lens')";
  }
  {
    chord = "Space c l r";
    does = "Run the code lens here";
    lua = "vim.lsp.codelens.run()";
    scope = "code";
  }
  {
    chord = "Space c l f";
    does = "Refresh the code lenses";
    lua = "vim.lsp.codelens.refresh()";
    scope = "code";
  }
  {
    chord = "Space c t Space";
    does = "Find a key for what code shows, by what it does";
    lua = "require('keys').find('Space c t', 'toggles')";
  }
  {
    chord = "Space c t i";
    does = "Show the types the code leaves out, or hide them";
    action = "code.inlayHints";
    scope = "code";
  }
  {
    chord = "Space c o Space";
    does = "Find a key for the outline, by what it does";
    lua = "require('keys').find('Space c o', 'outline')";
  }
  {
    chord = "Space c o o";
    does = "Show the outline, or hide it";
    command = "AerialToggle";
  }
  {
    chord = "Space c o O";
    does = "Go to the outline";
    command = "AerialOpen";
  }
  {
    chord = "Space c o n";
    does = "Go to the next symbol in the outline";
    command = "AerialNext";
  }
  {
    chord = "Space c o p";
    does = "Go to the symbol before in the outline";
    command = "AerialPrev";
  }
  {
    chord = "Space c o N";
    does = "Walk the outline's symbols";
    command = "AerialNavToggle";
  }
  {
    chord = "Space c r Space";
    does = "Find a key for refactoring, by what it does";
    lua = "require('keys').find('Space c r', 'refactor')";
  }
  {
    chord = "Space c r e";
    does = "Make the selection a function";
    lua = "require('refactoring').refactor('Extract Function')";
    modes = selected;
  }
  {
    chord = "Space c r f";
    does = "Make the selection a function in a file of its own";
    lua = "require('refactoring').refactor('Extract Function To File')";
    modes = selected;
  }
  {
    chord = "Space c r v";
    does = "Make the selection a variable";
    lua = "require('refactoring').refactor('Extract Variable')";
    modes = selected;
  }
  {
    chord = "Space c r i";
    does = "Put a variable's value where it's used";
    lua = "require('refactoring').refactor('Inline Variable')";
    modes = both;
  }
  {
    chord = "Space c r b";
    does = "Make the selection a block";
    lua = "require('refactoring').refactor('Extract Block')";
    modes = selected;
  }
  {
    chord = "Space c r B";
    does = "Move this block to a file of its own";
    lua = "require('refactoring').refactor('Extract Block To File')";
  }
  {
    chord = "Space c s Space";
    does = "Find a key for selecting code, by what it does";
    lua = "require('keys').find('Space c s', 'selection')";
  }
  {
    chord = "Space c s s";
    does = "Select the piece of code here";
    keys = "van";
    remap = true;
  }
  {
    chord = "Space c s i";
    does = "Select the code around the selection";
    keys = "an";
    remap = true;
    modes = selected;
  }
  {
    chord = "Space c s d";
    does = "Select less of the code";
    keys = "in";
    remap = true;
    modes = selected;
  }

  # ─── e: errors ──────────────────────────────────────────────────────────
  {
    chord = "Space e Space";
    does = "Find a problem in this file";
    lua = "require('fzf-lua').diagnostics_document()";
  }
  {
    chord = "Space e E";
    does = "Find a problem in any file";
    lua = "require('fzf-lua').diagnostics_workspace()";
  }
  {
    chord = "Space e h";
    does = "Show the problem here in full";
    lua = "vim.diagnostic.open_float({ border = 'rounded' })";
  }
  {
    chord = "Space e q";
    does = "Show the quickfix list as Trouble's, or hide it";
    lua = "require('trouble').toggle('quickfix')";
  }
  {
    chord = "Space e t Space";
    does = "Find a key for how problems show, by what it does";
    lua = "require('keys').find('Space e t', 'toggles')";
  }
  {
    chord = "Space e t d";
    does = "Show problems, or hide them";
    action = "errors.toggle";
  }
  {
    chord = "Space e t v";
    does = "Show problems beside their lines, or not";
    action = "errors.toggleBeside";
  }
  {
    chord = "Space e t s";
    does = "Show problems' signs in the margin, or not";
    action = "errors.toggleSigns";
  }

  # ─── f: files ───────────────────────────────────────────────────────────
  {
    chord = "Space f Space";
    does = "Find a file";
    lua = "require('fzf-lua').files()";
  }
  {
    chord = "Space f o";
    does = "Find a file anywhere, past this folder";
    lua = "require('fzf-lua').files({ cwd_only = false })";
  }
  {
    chord = "Space f r";
    does = "Find a file opened lately";
    lua = "require('fzf-lua').oldfiles()";
  }
  {
    chord = "Space f g";
    does = "Find a file git tracks";
    lua = "require('fzf-lua').git_files()";
  }
  {
    chord = "Space f e";
    does = "Show the folder as a file you can edit";
    command = "Oil";
  }
  {
    chord = "-";
    does = "Go up to the folder holding this";
    command = "Oil";
  }

  # ─── g: git ─────────────────────────────────────────────────────────────
  {
    chord = "Space g Space";
    does = "Find a key for git, by what it does";
    lua = "require('keys').find('Space g', 'git')";
  }
  {
    chord = "Space g g";
    does = "Show what's changed, file by file";
    lua = "require('fzf-lua').git_status()";
  }
  {
    chord = "Space g d";
    does = "Find a file changed since the last commit";
    lua = "require('fzf-lua').git_diff({ ref = 'HEAD' })";
  }
  {
    chord = "Space g a Space";
    does = "Find a stash";
    lua = "require('fzf-lua').git_stash()";
  }
  {
    chord = "Space g a s";
    does = "Stash the changes";
    command = "Git stash";
  }
  {
    chord = "Space g a p";
    does = "Take back the last stash";
    command = "Git stash pop";
  }
  {
    chord = "Space g a l";
    does = "Apply the last stash, keeping it";
    command = "Git stash apply";
  }
  {
    chord = "Space g a d";
    does = "Throw away the last stash";
    command = "Git stash drop";
  }
  {
    chord = "Space g b Space";
    does = "Find a branch";
    lua = "require('fzf-lua').git_branches()";
  }
  {
    chord = "Space g b -";
    does = "Go back to the branch before";
    command = "Git checkout -";
  }
  {
    chord = "Space g b Return";
    does = "Start a branch";
    keys = ":Git checkout -b ";
  }
  {
    chord = "Space g b w";
    does = "Delete a branch";
    keys = ":Git branch -d ";
  }
  {
    chord = "Space g b W";
    does = "Delete a branch, even unmerged";
    keys = ":Git branch -D ";
  }
  {
    chord = "Space g c Space";
    does = "Find a commit";
    lua = "require('fzf-lua').git_commits()";
  }
  {
    chord = "Space g c f";
    does = "Find a commit to this file";
    lua = "require('fzf-lua').git_bcommits()";
  }
  {
    chord = "Space g c g";
    does = "Show the commits as a graph";
    command = "Git log --graph --oneline";
  }
  {
    chord = "Space g c Return";
    does = "Commit";
    command = "Git commit";
  }
  {
    chord = "Space g c a";
    does = "Add to the last commit, rewording it";
    command = "Git commit --amend";
  }
  {
    chord = "Space g c !";
    does = "Add to the last commit as it's worded";
    command = "Git commit --amend --no-edit";
  }
  {
    chord = "Space g c b";
    does = "Show who wrote this line, and why";
    lua = "require('gitsigns').blame_line({ full = true })";
    scope = "git";
  }
  {
    chord = "Space g c t Space";
    does = "Find a key for what commits show, by what it does";
    lua = "require('keys').find('Space g c t', 'toggles')";
  }
  {
    chord = "Space g c t b";
    does = "Show who wrote each line as you go, or stop";
    lua = "require('gitsigns').toggle_current_line_blame()";
    scope = "git";
  }
  {
    chord = "Space g t Space";
    does = "Find a tag";
    lua = "require('fzf-lua').git_tags()";
  }
  {
    chord = "Space g t Return";
    does = "Tag this commit";
    action = "git.tag";
  }
  {
    chord = "Space g t w";
    does = "Delete a tag";
    action = "git.untag";
  }
  {
    chord = "Space g w Space";
    does = "Find a change not yet staged";
    lua = "require('fzf-lua').git_hunks()";
  }
  {
    chord = "Space g w b";
    does = "Find who wrote each line of the file";
    lua = "require('fzf-lua').git_blame()";
  }
  {
    chord = "Space g w h";
    does = "Show the change here";
    lua = "require('gitsigns').preview_hunk_inline()";
    scope = "git";
  }
  {
    chord = "Space g w v";
    does = "Show the file beside what's staged";
    lua = "require('gitsigns').diffthis()";
    scope = "git";
  }
  {
    chord = "Space g w H";
    does = "Show the file beside the last commit";
    lua = "require('gitsigns').diffthis('~1')";
    scope = "git";
  }
  {
    chord = "Space g w d";
    does = "Throw away the change here";
    lua = "require('gitsigns').reset_hunk()";
    scope = "git";
  }
  {
    chord = "Space g w d";
    does = "Throw away the selected changes";
    lua = "require('gitsigns').reset_hunk({ vim.fn.line('.'), vim.fn.line('v') })";
    modes = selected;
    scope = "git";
  }
  {
    chord = "Space g w D";
    does = "Throw away every change to the file";
    lua = "require('gitsigns').reset_buffer()";
    scope = "git";
  }
  {
    chord = "Space g s Space";
    does = "Find a key for staging, by what it does";
    lua = "require('keys').find('Space g s', 'stage')";
  }
  {
    chord = "Space g s a";
    does = "Stage the change here";
    lua = "require('gitsigns').stage_hunk()";
    scope = "git";
  }
  {
    chord = "Space g s a";
    does = "Stage the selected changes";
    lua = "require('gitsigns').stage_hunk({ vim.fn.line('.'), vim.fn.line('v') })";
    modes = selected;
    scope = "git";
  }
  {
    chord = "Space g s A";
    does = "Stage the whole file";
    lua = "require('gitsigns').stage_buffer()";
    scope = "git";
  }
  {
    chord = "Space g s u";
    does = "Unstage the change here";
    lua = "require('gitsigns').reset_hunk({ staged = true })";
    scope = "git";
  }
  {
    chord = "Space g s v";
    does = "Show what's staged beside the last commit";
    lua = "require('gitsigns').diffthis('--staged')";
    scope = "git";
  }
  {
    chord = "Space g n Space";
    does = "Find a key for the remote, by what it does";
    lua = "require('keys').find('Space g n', 'remote')";
  }
  {
    chord = "Space g n p";
    does = "Push";
    command = "Git push";
  }
  {
    chord = "Space g n l";
    does = "Pull";
    command = "Git pull";
  }
  {
    chord = "Space g n f";
    does = "Fetch";
    command = "Git fetch";
  }

  # ─── g h: GitHub ────────────────────────────────────────────────────────
  {
    chord = "Space g h Space";
    does = "Find a key for GitHub, by what it does";
    lua = "require('keys').find('Space g h', 'GitHub')";
  }
]
# Space in each group of octo's own buffers' keys, which finds the group's
# keys that work in the buffer here.
++
  map
    (group: {
      chord = "Space g h ${group.letters} Space";
      does = "Find a key for ${group.words}, by what it does";
      lua = "require('keys').find('Space g h ${group.letters}', '${group.name}')";
    })
    [
      {
        letters = "a";
        words = "assignees and reviewers";
        name = "people";
      }
      {
        letters = "b";
        words = "the issue or pull request";
        name = "this issue or pull request";
      }
      {
        letters = "c";
        words = "comments";
        name = "comments";
      }
      {
        letters = "k";
        words = "checks";
        name = "checks";
      }
      {
        letters = "l";
        words = "labels";
        name = "labels";
      }
      {
        letters = "m";
        words = "merging";
        name = "merge";
      }
      {
        letters = "m q";
        words = "merging through the queue";
        name = "through the queue";
      }
      {
        letters = "t";
        words = "reactions";
        name = "reactions";
      }
      {
        letters = "T";
        words = "threads";
        name = "threads";
      }
      {
        letters = "v";
        words = "reviewing";
        name = "review";
      }
    ]
++ [
  {
    chord = "Space g h h";
    does = "Show this repository's pull requests";
    command = "Octo pr list";
  }
  {
    chord = "Space g h s";
    does = "Search GitHub";
    command = "Octo search";
  }
  {
    chord = "Space g h p Space";
    does = "Find a pull request";
    command = "Octo pr list";
  }
  {
    chord = "Space g h p Return";
    does = "Open a pull request";
    command = "Octo pr create";
  }
  {
    chord = "Space g h p c";
    does = "Find one of your pull requests";
    command = "Octo pr list author:@me";
  }
  {
    chord = "Space g h p s";
    does = "Search pull requests";
    command = "Octo pr search";
  }
  {
    chord = "Space g h p C";
    does = "Show this pull request's checks";
    command = "Octo pr checks";
  }
  {
    chord = "Space g h p y";
    does = "Copy this branch's pull request's link";
    action = "github.copyPullRequest";
  }
  {
    chord = "Space g h p o";
    does = "Open this branch's pull request in the browser";
    action = "github.openPullRequest";
  }
  {
    chord = "Space g h i Space";
    does = "Find an issue";
    command = "Octo issue list";
  }
  {
    chord = "Space g h i Return";
    does = "Open an issue";
    command = "Octo issue create";
  }
  {
    chord = "Space g h i s";
    does = "Search issues";
    command = "Octo issue search";
  }
  {
    chord = "Space g h r Space";
    does = "Find a repository";
    command = "Octo repo list";
  }
  {
    chord = "Space g h r f";
    does = "Fork this repository";
    command = "Octo repo fork";
  }
  {
    chord = "Space g h r o";
    does = "Open this repository in the browser";
    command = "Octo repo browser";
  }
  {
    chord = "Space g h r y";
    does = "Copy this repository's link";
    action = "github.copyRepository";
  }
  {
    chord = "Space g h g Space";
    does = "Find a gist";
    command = "Octo gist list";
  }
  {
    chord = "Space g h g Return";
    does = "Make the file a gist";
    action = "github.gistFile";
  }
  {
    chord = "Space g h g Return";
    does = "Make the selection a gist";
    # Through the command line, which takes the selection's lines along.
    keys = ":write !gh gist create<CR>";
    modes = selected;
  }
  {
    chord = "Space g h n Space";
    does = "Find a GitHub notification";
    command = "Octo notification list";
  }
  {
    chord = "Space g h f Space";
    does = "Find a key for links to the file, by what it does";
    lua = "require('keys').find('Space g h f', 'file links')";
  }
  {
    chord = "Space g h f y";
    does = "Copy a lasting link to these lines";
    keys = ":GBrowse!<CR>";
    modes = both;
  }
  {
    chord = "Space g h f o";
    does = "Open these lines in the browser";
    keys = ":GBrowse<CR>";
    modes = both;
  }

  # ─── i: what Neovim holds ───────────────────────────────────────────────
  {
    chord = "Space i Space";
    does = "Find a key for Neovim's insides, by what it does";
    lua = "require('keys').find('Space i', 'insides')";
  }
  {
    chord = "Space i k";
    does = "Find a key, with what it runs";
    lua = "require('fzf-lua').keymaps()";
  }
  {
    chord = "Space i c";
    does = "Find a command";
    lua = "require('fzf-lua').commands()";
  }
  {
    chord = "Space i o";
    does = "Find a setting";
    lua = "require('fzf-lua').nvim_options()";
  }
  {
    chord = "Space i h";
    does = "Find a command run before";
    lua = "require('fzf-lua').command_history()";
  }
  {
    chord = "Space i r";
    does = "Find what's in a register";
    lua = "require('fzf-lua').registers()";
  }
  {
    chord = "Space i m";
    does = "Find a mark";
    lua = "require('fzf-lua').marks()";
  }
  {
    chord = "Space i j";
    does = "Find a place jumped from";
    lua = "require('fzf-lua').jumps()";
  }
  {
    chord = "Space i C";
    does = "Find a change made";
    lua = "require('fzf-lua').changes()";
  }
  {
    chord = "Space i a";
    does = "Find an autocommand";
    lua = "require('fzf-lua').autocmds()";
  }
  {
    chord = "Space i x";
    does = "Choose a spelling for the word here";
    lua = "require('fzf-lua').spell_suggest()";
  }
  {
    chord = "Space i n";
    does = "Show the notifications so far";
    lua = "Snacks.notifier.show_history()";
  }
  {
    chord = "Space i N";
    does = "Put the notifications away";
    lua = "Snacks.notifier.hide()";
  }
  {
    chord = "Space i l";
    does = "Show the language servers";
    # Neovim's own report; lspconfig's LspInfo, an alias for it, is gone
    # since Neovim has a command of its own.
    command = "checkhealth vim.lsp";
  }

  # ─── k: the language server's folders ───────────────────────────────────
  {
    chord = "Space k Space";
    does = "Find a key for the language server's folders, by what it does";
    lua = "require('keys').find('Space k', 'language server folders')";
  }
  {
    chord = "Space k a";
    does = "Give the language server another folder";
    lua = "vim.lsp.buf.add_workspace_folder()";
    scope = "code";
  }
  {
    chord = "Space k r";
    does = "Take a folder from the language server";
    lua = "vim.lsp.buf.remove_workspace_folder()";
    scope = "code";
  }
  {
    chord = "Space k l";
    does = "Show the language server's folders";
    action = "code.folders";
    scope = "code";
  }

  # ─── s: search ──────────────────────────────────────────────────────────
  {
    chord = "Space s Space";
    does = "Find a key for searching, by what it does";
    lua = "require('keys').find('Space s', 'search')";
  }
  {
    chord = "Space s p";
    does = "Search every file";
    lua = "require('fzf-lua').live_grep()";
  }
  {
    chord = "Space s w";
    does = "Search every file for the word here";
    lua = "require('fzf-lua').grep_cword()";
  }
  {
    chord = "Space s v";
    does = "Search every file for the selection";
    lua = "require('fzf-lua').grep_visual()";
    modes = selected;
  }
  {
    chord = "Space s b";
    does = "Find a line in the open files";
    lua = "require('fzf-lua').lines()";
  }
  {
    chord = "Space s y";
    does = "Find a symbol in any file";
    lua = "require('fzf-lua').lsp_workspace_symbols()";
  }
  {
    chord = "Space s s";
    does = "Find a symbol, without a language server";
    lua = "require('fzf-lua').treesitter()";
  }
  {
    chord = "Space s t";
    does = "Find every to-do and note";
    command = "TodoQuickFix";
  }
  {
    chord = "Space s h";
    does = "Find a search made before";
    lua = "require('fzf-lua').search_history()";
  }
  {
    chord = "Space s H";
    does = "Find in Neovim's help";
    lua = "require('fzf-lua').helptags()";
  }
  {
    chord = "Space s q";
    does = "Find in the quickfix list";
    lua = "require('fzf-lua').quickfix()";
  }
  {
    chord = "Space s Q";
    does = "Find in the location list";
    lua = "require('fzf-lua').loclist()";
  }
  {
    chord = "Space s c";
    does = "Stop marking what was searched for";
    command = "nohlsearch";
  }
  {
    chord = "Space s r";
    does = "Replace one thing with another, through the file";
    action = "search.replace";
  }

  # ─── t: tests ───────────────────────────────────────────────────────────
  {
    chord = "Space t Space";
    does = "Find a key for tests, by what it does";
    lua = "require('keys').find('Space t', 'tests')";
  }
  {
    chord = "Space t t";
    does = "Test the nearest";
    lua = "require('neotest').run.run()";
  }
  {
    chord = "Space t f";
    does = "Test the file";
    lua = "require('neotest').run.run(vim.fn.expand('%'))";
  }
  {
    chord = "Space t s";
    does = "Test everything";
    lua = "require('neotest').run.run(vim.fn.getcwd())";
  }
  {
    chord = "Space t l";
    does = "Test again what was tested last";
    lua = "require('neotest').run.run_last()";
  }
  {
    chord = "Space t x";
    does = "Stop testing";
    lua = "require('neotest').run.stop()";
  }
  {
    chord = "Space t o";
    does = "Show what the test here said";
    lua = "require('neotest').output.open({ enter = true, auto_close = true })";
  }
  {
    chord = "Space t O";
    does = "Show what every test said";
    lua = "require('neotest').output_panel.open()";
  }
  {
    chord = "Space t S";
    does = "Show every test, and how it went";
    lua = "require('neotest').summary.open()";
  }
  {
    chord = "Space t w";
    does = "Test the file whenever it changes, or stop";
    lua = "require('neotest').watch.toggle(vim.fn.expand('%'))";
  }
  {
    chord = "Space t W";
    does = "Test everything whenever it changes, or stop";
    lua = "require('neotest').watch.toggle(vim.fn.getcwd())";
  }

  # ─── T: tabs ────────────────────────────────────────────────────────────
  {
    chord = "Space T Space";
    does = "Find a tab";
    action = "tab.find";
  }
  {
    chord = "Space T Return";
    does = "Open a tab";
    command = "tabnew";
  }
  {
    chord = "Space T w";
    does = "Close the tab";
    command = "confirm tabclose";
  }
  {
    chord = "Space T o";
    does = "Close every other tab";
    command = "confirm tabonly";
  }
  {
    chord = "Space T f";
    does = "Go to the first tab";
    command = "tabfirst";
  }
  {
    chord = "Space T l";
    does = "Go to the last tab";
    command = "tablast";
  }

  # ─── u: how it looks ────────────────────────────────────────────────────
  {
    chord = "Space u Space";
    does = "Find a key for how it looks, by what it does";
    lua = "require('keys').find('Space u', 'looks')";
  }
  {
    chord = "Space u b";
    does = "Switch between light and dark";
    action = "look.background";
  }
  {
    chord = "Space u c";
    does = "Choose a colour scheme";
    lua = "require('fzf-lua').colorschemes()";
  }
  {
    chord = "Space u l";
    does = "Number lines from here, from the top, or not at all";
    action = "look.numbers";
  }
  {
    chord = "Space u t";
    does = "Dim all but the code here, or stop";
    command = "Twilight";
  }
  {
    chord = "Space u w";
    does = "Wrap long lines, or not";
    action = "look.wrap";
  }

  # ─── Steps through what's in order, [ before and ] after ────────────────
  {
    chord = "[ b";
    does = "Go to the open file before";
    command = "bprevious";
  }
  {
    chord = "] b";
    does = "Go to the next open file";
    command = "bnext";
  }
  {
    chord = "[ T";
    does = "Go to the tab before";
    command = "tabprevious";
  }
  {
    chord = "] T";
    does = "Go to the next tab";
    command = "tabnext";
  }
  {
    chord = "[ w";
    does = "Go to the split before";
    keys = "<C-w>W";
  }
  {
    chord = "] w";
    does = "Go to the next split";
    keys = "<C-w>w";
  }
  {
    chord = "[ q";
    does = "Go to the quickfix entry before";
    lua = "pcall(vim.cmd.cprevious)";
  }
  {
    chord = "] q";
    does = "Go to the next quickfix entry";
    lua = "pcall(vim.cmd.cnext)";
  }
  {
    chord = "[ l";
    does = "Go to the location before";
    lua = "pcall(vim.cmd.lprevious)";
  }
  {
    chord = "] l";
    does = "Go to the next location";
    lua = "pcall(vim.cmd.lnext)";
  }
  {
    chord = "[ d";
    does = "Go to the problem before";
    action = "errors.before";
  }
  {
    chord = "] d";
    does = "Go to the next problem";
    action = "errors.after";
  }
  {
    chord = "[ D";
    does = "Go to the first problem";
    action = "errors.first";
  }
  {
    chord = "] D";
    does = "Go to the last problem";
    action = "errors.last";
  }
  {
    chord = "[ g";
    does = "Go to the change before";
    lua = "if vim.wo.diff then vim.cmd.normal({ '[c', bang = true }) else require('gitsigns').nav_hunk('prev') end";
    scope = "git";
  }
  {
    chord = "] g";
    does = "Go to the next change";
    lua = "if vim.wo.diff then vim.cmd.normal({ ']c', bang = true }) else require('gitsigns').nav_hunk('next') end";
    scope = "git";
  }
  {
    chord = "[ t";
    does = "Go to the test before";
    lua = "require('neotest').jump.prev()";
  }
  {
    chord = "] t";
    does = "Go to the next test";
    lua = "require('neotest').jump.next()";
  }
  {
    chord = "[ n";
    does = "Go to the to-do or note before";
    lua = "require('todo-comments').jump_prev()";
  }
  {
    chord = "] n";
    does = "Go to the next to-do or note";
    lua = "require('todo-comments').jump_next()";
  }
  {
    chord = "[ s";
    does = "Go to the symbol before";
    command = "AerialPrev";
  }
  {
    chord = "] s";
    does = "Go to the next symbol";
    command = "AerialNext";
  }
]
++
  map
    (step: {
      inherit (step) chord does;
      lua = "require('nvim-treesitter-textobjects.move').${step.move}('${step.capture}', 'textobjects')";
      modes = moving;
    })
    [
      {
        chord = "[ f";
        does = "Go to the start of the function before";
        move = "goto_previous_start";
        capture = "@function.outer";
      }
      {
        chord = "] f";
        does = "Go to the start of the next function";
        move = "goto_next_start";
        capture = "@function.outer";
      }
      {
        chord = "[ F";
        does = "Go to the end of the function before";
        move = "goto_previous_end";
        capture = "@function.outer";
      }
      {
        chord = "] F";
        does = "Go to the end of the next function";
        move = "goto_next_end";
        capture = "@function.outer";
      }
      {
        chord = "[ c";
        does = "Go to the class before";
        move = "goto_previous_start";
        capture = "@class.outer";
      }
      {
        chord = "] c";
        does = "Go to the next class";
        move = "goto_next_start";
        capture = "@class.outer";
      }
      {
        chord = "[ C";
        does = "Go to the comment before";
        move = "goto_previous_start";
        capture = "@comment.outer";
      }
      {
        chord = "] C";
        does = "Go to the next comment";
        move = "goto_next_start";
        capture = "@comment.outer";
      }
    ]
++
  map
    (object: {
      inherit (object) chord does;
      lua = "require('nvim-treesitter-textobjects.select').select_textobject('${object.capture}', 'textobjects')";
      modes = around;
    })
    [
      {
        chord = "a f";
        does = "The function, all of it";
        capture = "@function.outer";
      }
      {
        chord = "i f";
        does = "The function's body";
        capture = "@function.inner";
      }
      {
        chord = "a c";
        does = "The class, all of it";
        capture = "@class.outer";
      }
      {
        chord = "i c";
        does = "The class's body";
        capture = "@class.inner";
      }
      {
        chord = "a l";
        does = "The loop, all of it";
        capture = "@loop.outer";
      }
      {
        chord = "i l";
        does = "The loop's body";
        capture = "@loop.inner";
      }
      {
        chord = "a a";
        does = "The argument, with its comma";
        capture = "@parameter.outer";
      }
      {
        chord = "i a";
        does = "The argument";
        capture = "@parameter.inner";
      }
    ]
++ [
  {
    chord = "i h";
    does = "The change here";
    command = "Gitsigns select_hunk";
    modes = around;
    scope = "git";
  }

  # ─── Going straight somewhere, g ────────────────────────────────────────
  {
    chord = "g d";
    does = "Go to where this is defined";
    lua = "require('fzf-lua').lsp_definitions()";
    scope = "code";
  }
  {
    chord = "g D";
    does = "Go to where this is declared";
    lua = "require('fzf-lua').lsp_declarations()";
    scope = "code";
  }
  {
    chord = "g i";
    does = "Go to what implements this";
    lua = "require('fzf-lua').lsp_implementations()";
    scope = "code";
  }
  {
    chord = "g y";
    does = "Go to this one's type";
    lua = "require('fzf-lua').lsp_typedefs()";
    scope = "code";
  }
  {
    chord = "g r";
    does = "Find where this is used";
    lua = "require('fzf-lua').lsp_references()";
    scope = "code";
  }
  {
    chord = "g s";
    does = "Jump to a place on screen, by typing it";
    lua = "require('flash').jump()";
    modes = moving;
  }
  {
    chord = "g S";
    does = "Jump to a piece of code on screen";
    lua = "require('flash').treesitter()";
    # With text selected, g S is vim-surround's, and a n and i n already
    # grow and shrink the selection by piece of code.
    modes = [
      "n"
      "o"
    ];
  }
  {
    chord = "g l";
    does = "Jump to a line on screen";
    action = "jump.line";
  }
  {
    chord = "g w";
    does = "Jump to a word on screen";
    action = "jump.word";
  }

  # ─── Plain keys ─────────────────────────────────────────────────────────
  {
    chord = "Down";
    does = "Go down a line as it's shown, wrapped or not";
    keys = "v:count == 0 ? 'gj' : 'j'";
    expr = true;
    modes = both;
  }
  {
    chord = "Up";
    does = "Go up a line as it's shown, wrapped or not";
    keys = "v:count == 0 ? 'gk' : 'k'";
    expr = true;
    modes = both;
  }
  {
    chord = "Ctrl+Q";
    does = "Show the quickfix list, or hide it";
    action = "quickfix.toggle";
  }
  {
    chord = "Shift+Escape";
    does = "Stop typing into the terminal";
    keys = "<C-\\><C-n>";
    modes = [ "t" ];
  }
  {
    chord = "Ctrl+X Ctrl+F";
    does = "Find a file's name to type";
    lua = "require('fzf-lua').complete_file({ winopts = { preview = { hidden = true } } })";
    modes = [ "i" ];
  }
  {
    chord = "Ctrl+X Ctrl+P";
    does = "Find a path to type";
    lua = "require('fzf-lua').complete_path()";
    modes = [ "i" ];
  }
  {
    chord = "Ctrl+X Ctrl+L";
    does = "Find a line to type again";
    lua = "require('fzf-lua').complete_line()";
    modes = [ "i" ];
  }

  # ─── In git's status, fugitive's window ─────────────────────────────────
  {
    chord = "c c";
    does = "Commit";
    command = "Git commit";
    scope = "status";
  }
  {
    chord = "c a";
    does = "Add to the last commit, rewording it";
    command = "Git commit --amend";
    scope = "status";
  }
  {
    chord = "c e";
    does = "Add to the last commit as it's worded";
    command = "Git commit --amend --no-edit";
    scope = "status";
  }
  {
    chord = "p p";
    does = "Push";
    command = "Git push";
    scope = "status";
  }
  {
    chord = "p f";
    does = "Push over what the remote has, if it's what was fetched";
    command = "Git push --force-with-lease";
    scope = "status";
  }
  {
    chord = "p u";
    does = "Push a new branch";
    command = "Git push -u origin HEAD";
    scope = "status";
  }
  {
    chord = "r b";
    does = "Rebase onto a commit, choosing what to keep";
    keys = ":Git rebase -i ";
    scope = "status";
  }
  {
    chord = "r i";
    does = "Rebase the last few commits, choosing what to keep";
    keys = ":Git rebase -i HEAD~";
    scope = "status";
  }
]

# The plugins' own windows, which bind their keys themselves: each key names
# the plugin's action, and its file hands the plugin these and no others.
# They share words with the pickers: Return opens, Ctrl+S below, Ctrl+V
# beside, Ctrl+T in a tab, q closes, ? shows the window's keys, Ctrl+L reads
# it again, and a float closes on Escape too. Neovim's own z keys fold.
++ inWindow "outline" [
  {
    chord = "?";
    does = "Show the outline's keys";
    plugin = "actions.show_help";
  }
  {
    chord = "Return";
    does = "Go to the symbol";
    plugin = "actions.jump";
  }
  {
    chord = "Ctrl+S";
    does = "Open the symbol below";
    plugin = "actions.jump_split";
  }
  {
    chord = "Ctrl+V";
    does = "Open the symbol beside";
    plugin = "actions.jump_vsplit";
  }
  {
    chord = "p";
    does = "Show the symbol, staying in the outline";
    plugin = "actions.scroll";
  }
  {
    chord = "Ctrl+J";
    does = "Show the next symbol, staying in the outline";
    plugin = "actions.down_and_scroll";
  }
  {
    chord = "Ctrl+K";
    does = "Show the symbol before, staying in the outline";
    plugin = "actions.up_and_scroll";
  }
  {
    chord = "{";
    does = "Go to the symbol before";
    plugin = "actions.prev";
  }
  {
    chord = "}";
    does = "Go to the next symbol";
    plugin = "actions.next";
  }
  {
    chord = "[ [";
    does = "Go to the symbol before, a level up";
    plugin = "actions.prev_up";
  }
  {
    chord = "] ]";
    does = "Go to the next symbol, a level up";
    plugin = "actions.next_up";
  }
  {
    chord = "q";
    does = "Close the outline";
    plugin = "actions.close";
  }
  {
    chord = "l";
    does = "Show what the symbol holds";
    plugin = "actions.tree_open";
  }
  {
    chord = "L";
    does = "Show everything the symbol holds";
    plugin = "actions.tree_open_recursive";
  }
  {
    chord = "h";
    does = "Fold the symbol";
    plugin = "actions.tree_close";
  }
  {
    chord = "H";
    does = "Fold the symbol and everything in it";
    plugin = "actions.tree_close_recursive";
  }
]
++ inWindow "outline" (folds outlineFolds)
++ inWindow "walk" [
  {
    chord = "Return";
    does = "Go to the symbol";
    plugin = "actions.jump";
  }
  {
    chord = "Ctrl+S";
    does = "Open the symbol below";
    plugin = "actions.jump_split";
  }
  {
    chord = "Ctrl+V";
    does = "Open the symbol beside";
    plugin = "actions.jump_vsplit";
  }
  {
    chord = "h";
    does = "Go out to the symbol holding this";
    plugin = "actions.left";
  }
  {
    chord = "l";
    does = "Go into the symbol";
    plugin = "actions.right";
  }
  {
    chord = "q";
    does = "Stop walking the outline";
    plugin = "actions.close";
  }
  {
    chord = "Escape";
    does = "Stop walking the outline";
    plugin = "actions.close";
  }
]
++ inWindow "files" [
  {
    chord = "?";
    does = "Show the file browser's keys";
    plugin = "actions.show_help";
  }
  {
    chord = "Return";
    does = "Open the file or folder";
    plugin = "actions.select";
  }
  {
    chord = "Ctrl+S";
    does = "Open the file below";
    plugin = "actions.select";
    options = "{ horizontal = true }";
  }
  {
    chord = "Ctrl+V";
    does = "Open the file beside";
    plugin = "actions.select";
    options = "{ vertical = true }";
  }
  {
    chord = "Ctrl+T";
    does = "Open the file in a tab";
    plugin = "actions.select";
    options = "{ tab = true }";
  }
  {
    chord = "Ctrl+P";
    does = "Show the file beside, staying here";
    plugin = "actions.preview";
  }
  {
    chord = "q";
    does = "Close the file browser";
    plugin = "actions.close";
  }
  {
    chord = "Ctrl+L";
    does = "Read the folder again";
    plugin = "actions.refresh";
  }
  {
    chord = "-";
    does = "Go up to the folder holding this";
    plugin = "actions.parent";
  }
  {
    chord = "_";
    does = "Go to the working folder";
    plugin = "actions.open_cwd";
  }
  {
    chord = "`";
    does = "Make this the working folder";
    plugin = "actions.cd";
  }
  {
    chord = "~";
    does = "Make this the tab's working folder";
    plugin = "actions.cd";
    options = "{ scope = 'tab' }";
  }
  {
    chord = "g x";
    does = "Open the file in its own program";
    plugin = "actions.open_external";
  }
  {
    chord = "g h";
    does = "Show hidden files, or hide them";
    plugin = "actions.toggle_hidden";
  }
  {
    chord = "g \\";
    does = "Show the trash, or the folder";
    plugin = "actions.toggle_trash";
  }
]
++ inWindow "problems" [
  {
    chord = "?";
    does = "Show the problems list's keys";
    plugin = "help";
  }
  {
    chord = "Return";
    does = "Go to the problem";
    plugin = "jump";
  }
  {
    chord = "o";
    does = "Go to the problem and close the list";
    plugin = "jump_close";
  }
  {
    chord = "Ctrl+S";
    does = "Open the problem below";
    plugin = "jump_split";
  }
  {
    chord = "Ctrl+V";
    does = "Open the problem beside";
    plugin = "jump_vsplit";
  }
  {
    chord = "p";
    does = "Show the problem, staying in the list";
    plugin = "preview";
  }
  {
    chord = "P";
    does = "Show each problem as the cursor reaches it, or stop";
    plugin = "toggle_preview";
  }
  {
    chord = "Escape";
    does = "Put back what the preview replaced";
    plugin = "cancel";
  }
  {
    chord = "q";
    does = "Close the problems list";
    plugin = "close";
  }
  {
    chord = "Ctrl+L";
    does = "Read the problems again";
    plugin = "refresh";
  }
  {
    chord = "R";
    does = "Keep the list up to date, or stop";
    plugin = "toggle_refresh";
  }
  {
    chord = "{";
    does = "Go to the problem before";
    plugin = "prev";
  }
  {
    chord = "}";
    does = "Go to the next problem";
    plugin = "next";
  }
  {
    chord = "d d";
    does = "Take the problem off the list";
    plugin = "delete";
  }
  {
    chord = "d";
    does = "Take the selected problems off the list";
    plugin = "delete";
    modes = selected;
  }
  {
    chord = "i";
    does = "Show everything the list knows of the problem";
    plugin = "inspect";
  }
  {
    chord = "g b";
    does = "Show only this file's problems, or every file's";
    plugin = "only.file";
  }
  {
    chord = "s";
    does = "Show only the worse problems, a step at a time";
    plugin = "only.worse";
  }
]
++ inWindow "problems" (folds problemFolds)
++ inWindow "completion" (
  map (key: { modes = typing; } // key) [
    {
      chord = "Tab";
      does = "Choose the next suggestion, or suggest";
      plugin = "next";
      modes = [
        "i"
        "s"
      ];
    }
    {
      chord = "Shift+Tab";
      does = "Choose the suggestion before";
      plugin = "before";
      modes = [
        "i"
        "s"
      ];
    }
    {
      chord = "Ctrl+N";
      does = "Choose the next suggestion, or suggest";
      plugin = "next";
    }
    {
      chord = "Ctrl+P";
      does = "Choose the suggestion before";
      plugin = "before";
    }
    {
      chord = "Ctrl+Space";
      does = "Suggest";
      plugin = "suggest";
    }
    {
      chord = "Ctrl+Y";
      does = "Take the suggestion";
      plugin = "take";
    }
    {
      chord = "Ctrl+E";
      does = "Put the suggestions away";
      plugin = "away";
    }
    {
      chord = "Ctrl+B";
      does = "Scroll the suggestion's notes up";
      plugin = "notes.up";
    }
    {
      chord = "Ctrl+F";
      does = "Scroll the suggestion's notes down";
      plugin = "notes.down";
    }
  ]
)
# The command line's suggestions, for a command (:) and a search (/ and ?),
# in the same words. With none showing, Tab suggests, and Ctrl+N and Ctrl+P
# step through what was typed before, as they do in Vim.
++ inWindow "commands" (
  map (key: { modes = [ "c" ]; } // key) [
    {
      chord = "Tab";
      does = "Choose the next suggestion, or suggest";
      plugin = "next";
    }
    {
      chord = "Shift+Tab";
      does = "Choose the suggestion before, or suggest";
      plugin = "before";
    }
    {
      chord = "Ctrl+N";
      does = "Choose the next suggestion, or a later line from history";
      plugin = "later";
    }
    {
      chord = "Ctrl+P";
      does = "Choose the suggestion before, or an earlier line from history";
      plugin = "earlier";
    }
    {
      chord = "Ctrl+Y";
      does = "Take the suggestion";
      plugin = "take";
    }
    {
      chord = "Ctrl+E";
      does = "Put the suggestions away";
      plugin = "away";
    }
  ]
)

# octo's buffers: an issue, a pull request, a review's thread, the window a
# review is submitted from, the change a review shows and its changed files,
# and the notifications it finds, whose keys work in the finder.
++ inWindow "octo-issue" (
  octo.thing "issue" "issue_options"
  ++ octo.assignees
  ++ octo.labels
  ++ octo.comments
  ++ octo.reactions
)
++ inWindow "octo-pull" (
  octo.thing "pull request" "pr_options"
  ++ [
    (octo.key "Space g h b Y" "Copy the commit's id" "copy_sha")
    (octo.key "Space g h m o" "Check the pull request out" "checkout_pr")
    (octo.key "Space g h m m" "Merge it with a merge commit" "merge_pr")
    (octo.key "Space g h m s" "Squash it and merge" "squash_and_merge_pr")
    (octo.key "Space g h m r" "Rebase it and merge" "rebase_and_merge_pr")
    (octo.key "Space g h m k" "Merge it and every pull request below it in its stack" "merge_pr_stack")
    (octo.key "Space g h m q m" "Merge it with a merge commit, through the queue" "merge_pr_queue")
    (octo.key "Space g h m q s" "Squash it and merge, through the queue" "squash_and_merge_queue")
    (octo.key "Space g h m q r" "Rebase it and merge, through the queue" "rebase_and_merge_queue")
    (octo.key "Space g h v a" "Approve the pull request" "approve_pr")
    (octo.key "Space g h v c" "Find one of the pull request's commits" "list_commits")
    (octo.key "Space g h v f" "Find a file the pull request changes" "list_changed_files")
    (octo.key "Space g h v D" "Show everything the pull request changes" "show_pr_diff")
    (octo.key "Space g h v s" "Start a review" "review_start")
    (octo.key "Space g h v r" "Go back to your review" "review_resume")
    (octo.key "] p" "Open the pull request above it in its stack" "stack_up")
    (octo.key "[ p" "Open the pull request below it in its stack" "stack_down")
    (octo.key "Space g h k o" "Open the check here" "goto_check")
    (octo.key "Space g h k z" "Fold the checks, or unfold them" "toggle_checks")
    (octo.key "g f" "Go to the file" "goto_file")
  ]
  ++ octo.reviewers
  ++ octo.assignees
  ++ octo.labels
  ++ octo.comments
  ++ octo.threads
  ++ octo.reactions
)
++ inWindow "octo-thread" (
  [ (octo.key "Space g h c s" "Suggest a change" "add_suggestion") ]
  ++ octo.comments
  ++ octo.changes
  ++ octo.threads
  ++ octo.reactions
)
++ inWindow "octo-submit" (
  map
    (
      key:
      key
      // {
        modes = [
          "n"
          "i"
        ];
      }
    )
    [
      (octo.key "Ctrl+A" "Approve, with what's written" "approve_review")
      (octo.key "Ctrl+M" "Comment, with what's written" "comment_review")
      (octo.key "Ctrl+R" "Ask for changes, with what's written" "request_changes")
      (octo.key "Ctrl+C" "Close the review" "close_review_tab")
    ]
)
++ inWindow "octo-change" (
  map (key: key // { modes = both; }) [
    (octo.key "Space g h c Return" "Comment on the line, or the selection" "add_review_comment")
    (octo.key "Space g h c s" "Suggest a change to the line, or the selection" "add_review_suggestion")
  ]
  ++ [
    (octo.key "] C" "Go to the next thread" "next_thread")
    (octo.key "[ C" "Go to the thread before" "prev_thread")
    (octo.key "Space g h b Y" "Copy the commit's id" "copy_sha")
    (octo.key "g f" "Go to the file" "goto_file")
  ]
  ++ octo.submitting
  ++ octo.panel
  ++ octo.changes
)
++ inWindow "octo-files" (
  [
    (octo.key "j" "Go to the next changed file" "next_entry")
    (octo.key "k" "Go to the changed file before" "prev_entry")
    (octo.key "Return" "Show the changed file" "select_entry")
    (octo.key "Ctrl+L" "Read the changed files again" "refresh_files")
  ]
  ++ octo.submitting
  ++ octo.panel
  ++ octo.changes
)
++ inWindow "octo-runs" [
  (octo.key "Return" "Open the step, or close it" "expand_step")
  (octo.key "] e" "Go to the next step" "next_step")
  (octo.key "[ e" "Go to the step before" "prev_step")
  (octo.key "] j" "Go to the next job" "next_job")
  (octo.key "[ j" "Go to the job before" "prev_job")
  (octo.key "Ctrl+L" "Read the run again" "refresh")
  (octo.key "Space g h b o" "Open the run in the browser" "open_in_browser")
  (octo.key "Space g h b y" "Copy the run's link" "copy_url")
  (octo.key "Space g h k r" "Run it again" "rerun")
  (octo.key "Space g h k f" "Run its failed jobs again" "rerun_failed")
  (octo.key "Space g h k w" "Cancel the run" "cancel")
]
++ inWindow "octo-notices" [
  (octo.key "Ctrl+R" "Mark the notification read" "read")
  (octo.key "Ctrl+X" "Mark the notification done" "done")
  (octo.key "Alt+X" "Unsubscribe from what the notification is about" "unsubscribe")
]
