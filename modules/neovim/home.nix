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
in
{
  imports = [ ../theme/home.nix ];

  # The theme's roles, for the plugins that draw on the terminal's own
  # background (require('theme').background), with the terminal's colours,
  # the shades and the marks beside them.
  xdg.configFile."nvim/lua/theme.lua".text = "return ${
    lib.generators.toLua { } (
      config.jitsusama.theme.roles
      // {
        inherit (config.jitsusama.theme) terminal shades;
        glyphs = lib.mapAttrs (_: mark: mark.glyph) config.jitsusama.theme.glyphs;
      }
    )
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
      (builtins.readFile ./init.lua)
    ];

    plugins = [
      # A code outline window for skimming and quick navigation.
      (configured aerial ./plugins/aerial.lua)
      # The colour scheme Omarchy gives Neovim in its Osaka Jade theme.
      (configured vimPlugins.bamboo-nvim ./plugins/bamboo.lua)
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
      # A blazing fast and statusline.
      (configured vimPlugins.lualine-nvim ./plugins/lualine.lua)
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
      (configured vimPlugins.vim-rhubarb ./plugins/rhubarb.lua)
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
      (configured vimPlugins.vim-easy-align ./plugins/easy-align.lua)
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
