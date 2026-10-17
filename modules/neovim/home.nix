{ neovim-pi }:
{
  config,
  pkgs,
  lib,
  ...
}:
let
  inherit (pkgs) vimPlugins;
  # Most plugins are configured by a Lua file of the same name in ./plugins.
  configured = plugin: file: {
    inherit plugin;
    type = "lua";
    config = builtins.readFile file;
  };
  # The rest wait until they're wanted (later.lua), each with what wakes
  # it, and run their file, if any, once loaded.
  waits = plugin: file: how: {
    inherit plugin;
    optional = true;
    type = "lua";
    config = "require('later').add(${
      lib.generators.toLua { } (lib.getName plugin)
    }, ${lib.generators.toLua { } how}, ${
      if file == null then "nil" else "function()\n${builtins.readFile file}\nend"
    })";
  };
  aerial = vimPlugins.aerial-nvim.overrideAttrs (old: {
    patches = (old.patches or [ ]) ++ [ ./patches/aerial-fzf-lua.patch ];
  });
  pi = pkgs.vimUtils.buildVimPlugin {
    pname = "neovim-pi";
    version = neovim-pi.shortRev or "dev";
    src = neovim-pi;
  };

  keys = map (
    key:
    {
      modes = [ "n" ];
      scope = null;
    }
    // key
  ) (import ./keys.nix);

  # A chord as the map spells it, in Neovim's own spelling: Space b w is
  # <leader>bw, Ctrl+W Space is <C-w><Space>.
  named = {
    Space = "<Space>";
    Tab = "<Tab>";
    Return = "<CR>";
    Escape = "<Esc>";
    Left = "<Left>";
    Right = "<Right>";
    Up = "<Up>";
    Down = "<Down>";
    "<" = "<lt>";
  };
  held = {
    Ctrl = "C";
    Shift = "S";
    Alt = "M";
  };
  spellStep =
    index: step:
    let
      # + joins the keys held together, so + alone is the key itself.
      parts = if step == "+" then [ "+" ] else lib.splitString "+" step;
      key = lib.last parts;
      # A named key held with others loses its brackets: <S-Esc>.
      bare = lib.removePrefix "<" (lib.removeSuffix ">" (named.${key} or key));
    in
    if index == 0 && step == "Space" then
      "<leader>"
    else if lib.length parts == 1 then
      named.${step} or step
    else
      "<${lib.concatMapStrings (modifier: "${held.${modifier}}-") (lib.init parts)}${
        if lib.stringLength key == 1 then lib.toLower key else bare
      }>";
  spell = chord: lib.concatStrings (lib.imap0 spellStep (lib.splitString " " chord));

  # Where a key works, for the map, when it's not everywhere.
  scopes = {
    code = "in code";
    git = "in a file git tracks";
    status = "in git's status";
    outline = "in the outline";
    walk = "walking the outline";
    files = "in the file browser";
    problems = "in the problems list";
    completion = "with suggestions";
    commands = "on the command line";
    octo-issue = "in a GitHub issue";
    octo-pull = "in a pull request";
    octo-thread = "in a review's thread";
    octo-submit = "submitting a review";
    octo-change = "in a review's change";
    octo-files = "in a review's changed files";
    octo-runs = "in a workflow run";
    octo-notices = "finding GitHub's notifications";
  };
  modeWhere = {
    x = "with text selected";
    "x o" = "selecting, or after an operator";
    t = "in a terminal";
    i = "while typing";
    "i s" = "while typing";
  };
  where =
    key:
    lib.concatStringsSep ", " (
      lib.filter (part: part != "") [
        (if key.scope == null then "" else scopes.${key.scope})
        (modeWhere.${lib.concatStringsSep " " key.modes} or "")
      ]
    );

  # What Neovim binds each key to, for lua/keys.lua.
  bound =
    key:
    {
      inherit (key) chord modes;
      lhs = spell key.chord;
      desc = key.does;
    }
    // lib.optionalAttrs (key.scope != null) { inherit (key) scope; }
    // lib.optionalAttrs (key ? lua) {
      lua = lib.generators.mkLuaInline "function() ${key.lua} end";
    }
    // lib.optionalAttrs (key ? options) {
      options = lib.generators.mkLuaInline key.options;
    }
    // lib.getAttrs (lib.filter (field: key ? ${field}) [
      "action"
      "command"
      "keys"
      "expr"
      "remap"
      "plugin"
    ]) key;
in
{
  imports = [
    ../theme/home.nix
    ../keys/home.nix
    ../prompt/home.nix
  ];

  # Every key, for the launcher to find by what it does.
  jitsusama.keys.neovim = map (key: {
    inherit (key) chord does;
    when = where key;
  }) keys;

  # Every key bound, from keys.nix: the global ones as Neovim starts, and a
  # scope's in a buffer when its plugin calls attach. A key whose action a
  # plugin's file registers looks it up when it's pressed, so the files
  # can load in any order.
  xdg.configFile."nvim/lua/keys.lua".text = ''
    -- Neovim's keys, written from keys.nix, which says what each is for.
    local M = { actions = {} }

    M.keys = ${lib.generators.toLua { } (map bound keys)}

    -- A function for a key to run, by the name keys.nix gives it.
    function M.act(name, fn)
      M.actions[name] = fn
    end

    local function rhs(key)
      if key.lua then
        return key.lua
      elseif key.action then
        return function(...)
          local fn = M.actions[key.action]
          if fn == nil and require('later').provide(key.action) then
            fn = M.actions[key.action]
          end
          if fn == nil then
            vim.notify('Nothing does ' .. key.action .. ' yet', vim.log.levels.ERROR)
            return
          end
          return fn(...)
        end
      elseif key.command then
        return '<cmd>' .. key.command .. '<cr>'
      end
      return key.keys
    end

    local function bind(key, buffer)
      vim.keymap.set(key.modes, key.lhs, rhs(key), {
        desc = key.desc,
        buffer = buffer,
        expr = key.expr,
        remap = key.remap,
        silent = key.command ~= nil,
      })
    end

    -- A scope's keys, in the buffer it has arrived in.
    function M.attach(scope, buffer)
      for _, key in ipairs(M.keys) do
        if key.scope == scope and key.plugin == nil then
          bind(key, buffer)
        end
      end
    end

    -- A plugin window's keys, for its plugin to bind: each key's spelling,
    -- to what make turns it into in the plugin's own terms.
    function M.window(scope, make)
      local window = {}
      for _, key in ipairs(M.keys) do
        if key.scope == scope and key.plugin ~= nil then
          window[key.lhs] = make(key)
        end
      end
      return window
    end

    -- A key run as if pressed: a plugin's own by pressing it, since its
    -- plugin bound it, and the rest by what keys.nix says it does.
    local function run(key)
      if key.plugin then
        return vim.api.nvim_feedkeys(vim.keycode(key.lhs), 'm', false)
      end
      local how = rhs(key)
      if type(how) == 'function' then
        return how()
      end
      vim.api.nvim_feedkeys(vim.keycode(how), key.remap and 'm' or 'n', false)
    end

    -- Whether a key works here: everywhere, or bound in this buffer as this
    -- key, since a plugin's windows can give one chord different keys.
    local function here(key)
      if key.scope == nil then
        return true
      end
      local bound = vim.fn.maparg(key.lhs, 'n', false, true)
      return bound.buffer == 1 and bound.desc == key.desc
    end

    -- Words in one of the theme's roles, as the finder draws them.
    local function painted(role, words)
      local hex = require('theme')[role]
      local red, green, blue =
        tonumber(hex:sub(2, 3), 16), tonumber(hex:sub(4, 5), 16), tonumber(hex:sub(6, 7), 16)
      return ('\27[38;2;%d;%d;%dm%s\27[0m'):format(red, green, blue, words)
    end

    -- A group's keys, found by what they do and run as if pressed: Space
    -- in a group whose keys are things to do rather than things to find.
    -- Each row reads as the launcher's do, what it does and then its keys,
    -- one step quieter, in a column of their own.
    function M.find(group, name)
      local found, seen, widest = {}, {}, 0
      for _, key in ipairs(M.keys) do
        local rest = key.chord:sub(#group + 2)
        if
          vim.tbl_contains(key.modes, 'n')
          and not seen[key.lhs]
          and here(key)
          and key.chord:sub(1, #group + 1) == group .. ' '
          and rest ~= 'Space'
        then
          table.insert(found, { key = key, rest = rest })
          seen[key.lhs] = true
          widest = math.max(widest, vim.fn.strdisplaywidth(key.desc))
        end
      end
      if #found == 0 then
        vim.notify('No key for ' .. name .. ' works here', vim.log.levels.INFO)
        return
      end
      vim.ui.select(found, {
        prompt = (name:gsub('^%l', string.upper)),
        format_item = function(item)
          local gap = string.rep(' ', widest - vim.fn.strdisplaywidth(item.key.desc) + 2)
          return painted('strong', item.key.desc) .. gap .. painted('text', item.rest)
        end,
      }, function(item)
        if item then
          run(item.key)
        end
      end)
    end

    -- The names keys.nix gives that no file has registered.
    function M.missing()
      local names = {}
      for _, key in ipairs(M.keys) do
        if key.action and M.actions[key.action] == nil then
          table.insert(names, key.action)
        end
      end
      return names
    end

    for _, key in ipairs(M.keys) do
      if key.scope == nil then
        bind(key)
      end
    end

    return M
  '';

  # The theme's roles, for the colour scheme and the plugins that draw on the
  # terminal's own background (require('theme').background), with the
  # terminal's colours, code's roles, the shades and the marks beside them.
  xdg.configFile."nvim/lua/theme.lua".text = "return ${
    lib.generators.toLua { } (
      config.jitsusama.theme.roles
      // {
        inherit (config.jitsusama.theme)
          terminal
          syntax
          shades
          appearance
          ;
        glyphs = lib.mapAttrs (_: mark: mark.glyph) config.jitsusama.theme.glyphs;
      }
    )
  }\n";

  # The theme as Neovim's colour scheme, every colour one of its roles.
  xdg.configFile."nvim/colors/jitsusama.lua".source = ./colors/jitsusama.lua;
  # The names English doesn't know, checked beside it (init.lua's
  # spelllang), built once here rather than each time Neovim starts.
  xdg.configFile."nvim/spell/names.utf-8.spl".source =
    pkgs.runCommandLocal "names.utf-8.spl" { nativeBuildInputs = [ pkgs.neovim-unwrapped ]; }
      ''
        HOME=$TMPDIR nvim --clean --headless -c "mkspell! $TMPDIR/names.utf-8.spl ${./names.add}" -c 'qa!'
        mv $TMPDIR/names.utf-8.spl $out
      '';

  # The plugins that wait, and what each brings: the modules it holds and
  # the commands it and its file make, found by starting this Neovim once
  # with nothing yet known and loading every plugin in turn.
  xdg.configFile."nvim/lua/later.lua".source = ./later.lua;
  xdg.configFile."nvim/lua/waiting.lua".source =
    let
      files = [
        "init.lua"
        "lua/keys.lua"
        "lua/theme.lua"
        "lua/where.lua"
        "lua/status.lua"
        "lua/later.lua"
        "colors/jitsusama.lua"
      ];
    in
    # gitsigns and octo look for git and gh as they set up, and neovim-pi
    # opens a socket.
    pkgs.runCommandLocal "waiting.lua"
      {
        nativeBuildInputs = [
          config.programs.neovim.package
          pkgs.git
          pkgs.gh
        ];
      }
      ''
        export HOME=$PWD XDG_CONFIG_HOME=$PWD/config XDG_DATA_HOME=$PWD/data
        export XDG_STATE_HOME=$PWD/state XDG_CACHE_HOME=$PWD/cache XDG_RUNTIME_DIR=$PWD/run
        mkdir -p config/nvim/lua config/nvim/colors data/nvim/site/pack
        ${lib.concatMapStrings (file: ''
          cp ${config.xdg.configFile."nvim/${file}".source} config/nvim/${file}
        '') files}
        echo 'return {}' > config/nvim/lua/waiting.lua
        ln -s ${config.xdg.dataFile."nvim/site/pack/hm".source} data/nvim/site/pack/hm
        # An error anywhere fails the build, since one in init.lua stops it
        # before any plugin says how it waits, and the list would be empty.
        nvim --headless -c "lua os.exit(require('later').find('$out'))" 2> complaints || {
          cat complaints
          exit 1
        }
        if grep -v '\[INFO\]' complaints | grep -q .; then
          cat complaints
          exit 1
        fi
      '';

  # The status line, and what it tells a file's place and git's state by:
  # the prompt's root markers and its git-engine, so the two agree.
  xdg.configFile."nvim/lua/status.lua".source = ./status.lua;
  xdg.configFile."nvim/lua/where.lua".text = "return ${
    lib.generators.toLua { } {
      engine = lib.getExe (pkgs.callPackage ../git-engine/package.nix { });
      inherit (config.jitsusama.prompt) roots;
    }
  }\n";

  programs.git.ignores = [
    "*.swp"
    "*.swo"
    "*.swn"
    ".*.swp"
    ".*.swo"
    ".*.swn"
  ];

  programs.neovim = {
    enable = true;
    defaultEditor = true;
    viAlias = true;
    vimAlias = true;
    withRuby = false;

    extraPackages = [
      pkgs.nil # Nix LSP
      pkgs.nixfmt # Official Nix formatter
      pkgs.lua-language-server # Lua LSP
      pkgs.stylua # Lua formatter for conform.nvim
    ];

    # The bytecode cache first, so every module after it loads from it, then
    # what wakes a waiting plugin, so a module of one required anywhere
    # loads it.
    initLua = lib.concatLines [
      "vim.loader.enable()"
      "require('later')"
      "vim.cmd.colorscheme('jitsusama')"
      (builtins.readFile ./init.lua)
    ];

    plugins = [
      # Loaded as Neovim starts, since the first screen needs them.

      # Tree-sitter's grammars, to colour the first file as it's drawn.
      (configured vimPlugins.nvim-treesitter.withAllGrammars ./plugins/treesitter.lua)
      # Syntax aware text-objects, select, move, swap, and peek support.
      vimPlugins.nvim-treesitter-textobjects
      # The start screen, notices, and files too big to read whole.
      (configured vimPlugins.snacks-nvim ./plugins/snacks.lua)
      # The file browser, which a folder opens in.
      (configured vimPlugins.oil-nvim ./plugins/oil.lua)
      # Automatically toggle between relative and absolute line numbers.
      vimPlugins.nvim-numbertoggle
      # Support for writing Nix expressions.
      vimPlugins.vim-nix

      # Loaded once the first screen is drawn, since they work in the
      # background: language servers, suggestions, git's marks, the keys'
      # hints, and the agents' pairing.

      # The language servers, and the suggestions they feed.
      (waits vimPlugins.nvim-lspconfig ./plugins/lsp.lua {
        soon = true;
        actions = [
          "code.inlayHints"
          "code.folders"
        ];
      })
      (waits vimPlugins.nvim-cmp ./plugins/cmp.lua {
        soon = true;
        also = [
          "cmp-buffer"
          "cmp-cmdline"
          "cmp-nvim-lsp"
          "cmp-path"
        ];
      })
      (waits vimPlugins.cmp-buffer null { })
      (waits vimPlugins.cmp-cmdline null { })
      (waits vimPlugins.cmp-nvim-lsp null { })
      (waits vimPlugins.cmp-path null { })
      # Automatic insertion of closing brackets, quotes, etc.
      (waits vimPlugins.nvim-autopairs ./plugins/autopairs.lua { soon = true; })
      # Deep buffer integration for Git.
      (waits vimPlugins.gitsigns-nvim ./plugins/gitsigns.lua { soon = true; })
      # Highlight and search for todo comments like TODO, HACK, BUG in your code.
      (waits vimPlugins.todo-comments-nvim ./plugins/todo-comments.lua { soon = true; })
      # Helps you remember your keymaps, by showing available keybindings in a popup as you type.
      (waits vimPlugins.which-key-nvim ./plugins/which-key.lua { soon = true; })
      # Provides mappings to easily delete, change and add surroundings in pairs.
      (waits vimPlugins.vim-surround null { soon = true; })
      # Simple text alignment plugin.
      (waits vimPlugins.vim-easy-align null { soon = true; })
      # Companion plugin for pi: pairs nvim with a pi session over msgpack-rpc.
      (waits pi ./plugins/neovim-pi.lua { soon = true; })

      # Loaded when first asked for: a module required, a command run, an
      # action pressed, or the event it waits for.

      # Lightweight yet powerful formatter plugin, wanted when a file's written.
      (waits vimPlugins.conform-nvim ./plugins/conform.lua {
        on = [ "BufWritePre" ];
        commands = [ "ConformInfo" ];
      })
      # A code outline window for skimming and quick navigation.
      (waits aerial ./plugins/aerial.lua { })
      # Navigate your code with search labels.
      (waits vimPlugins.flash-nvim ./plugins/flash.lua {
        actions = [
          "jump.line"
          "jump.word"
        ];
      })
      # Powerful fuzzy finder powered by fzf, which answers every choice.
      (waits vimPlugins.fzf-lua ./plugins/fzf-lua.lua { replaces = [ "vim.ui.select" ]; })
      # A Git wrapper so awesome, it should be illegal.
      (waits vimPlugins.vim-fugitive ./plugins/fugitive.lua {
        actions = [
          "git.tag"
          "git.untag"
        ];
        also = [ "vim-rhubarb" ];
      })
      # GitHub extension for fugitive.
      (waits vimPlugins.vim-rhubarb null { needs = [ "vim-fugitive" ]; })
      # A framework for interacting with tests.
      (waits vimPlugins.neotest ./plugins/neotest.lua { })
      # Neotest adapter for Minitest.
      (waits vimPlugins.neotest-minitest null { })
      # Edit and review GitHub issues and pull requests.
      (waits vimPlugins.octo-nvim ./plugins/octo.lua { })
      # The Refactoring library based off the refactoring book by Martin Fowler.
      (waits vimPlugins.refactoring-nvim ./plugins/refactoring.lua { })
      # Visualize undo history as a tree structure.
      (waits vimPlugins.undotree ./plugins/undotree.lua { })
      # List for showing diagnostics, references, telescope results, quickfix and location lists.
      (waits vimPlugins.trouble-nvim ./plugins/trouble.lua { })
      # Dims inactive portions of code for better focus using TreeSitter.
      (waits vimPlugins.twilight-nvim ./plugins/twilight.lua { })
      # Libraries the others use.
      (waits vimPlugins.nvim-nio null { })
      (waits vimPlugins.plenary-nvim null { })
      (waits vimPlugins.sqlite-lua null { })
      (waits vimPlugins.nvim-web-devicons ./plugins/devicons.lua { })
    ];
  };
}
