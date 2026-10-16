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
      parts = lib.splitString "+" step;
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
      inherit (key) modes;
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

    initLua = lib.concatLines [
      "vim.cmd.colorscheme('jitsusama')"
      (builtins.readFile ./init.lua)
    ];

    plugins = [
      # A code outline window for skimming and quick navigation.
      (configured aerial ./plugins/aerial.lua)
      # Seamless integration with Claude Code.
      (configured vimPlugins.claudecode-nvim ./plugins/claudecode.lua)
      # A completion engine plugin.
      (configured vimPlugins.nvim-cmp ./plugins/cmp.lua)
      # nvim-cmp source for buffer words.
      vimPlugins.cmp-buffer
      # nvim-cmp source for vim's cmdline.
      vimPlugins.cmp-cmdline
      # nvim-cmp source for neovim's built-in language server client.
      vimPlugins.cmp-nvim-lsp
      # nvim-cmp source for filesystem paths.
      vimPlugins.cmp-path
      # Automatic insertion of closing brackets, quotes, etc.
      (configured vimPlugins.nvim-autopairs ./plugins/autopairs.lua)
      # Lightweight yet powerful formatter plugin.
      (configured vimPlugins.conform-nvim ./plugins/conform.lua)
      # Navigate your code with search labels, enhanced character motions and
      # Treesitter integration.
      (configured vimPlugins.flash-nvim ./plugins/flash.lua)
      # Powerful fuzzy finder powered by fzf.
      (configured vimPlugins.fzf-lua ./plugins/fzf-lua.lua)
      # A Git wrapper so awesome, it should be illegal.
      (configured vimPlugins.vim-fugitive ./plugins/fugitive.lua)
      # Deep buffer integration for Git.
      (configured vimPlugins.gitsigns-nvim ./plugins/gitsigns.lua)
      # Quickstart configs for Nvim LSP.
      (configured vimPlugins.nvim-lspconfig ./plugins/lsp.lua)
      # A framework for interacting with tests.
      (configured vimPlugins.neotest ./plugins/neotest.lua)
      # Neotest adapter for Minitest.
      vimPlugins.neotest-minitest
      # Companion plugin for pi: pairs nvim with a pi session over msgpack-rpc.
      (configured pi ./plugins/neovim-pi.lua)
      # A library for asynchronous IO.
      vimPlugins.nvim-nio
      # Support for writing Nix expressions.
      vimPlugins.vim-nix
      # Automatically toggle between relative and absolute line numbers.
      vimPlugins.nvim-numbertoggle
      # Edit and review GitHub issues and pull requests.
      (configured vimPlugins.octo-nvim ./plugins/octo.lua)
      # Edit your filesystem like a buffer.
      (configured vimPlugins.oil-nvim ./plugins/oil.lua)
      # Integration with OpenCode AI coding assistant.
      (configured vimPlugins.opencode-nvim ./plugins/opencode.lua)
      # Asynchronous utilities for writing neovim LUA.
      vimPlugins.plenary-nvim
      # The Refactoring library based off the refactoring book by Martin Fowler.
      (configured vimPlugins.refactoring-nvim ./plugins/refactoring.lua)
      # GitHub extension for fugitive.
      vimPlugins.vim-rhubarb
      # A collection of small QoL plugins.
      (configured vimPlugins.snacks-nvim ./plugins/snacks.lua)
      # SQLite LuaJIT binding with a very simple api.
      vimPlugins.sqlite-lua
      # Nvim Treesitter configurations and abstraction layer.
      (configured vimPlugins.nvim-treesitter.withAllGrammars ./plugins/treesitter.lua)
      # Syntax aware text-objects, select, move, swap, and peek support.
      vimPlugins.nvim-treesitter-textobjects
      # Provides mappings to easily delete, change and add surroundings in pairs.
      vimPlugins.vim-surround
      # Simple text alignment plugin.
      vimPlugins.vim-easy-align
      # Visualize undo history as a tree structure.
      (configured vimPlugins.undotree ./plugins/undotree.lua)
      # Highlight and search for todo comments like TODO, HACK, BUG in your code.
      (configured vimPlugins.todo-comments-nvim ./plugins/todo-comments.lua)
      # List for showing diagnostics, references, telescope results, quickfix and location lists.
      (configured vimPlugins.trouble-nvim ./plugins/trouble.lua)
      # Dims inactive portions of code for better focus using TreeSitter.
      (configured vimPlugins.twilight-nvim ./plugins/twilight.lua)
      # Provides file type icons, from the Nerd Fonts set, for plugins.
      vimPlugins.nvim-web-devicons
      # Minimal telescope for aerial compatibility.
      {
        plugin = vimPlugins.telescope-nvim;
        type = "lua";
        config = ''
          require('telescope').setup({})
        '';
      }
      # Helps you remember your keymaps, by showing available keybindings in a popup as you type.
      (configured vimPlugins.which-key-nvim ./plugins/which-key.lua)
    ];
  };
}
