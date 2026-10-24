# The checks of neovim, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  holds,
  lib,
  neovimAt,
  onLinux,
  pkgs,
  workstation,
  ...
}:
onLinux {
  # Neovim starts with the configuration an account gets, in the theme's own
  # colour scheme drawn on the terminal's own background, with Tree-sitter
  # drawing a file in a language it has a parser for and its text objects
  # mapped, and nothing it loads complains.
  neovim-starts-in-the-theme =
    let
      home = account workstation;
      report = lib.concatStringsSep " .. ' ' .. " [
        "vim.g.colors_name"
        "tostring(vim.api.nvim_get_hl(0, { name = 'Normal' }).bg)"
        "tostring(vim.treesitter.highlighter.active[vim.api.nvim_get_current_buf()] ~= nil)"
        "(vim.fn.mapcheck('af', 'o') ~= '' and 'objects' or 'none')"
      ];
    in
    pkgs.runCommandLocal "neovim-starts-in-the-theme"
      {
        # The programs the account gets, git and gh among them, which
        # plugins call as they start.
        nativeBuildInputs = [ home.home.path ];
      }
      ''
        ${neovimAt home}
        echo 'return {}' > probe.lua
        nvim --headless probe.lua -c "lua io.stdout:write(${report})" -c 'qa!' > answer 2> complaints
        if [ -s complaints ]; then cat complaints; exit 1; fi
        if [ "$(cat answer)" != "jitsusama nil true objects" ]; then cat answer; exit 1; fi
        touch $out
      '';

  # Neovim starts with only what draws its first screen: with a file open,
  # no plugin that waits has loaded, nor vim.diagnostic, yet every command
  # a key runs is there, standing in. A command run, a module required and
  # an action asked for each load their plugin, the background ones load
  # once the first screen is drawn, and the Lua language server reads
  # only Neovim's own runtime.
  neovim-starts-with-only-what-it-draws =
    let
      home = account workstation;
      probe = pkgs.writeText "probe.lua" ''
        local later = require('later')
        local keys = require('keys')
        local wrong = {}
        local function say(line)
          table.insert(wrong, line)
        end
        for name in pairs(require('waiting')) do
          if later.loaded(name) then
            say(name .. ' loaded as Neovim started')
          end
        end
        -- What draws the first screen, and what those need.
        local first = {
          ['async.nvim'] = true,
          ['nvim-numbertoggle'] = true,
          ['nvim-treesitter'] = true,
          ['nvim-treesitter-grammars'] = true,
          ['nvim-treesitter-textobjects'] = true,
          ['oil.nvim'] = true,
          ['snacks.nvim'] = true,
          ['vim-nix'] = true,
          ['vim-repeat'] = true,
        }
        for _, dir in ipairs(vim.fn.globpath(vim.o.packpath, 'pack/*/start/*', false, true)) do
          if not first[vim.fs.basename(dir)] then
            say(vim.fs.basename(dir) .. ' starts with Neovim, though the first screen needs none of it')
          end
        end
        if package.loaded['vim.diagnostic'] then
          say('vim.diagnostic loaded as Neovim started')
        end
        for _, key in ipairs(keys.keys) do
          local command = key.command and key.command:match('^%S+')
          if command and vim.fn.exists(':' .. command) ~= 2 then
            say(key.lhs .. ' runs ' .. command .. ', which is not there')
          end
        end
        local windows = #vim.api.nvim_list_wins()
        vim.cmd('UndotreeShow')
        if not later.loaded('undotree') or #vim.api.nvim_list_wins() == windows then
          say('UndotreeShow did not load undotree and show it')
        end
        require('trouble')
        if not later.loaded('trouble.nvim') then
          say('requiring trouble did not load it')
        end
        -- aerial's setup requires aerial itself, so the first require has to
        -- answer with the copy that was set up; another copy has no settings
        -- and fails when asked for an outline. This file has no symbols, so
        -- the outline it gives is empty.
        local opened, problem = pcall(function()
          require('aerial').nav_open()
        end)
        if not opened then
          say('aerial, called before it loaded, was not set up: ' .. tostring(problem))
        end
        later.provide('jump.line')
        if not later.loaded('flash.nvim') or keys.actions['jump.line'] == nil then
          say('asking for jump.line did not load flash')
        end
        vim.api.nvim_exec_autocmds('UIEnter', {})
        local background = { 'nvim-lspconfig', 'nvim-cmp', 'gitsigns.nvim', 'which-key.nvim' }
        vim.wait(5000, function()
          return vim.iter(background):all(later.loaded)
        end)
        for _, name in ipairs(background) do
          if not later.loaded(name) then
            say(name .. ' did not load once the first screen was drawn')
          end
        end
        -- Completion's sources register in their after/plugin files.
        vim.wait(2000, function()
          return #require('cmp').core:get_sources() >= 2
        end)
        local sources = {}
        for _, source in ipairs(require('cmp').core:get_sources()) do
          sources[source.name] = true
        end
        if not (sources.buffer and sources.path) then
          say('suggestions come from ' .. table.concat(vim.tbl_keys(sources), ' '))
        end
        local library = vim.lsp.config.lua_ls.settings.Lua.workspace.library
        if not vim.deep_equal(library, { vim.env.VIMRUNTIME, "''${3rd}/luv/library" }) then
          say('lua_ls reads ' .. #library .. ' folders')
        end
        io.stdout:write(#wrong == 0 and 'as designed' or table.concat(wrong, '\n'))
        vim.cmd('qa!')
      '';
    in
    pkgs.runCommandLocal "neovim-starts-with-only-what-it-draws"
      { nativeBuildInputs = [ home.home.path ]; }
      ''
        ${neovimAt home}
        echo 'return {}' > probe.lua
        # From VimEnter, which a person meets Neovim after, rather than from
        # a -c command, which runs before it. A probe that fails says why and
        # quits, and one that never finishes is stopped, so the check fails
        # rather than waits.
        timeout 120 nvim --headless --cmd 'autocmd VimEnter * ++once lua local ok, problem = xpcall(dofile, debug.traceback, "${probe}") if not ok then io.stdout:write(problem) vim.cmd("qa!") end' probe.lua \
          > answer 2> complaints || true
        if [ -s complaints ]; then cat complaints; exit 1; fi
        if [ ! -s answer ]; then echo 'Neovim did not finish in two minutes'; exit 1; fi
        if [ "$(cat answer)" != "as designed" ]; then cat answer; exit 1; fi
        touch $out
      '';

  # Neovim's floats stand apart by their shade, with a step of it round
  # their words rather than a line, and nothing behind them shows through:
  # a problem shown in full, a language server's note, a notice, the
  # suggestions, the outline and a change's preview. A notice's mark comes
  # before its words.
  neovim-floats-by-shade =
    let
      home = account workstation;
      probe = pkgs.writeText "probe.lua" ''
        require('later').load_all()
        local wrong = {}
        local function say(line)
          table.insert(wrong, line)
        end
        local function solid(where, border)
          if border ~= 'solid' then
            say(where .. ' has the border ' .. vim.inspect(border))
          end
        end
        -- A float Neovim drew: its border all spaces, and nothing through it.
        local function look(where, win)
          if not win or not vim.api.nvim_win_is_valid(win) then
            return say(where .. ' opened no float')
          end
          local border = vim.api.nvim_win_get_config(win).border
          local spaces = type(border) == 'table' and #border == 8
          for _, part in ipairs(type(border) == 'table' and border or {}) do
            if (type(part) == 'table' and part[1] or part) ~= ' ' then
              spaces = false
            end
          end
          if not spaces then
            say(where .. ' is drawn with the border ' .. vim.inspect(border))
          end
          if vim.wo[win].winblend ~= 0 then
            say(where .. ' shows what is behind it')
          end
        end

        if vim.o.pumblend ~= 0 then
          say('the suggestions show what is behind them')
        end
        local namespace = vim.api.nvim_create_namespace('probe')
        vim.diagnostic.set(namespace, 0, {
          { lnum = 0, col = 0, message = 'a problem', severity = vim.diagnostic.severity.WARN },
        })
        local _, problem = vim.diagnostic.open_float()
        look('a problem shown in full', problem)
        pcall(vim.api.nvim_win_close, problem, true)
        local _, note = vim.lsp.util.open_floating_preview({ 'a note' }, 'markdown', {})
        look("a language server's note", note)
        pcall(vim.api.nvim_win_close, note, true)

        vim.notify('a notice', vim.log.levels.INFO)
        local notice
        vim.wait(2000, function()
          for _, win in ipairs(vim.api.nvim_list_wins()) do
            if vim.bo[vim.api.nvim_win_get_buf(win)].filetype == 'snacks_notif' then
              notice = win
            end
          end
          return notice ~= nil
        end)
        look('a notice', notice)
        if notice then
          local first = vim.api.nvim_buf_get_lines(vim.api.nvim_win_get_buf(notice), 0, 1, false)[1]
          local mark = require('theme').glyphs.note
          if first:sub(1, #mark) ~= mark then
            say('a notice reads "' .. first .. '", not its mark and then its words')
          end
        end

        local window = require('cmp').get_config().window
        solid('the suggestions', window.completion.border)
        solid("a suggestion's notes", window.documentation.border)
        -- aerial puts its settings in place when first used.
        require('aerial').sync_load()
        local aerial = require('aerial.config')
        solid('the outline', aerial.nav.border)
        solid('the outline beside the cursor', aerial.float.border)
        if aerial.nav.win_opts.winblend ~= 0 then
          say('the outline shows what is behind it')
        end
        solid("a change's preview", require('gitsigns.config').config.preview_config.border)

        -- The finder: on the floats' shade, nothing dimmed behind it, and
        -- its title at the list's edge rather than a space in.
        local finder = require('fzf-lua.config').globals.winopts
        solid('the finder', finder.border)
        solid("the finder's preview", finder.preview.border)
        if finder.backdrop ~= false then
          say('the finder dims what is behind it')
        end
        local shade = vim.api.nvim_get_hl(0, { name = 'NormalFloat', link = false }).bg
        if vim.api.nvim_get_hl(0, { name = 'FzfLuaNormal', link = false }).bg ~= shade then
          say("the finder isn't on the floats' shade")
        end
        local titled = require('fzf-lua.utils').nvim_open_win(vim.api.nvim_create_buf(false, true), false, {
          relative = 'editor', row = 1, col = 1, width = 20, height = 2, title = { { ' Files ', 'FloatTitle' } },
        })
        local title = vim.api.nvim_win_get_config(titled).title
        if title[1][1] ~= 'Files' then
          say('the finder is titled "' .. title[1][1] .. '", not from its edge')
        end

        -- While a float has the keys, the status line names the file under it.
        vim.api.nvim_set_current_win(titled)
        local named = {}
        for _, piece in ipairs((require('status').layout(200))) do
          table.insert(named, piece[2])
        end
        if not table.concat(named):find('probe.lua', 1, true) then
          say('the status line names "' .. table.concat(named) .. '" over a float, not the file under it')
        end
        io.stdout:write(#wrong == 0 and 'as designed' or table.concat(wrong, '\n'))
        vim.cmd('qa!')
      '';
    in
    pkgs.runCommandLocal "neovim-floats-by-shade" { nativeBuildInputs = [ home.home.path ]; } ''
      ${neovimAt home}
      echo 'return {}' > probe.lua
      timeout 120 nvim --headless --cmd 'autocmd VimEnter * ++once lua local ok, problem = xpcall(dofile, debug.traceback, "${probe}") if not ok then io.stdout:write(problem) vim.cmd("qa!") end' probe.lua \
        > answer 2> complaints || true
      if [ -s complaints ]; then cat complaints; exit 1; fi
      if [ ! -s answer ]; then echo 'Neovim did not finish in two minutes'; exit 1; fi
      if [ "$(cat answer)" != "as designed" ]; then cat answer; exit 1; fi
      touch $out
    '';

  # Space in any of Neovim's groups finds among what it holds: every group
  # keys.nix gives keys has its own Space, and one whose keys are things
  # to do lists exactly its keys that work here, each by what it does in
  # strong and then its keys in text, in a column of their own, titled by
  # the group, and runs the one chosen as if pressed. A group with nothing
  # that works here says so rather than opening an empty list.
  neovim-finds-in-every-group =
    let
      home = account workstation;
      chords = map (key: key.chord) (import ./keys.nix);
      groups = lib.unique (
        lib.concatMap (
          chord:
          let
            words = lib.splitString " " chord;
          in
          lib.optionals (lib.head words == "Space" && lib.length words > 2) (
            map (length: lib.concatStringsSep " " (lib.take length words)) (lib.range 2 (lib.length words - 1))
          )
        ) chords
      );
      lacking = lib.filter (group: !(lib.elem "${group} Space" chords)) groups;
      probe = pkgs.writeText "probe.lua" ''
        -- Every plugin loaded first, so none replaces the stand-ins after.
        require('later').load_all()
        local keys = require('keys')
        local wrong = {}
        local function say(line)
          table.insert(wrong, line)
        end
        local offered, rows, chosen, told, titled
        vim.ui.select = function(items, opts, on_choice)
          offered, rows, titled = {}, #items, opts.prompt
          for _, item in ipairs(items) do
            offered[item.rest] = opts.format_item(item)
          end
          chosen = function(rest)
            for _, item in ipairs(items) do
              if item.rest == rest then
                return on_choice(item)
              end
            end
          end
        end
        vim.notify = function(message)
          told = message
        end

        local looks = 0
        for _, key in ipairs(keys.keys) do
          if key.chord:match('^Space u %S+$') and key.chord ~= 'Space u Space' then
            looks = looks + 1
          end
        end
        keys.find('Space u', 'looks')
        local count = vim.tbl_count(offered or {})
        if count ~= looks then
          say('Space u Space offers ' .. count .. ' keys, not its ' .. looks)
        end
        local function paint(role, words)
          local hex = require('theme')[role]
          return ('\27[38;2;%d;%d;%dm%s\27[0m'):format(
            tonumber(hex:sub(2, 3), 16), tonumber(hex:sub(4, 5), 16), tonumber(hex:sub(6, 7), 16), words)
        end
        local row = (offered or {}).w or '''
        local said = row:match('^' .. vim.pesc(paint('strong', 'Wrap long lines, or not')) .. ' +')
        if not said or row ~= said .. paint('text', 'w') then
          say('Space u Space shows w as "' .. vim.inspect(row) .. '"')
        end
        local columns = {}
        for rest, line in pairs(offered or {}) do
          local plain = line:gsub('\27%[[%d;]*m', ''')
          columns[#plain - #rest] = true
        end
        if vim.tbl_count(columns) ~= 1 then
          say('Space u Space puts its keys in ' .. vim.tbl_count(columns) .. ' columns, not one')
        end
        if titled ~= 'Looks' then
          say('Space u Space is titled "' .. tostring(titled) .. '"')
        end
        local wrapped = vim.wo.wrap
        chosen('w')
        if vim.wo.wrap == wrapped then
          say('choosing w from Space u Space left wrapping as it was')
        end

        -- In an issue, its comments' keys, once each, though octo's other
        -- windows give the same chords keys of their own.
        vim.cmd('enew')
        require('octo.utils').apply_mappings('issue', 0)
        offered = nil
        keys.find('Space g h c', 'comments')
        local comments = vim.tbl_keys(offered or {})
        table.sort(comments)
        if table.concat(comments, ' ') ~= 'Return h i r w' or rows ~= 5 then
          say('Space g h c Space in an issue offers ' .. tostring(rows) .. ': ' .. table.concat(comments, ', '))
        end
        -- And in a review's change, its own comment key, not an issue's.
        vim.cmd('enew')
        require('octo.utils').apply_mappings('review_diff', 0)
        offered = nil
        keys.find('Space g h c', 'comments')
        local comment = (offered or {}).Return or '''
        if not comment:find('Comment on the line', 1, true) then
          say('Space g h c Space in a change offers Return as "' .. comment .. '"')
        end
        vim.cmd('bwipeout!')
        vim.cmd('bwipeout!')

        offered = nil
        keys.find('Space g s', 'stage')
        if offered ~= nil then
          say('Space g s Space offers git keys in a file git does not track')
        elseif told ~= 'No key for stage works here' then
          say('Space g s Space with nothing to offer said "' .. tostring(told) .. '"')
        end
        io.stdout:write(#wrong == 0 and 'as designed' or table.concat(wrong, '\n'))
        vim.cmd('qa!')
      '';
    in
    if lacking != [ ] then
      holds "neovim-finds-in-every-group"
        "These groups have no Space: ${lib.concatStringsSep ", " lacking}."
        false
    else
      pkgs.runCommandLocal "neovim-finds-in-every-group" { nativeBuildInputs = [ home.home.path ]; } ''
        ${neovimAt home}
        echo 'return {}' > probe.lua
        timeout 120 nvim --headless --cmd 'autocmd VimEnter * ++once lua local ok, problem = xpcall(dofile, debug.traceback, "${probe}") if not ok then io.stdout:write(problem) vim.cmd("qa!") end' probe.lua \
          > answer 2> complaints || true
        if [ -s complaints ]; then cat complaints; exit 1; fi
        if [ ! -s answer ]; then echo 'Neovim did not finish in two minutes'; exit 1; fi
        if [ "$(cat answer)" != "as designed" ]; then cat answer; exit 1; fi
        touch $out
      '';

  # Neovim's status line, drawn for a file in a repository with changes
  # not yet committed, with git's state from the engine: the file told
  # from its root, the branch with its unsaved mark, a mark once the
  # buffer changes, an error counted, the mode once it isn't Normal, and
  # all of it fitting a narrow window.
  neovim-draws-its-status-line =
    let
      home = account workstation;
      probe = pkgs.writeText "probe.lua" ''
        local status = require('status')
        local g = require('theme').glyphs
        local wrong = {}
        local function plain(pieces)
          local text = {}
          for _, piece in ipairs(pieces) do
            table.insert(text, piece[2])
          end
          return table.concat(text)
        end
        local function expect(what, columns, left, right)
          local l, r = status.layout(columns)
          l, r = plain(l), plain(r)
          if l ~= left or r ~= right then
            table.insert(wrong, what .. ': "' .. l .. '" and "' .. r .. '"')
          end
        end
        vim.wait(5000, function()
          return #select(2, status.layout(200)) > 0
        end)
        expect('at rest', 120, 'project/sub/notes.txt', g.branch .. ' main ' .. g.on)
        vim.api.nvim_buf_set_lines(0, -1, -1, false, { 'more' })
        expect('changed', 120, 'project/sub/notes.txt ' .. g.on, g.branch .. ' main ' .. g.on)
        vim.diagnostic.set(vim.api.nvim_create_namespace('probe'), 0, {
          { lnum = 0, col = 0, severity = vim.diagnostic.severity.ERROR, message = 'e' },
        })
        expect('an error', 120, 'project/sub/notes.txt ' .. g.on,
          g.failed .. ' 1  ' .. g.branch .. ' main ' .. g.on)
        expect('narrow', 35, 'project/' .. g.more .. '/notes.txt ' .. g.on,
          g.failed .. ' 1  ' .. g.branch .. ' ' .. g.on .. '  ' .. g.more)
        vim.api.nvim_feedkeys('i', 'n', false)
        vim.defer_fn(function()
          expect('typing', 120, 'project/sub/notes.txt ' .. g.on .. '  insert',
            g.failed .. ' 1  ' .. g.branch .. ' main ' .. g.on)
          vim.cmd('stopinsert')
          vim.defer_fn(function()
            -- The file as fugitive opens it in the index: told from its
            -- root as the file is, the version after it, git's state its
            -- folder's.
            vim.cmd('enew')
            vim.bo.buftype = 'nowrite'
            vim.api.nvim_buf_set_name(0, 'fugitive://' .. vim.fn.getcwd() .. '/project/.git//0/sub/notes.txt')
            expect('a version', 120, 'project/sub/notes.txt  index', g.branch .. ' main ' .. g.on)
            -- A terminal by its program, not its whole term:// address.
            vim.cmd('terminal sleep 5')
            expect('a terminal', 120, 'terminal sleep 5', ''')
            io.stdout:write(#wrong == 0 and 'as designed' or table.concat(wrong, '\n'))
            vim.cmd('qa!')
          end, 100)
        end, 200)
      '';
    in
    pkgs.runCommandLocal "neovim-draws-its-status-line" { nativeBuildInputs = [ home.home.path ]; } ''
      ${neovimAt home}
      mkdir -p project/sub
      # Text, which no language server reads, so none reports its progress.
      echo 'a first line' > project/sub/notes.txt
      git -C project init -q -b main
      git -C project add -A
      git -C project -c user.name=check -c user.email=check@localhost -c commit.gpgsign=false \
        commit -q -m first
      echo 'a second line' >> project/sub/notes.txt
      nvim --headless project/sub/notes.txt -c 'luafile ${probe}' > answer 2> complaints
      if [ -s complaints ]; then cat complaints; exit 1; fi
      if [ "$(cat answer)" != "as designed" ]; then cat answer; exit 1; fi
      touch $out
    '';

  # Neovim's numbers stand in one column: the line the cursor is on says
  # its own number, marked, right-aligned with the counts above and below
  # it rather than pushed left, and a wrapped line's later rows say none.
  neovim-keeps-its-numbers-in-one-column =
    let
      home = account workstation;
      probe = pkgs.writeText "probe.lua" ''
        local wrong = {}
        vim.wo.wrap = true
        vim.api.nvim_win_set_cursor(0, { 61, 0 })
        local function column(lnum)
          return vim.api.nvim_eval_statusline(vim.o.statuscolumn, {
            use_statuscol_lnum = lnum, highlights = true, winid = 0,
          })
        end
        local above, here, below = column(60), column(61), column(62)
        if not here.str:match('^%s*61 $') then
          table.insert(wrong, 'the line here reads "' .. here.str .. '"')
        end
        if not above.str:match('^%s*1 $') or not below.str:match('^%s*1 $') then
          table.insert(wrong, 'the lines beside it read "' .. above.str .. '" and "' .. below.str .. '"')
        end
        if #above.str ~= #here.str or #below.str ~= #here.str then
          table.insert(wrong, 'the numbers are not one width: "' .. above.str .. '", "' .. here.str .. '", "' .. below.str .. '"')
        end
        local marked = false
        for _, mark in ipairs(here.highlights) do
          marked = marked or mark.group == 'CursorLineNr'
        end
        if not marked then
          table.insert(wrong, 'the line here is not marked as the cursor line')
        end
        vim.wo.relativenumber, vim.wo.number = false, false
        if vim.trim(column(60).str) ~= ''' then
          table.insert(wrong, 'with numbers off the column says "' .. column(60).str .. '"')
        end
        io.stdout:write(#wrong == 0 and 'as designed' or table.concat(wrong, '\n'))
        vim.cmd('qa!')
      '';
    in
    pkgs.runCommandLocal "neovim-keeps-its-numbers-in-one-column"
      { nativeBuildInputs = [ home.home.path ]; }
      ''
        ${neovimAt home}
        seq 1 120 > lines.txt
        nvim --headless lines.txt -c 'luafile ${probe}' > answer 2> complaints
        if [ -s complaints ]; then cat complaints; exit 1; fi
        if [ "$(cat answer)" != "as designed" ]; then cat answer; exit 1; fi
        touch $out
      '';

  # Neovim's spell check in a markdown file knows the names this writing
  # uses, and still finds a word spelt wrong.
  neovim-knows-the-names-it-writes =
    let
      home = account workstation;
      probe = pkgs.writeText "probe.lua" ''
        local line = vim.api.nvim_buf_get_lines(0, 0, 1, false)[1]
        local flagged = {}
        for _, bad in ipairs(vim.spell.check(line)) do
          table.insert(flagged, bad[1])
        end
        local said = table.concat(flagged, ' ')
        io.stdout:write(vim.wo.spell and said == 'speling' and 'as designed'
          or ('spell ' .. tostring(vim.wo.spell) .. ', flagged: ' .. said))
        vim.cmd('qa!')
      '';
    in
    pkgs.runCommandLocal "neovim-knows-the-names-it-writes" { nativeBuildInputs = [ home.home.path ]; }
      ''
        ${neovimAt home}
        echo "Neovim's niri and Quickshell, a YubiKey and the TPM, in monospace: a speling slip." > notes.md
        nvim --headless notes.md -c 'luafile ${probe}' > answer 2> complaints
        if [ -s complaints ]; then cat complaints; exit 1; fi
        if [ "$(cat answer)" != "as designed" ]; then cat answer; exit 1; fi
        touch $out
      '';

  # The account's Neovim binds every key in the map, and only those: each
  # action a key names is one a file registered, or one a plugin that
  # waits provides, each key is bound as it says, a scope's in a buffer it
  # arrives in, each plugin's window holds its keys and none of the
  # plugin's own, and no key Neovim has that says what it does is missing
  # from the map. Every plugin is loaded first, as it would be once used.
  neovim-binds-every-key =
    let
      home = account workstation;
      probe = pkgs.writeText "probe.lua" ''
        local keys = require('keys')
        local later = require('later')
        local wrong = {}
        local function say(line)
          table.insert(wrong, line)
        end
        for _, name in ipairs(keys.missing()) do
          if not later.provides(name) then
            say('nothing provides ' .. name)
          end
        end
        later.load_all()
        for _, name in ipairs(keys.missing()) do
          say('no action ' .. name)
        end
        -- Every scope arrives in this one buffer, as each would in its own.
        for _, scope in ipairs({ 'code', 'git', 'status' }) do
          keys.attach(scope, 0)
        end
        local ours = {}
        for _, key in ipairs(vim.tbl_filter(function(key)
          return key.plugin == nil
        end, keys.keys)) do
          for _, mode in ipairs(key.modes) do
            local map = vim.fn.maparg(key.lhs, mode, false, true)
            if map.desc ~= key.desc then
              say(mode .. ' ' .. key.lhs .. ' is ' .. tostring(map.desc))
            elseif (map.buffer == 1) ~= (key.scope ~= nil) then
              say(mode .. ' ' .. key.lhs .. ' is in the wrong place')
            else
              ours[mode .. map.lhsraw] = true
            end
          end
        end
        for _, mode in ipairs({ 'n', 'x', 'o', 'i', 't' }) do
          local maps = vim.list_extend(
            vim.api.nvim_get_keymap(mode),
            vim.api.nvim_buf_get_keymap(0, mode)
          )
          for _, map in ipairs(maps) do
            local raw = vim.fn.maparg(map.lhs, mode, false, true).lhsraw
            -- A key set once Neovim had started has no script, nor was it
            -- set by the configuration.
            local from = map.sid > 0
                and (vim.fn.getscriptinfo({ sid = map.sid })[1] or {}).name
              or ""
            if map.desc and not ours[mode .. raw] and from:find('/config/nvim/', 1, true) then
              say('stray ' .. mode .. ' ' .. map.lhs .. ' ' .. map.desc)
            end
          end
        end

        -- A plugin's window binds its own keys, so each window is opened
        -- as it would be, and holds its keys from keys.nix and no others
        -- but a mouse's.
        vim.fn.writefile({ 'local function hello()', 'end' }, 'probe.lua')
        vim.cmd('edit probe.lua')
        local function window(scope, filetype, open, theirs)
          open()
          if not vim.wait(5000, function() return vim.bo.filetype == filetype end) then
            say('no ' .. scope .. ' window')
            return
          end
          local mine, modes = {}, {}
          for _, key in ipairs(keys.keys) do
            if key.scope == scope then
              for _, mode in ipairs(key.modes) do
                modes[mode] = true
                local map = vim.fn.maparg(key.lhs, mode, false, true)
                if map.desc ~= key.desc or map.buffer ~= 1 then
                  say(scope .. ' ' .. mode .. ' ' .. key.lhs .. ' is ' .. tostring(map.desc))
                else
                  mine[mode .. map.lhsraw] = true
                end
              end
            end
          end
          for mode in pairs(modes) do
            for _, map in ipairs(vim.api.nvim_buf_get_keymap(0, mode)) do
              local raw = vim.fn.maparg(map.lhs, mode, false, true).lhsraw
              if not mine[mode .. raw] and not map.lhs:find('Mouse') and not (theirs and theirs(map)) then
                say('stray ' .. scope .. ' ' .. mode .. ' ' .. map.lhs .. ' ' .. tostring(map.desc))
              end
            end
          end
        end
        window('outline', 'aerial', function()
          require('aerial').open({ focus = true })
        end)
        vim.cmd('AerialClose')
        window('walk', 'aerial-nav', function()
          require('aerial').nav_open()
        end)
        vim.wait(200)
        require('aerial').nav_close()
        vim.wait(200)
        window('files', 'oil', function()
          vim.cmd('Oil .')
        end)
        vim.cmd('edit probe.lua')
        window('problems', 'trouble', function()
          vim.fn.setqflist({ { filename = 'probe.lua', lnum = 1, text = 'a problem' } })
          require('trouble').toggle('quickfix')
        end)
        -- octo binds a buffer's keys as it opens one, through apply_mappings,
        -- and its notifications' in the finder, from its configuration.
        local octo = {
          ['octo-issue'] = 'issue',
          ['octo-pull'] = 'pull_request',
          ['octo-thread'] = 'review_thread',
          ['octo-submit'] = 'submit_win',
          ['octo-change'] = 'review_diff',
          ['octo-files'] = 'file_panel',
        }
        for _, scope in ipairs(vim.tbl_keys(octo)) do
          -- autopairs binds its own keys for typing in every buffer.
          window(scope, 'octo-probe', function()
            vim.cmd('enew')
            vim.bo.filetype = 'octo-probe'
            require('octo.utils').apply_mappings(octo[scope], 0)
          end, function(map)
            return (map.desc or '''):find('^autopairs') ~= nil
          end)
        end
        -- Its notifications' and workflow runs' keys octo binds its own
        -- way, so they're read from its configuration, and its runs'
        -- view, which reads them as it loads, loads.
        local given = require('octo.config').values.mappings
        local theirs = { ['octo-notices'] = 'notification', ['octo-runs'] = 'runs' }
        for _, key in ipairs(keys.keys) do
          if theirs[key.scope] then
            local action = (given[theirs[key.scope]] or {})[key.plugin] or {}
            if action.lhs ~= key.lhs or action.desc ~= key.desc then
              say(key.scope .. ' ' .. key.lhs .. ' is not ' .. key.plugin)
            end
          end
        end
        local loaded, problem = pcall(require, 'octo.workflow_runs')
        if not loaded then
          say('octo cannot show a workflow run: ' .. tostring(problem))
        end
        -- cmp binds the suggestions' keys as it suggests, from these.
        local mapping = require('cmp').get_config().mapping
        local suggesting = vim.tbl_filter(function(key)
          return key.scope == 'completion'
        end, keys.keys)
        for _, key in ipairs(suggesting) do
          if mapping[require('cmp.utils.keymap').normalize(key.lhs)] == nil then
            say('completion ' .. key.lhs .. ' is unbound')
          end
        end
        if vim.tbl_count(mapping) ~= #suggesting then
          say('completion has ' .. vim.tbl_count(mapping) .. ' keys, not ' .. #suggesting)
        end
        -- The command line's are its own, for a command and a search alike.
        local lines = require('cmp.config').cmdline
        local commanding = vim.tbl_filter(function(key)
          return key.scope == 'commands'
        end, keys.keys)
        for _, kind in ipairs({ ':', '/', '?' }) do
          local line = require('cmp.config').normalize(lines[kind] or {}).mapping or {}
          for _, key in ipairs(commanding) do
            local bound = line[require('cmp.utils.keymap').normalize(key.lhs)]
            if bound == nil or bound.c == nil then
              say('command line ' .. kind .. ' ' .. key.lhs .. ' is unbound')
            end
          end
          if vim.tbl_count(line) ~= #commanding then
            say('command line ' .. kind .. ' has ' .. vim.tbl_count(line) .. ' keys, not ' .. #commanding)
          end
        end
        io.stdout:write(#wrong == 0 and 'every key' or table.concat(wrong, '\n'))
      '';
    in
    pkgs.runCommandLocal "neovim-binds-every-key" { nativeBuildInputs = [ home.home.path ]; } ''
      ${neovimAt home}
      # Neovim says where each key was set only when it's verbose, which
      # goes to a file rather than among the complaints.
      nvim --headless --cmd 'set verbose=1 verbosefile=verbose' \
        -c 'set verbose=0' -c 'luafile ${probe}' -c 'qa!' > answer 2> complaints
      if [ -s complaints ]; then cat complaints; exit 1; fi
      if [ "$(cat answer)" != "every key" ]; then cat answer; exit 1; fi
      touch $out
    '';
}
