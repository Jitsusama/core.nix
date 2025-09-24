{
  pkgs,
  lib,
  ...
}:
{
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

    extraPackages = with pkgs; [
      nil # Nix LSP
      nixfmt-rfc-style # Official Nix formatter
      lua-language-server # Lua LSP
      stylua # Lua formatter for conform.nvim
    ];

    extraLuaConfig = lib.concatLines [
      (builtins.readFile ./init.lua)
    ];

    plugins = with pkgs.vimPlugins; [
      # A code outline window for skimming and quick navigation. - TODO: FIXME!!
      # {
      #   plugin = aerial-nvim;
      #   type = "lua";
      #   config = builtins.readFile ./plugins/aerial.lua;
      # }
      # Seamless integration with Claude Code.
      {
        plugin = pkgs.vimUtils.buildVimPlugin {
          pname = "claudecode-nvim";
          version = "2024-12-15";
          src = pkgs.fetchFromGitHub {
            owner = "coder";
            repo = "claudecode.nvim";
            rev = "v0.3.0";
            sha256 = "sha256-sOBY2y/buInf+SxLwz6uYlUouDULwebY/nmDlbFbGa8=";
          };
          # Patch to make config options actually work - TODO: see if this is no longer needed.
          # patches = [
          #   ./patches/claudecode-split.patch # Respect vertical_split setting
          # ];
        };
        type = "lua";
        config = builtins.readFile ./plugins/claudecode.lua;
      }
      # A completion engine plugin.
      {
        plugin = nvim-cmp;
        type = "lua";
        config = builtins.readFile ./plugins/cmp.lua;
      }
      # nvim-cmp source for buffer words.
      cmp-buffer
      # nvim-cmp source for vim's cmdline.
      cmp-cmdline
      # nvim-cmp source for neovim's built-in language server client.
      cmp-nvim-lsp
      # nvim-cmp source for filesystem paths.
      cmp-path
      # Automatic insertion of closing brackets, quotes, etc.
      {
        plugin = nvim-autopairs;
        type = "lua";
        config = builtins.readFile ./plugins/autopairs.lua;
      }
      # Lightweight yet powerful formatter plugin.
      {
        plugin = conform-nvim;
        type = "lua";
        config = builtins.readFile ./plugins/conform.lua;
      }
      # Navigate your code with search labels, enhanced character motions, and Treesitter integration.
      {
        plugin = flash-nvim;
        type = "lua";
        config = builtins.readFile ./plugins/flash.lua;
      }
      # Powerful fuzzy finder powered by fzf.
      {
        plugin = fzf-lua;
        type = "lua";
        config = builtins.readFile ./plugins/fzf-lua.lua;
      }
      # A Git wrapper so awesome, it should be illegal.
      {
        plugin = vim-fugitive;
        type = "lua";
        config = builtins.readFile ./plugins/fugitive.lua;
      }
      # Deep buffer integration for Git.
      {
        plugin = gitsigns-nvim;
        type = "lua";
        config = builtins.readFile ./plugins/gitsigns.lua;
      }
      # A port of gruvbox theme with treesitter and semantic highlighting.
      {
        plugin = gruvbox-nvim;
        type = "lua";
        config = builtins.readFile ./plugins/gruvbox.lua;
      }
      # Quickstart configs for Nvim LSP.
      {
        plugin = nvim-lspconfig;
        type = "lua";
        config = builtins.readFile ./plugins/lsp.lua;
      }
      # A blazing fast and statusline.
      {
        plugin = lualine-nvim;
        type = "lua";
        config = builtins.readFile ./plugins/lualine.lua;
      }
      # A framework for interacting with tests.
      {
        plugin = neotest;
        type = "lua";
        config = builtins.readFile ./plugins/neotest.lua;
      }
      # Neotest adapter for Minitest.
      {
        plugin = pkgs.vimUtils.buildVimPlugin {
          pname = "neotest-minitest";
          version = "2024-12-24";
          src = pkgs.fetchFromGitHub {
            owner = "zidhuss";
            repo = "neotest-minitest";
            rev = "4d1c19f80be0efff7656dea76a589c02bf418b68";
            sha256 = "sha256-cw8o6UG2ETPAtwdIP4dAjs+d4//SbUljfYBhzMqdUZ4=";
          };
          doCheck = false;
        };
      }
      # A library for asynchronous IO.
      nvim-nio
      # Support for writing Nix expressions.
      vim-nix
      # Automatically toggle between relative and absolute line numbers.
      nvim-numbertoggle
      # Edit and review GitHub issues and pull requests.
      {
        plugin = octo-nvim;
        type = "lua";
        config = builtins.readFile ./plugins/octo.lua;
      }
      # Edit your filesystem like a buffer.
      {
        plugin = oil-nvim;
        type = "lua";
        config = builtins.readFile ./plugins/oil.lua;
      }
      # Asynchronous utilities for writing neovim LUA.
      plenary-nvim
      # The Refactoring library based off the refactoring book by Martin Fowler.
      {
        plugin = refactoring-nvim;
        type = "lua";
        config = builtins.readFile ./plugins/refactoring.lua;
      }
      # GitHub extension for fugitive.
      {
        plugin = vim-rhubarb;
        type = "lua";
        config = builtins.readFile ./plugins/rhubarb.lua;
      }
      # A collection of small QoL plugins.
      {
        plugin = snacks-nvim;
        type = "lua";
        config = builtins.readFile ./plugins/snacks.lua;
      }
      # SQLite LuaJIT binding with a very simple api.
      sqlite-lua
      # Nvim Treesitter configurations and abstraction layer.
      {
        plugin = nvim-treesitter.withAllGrammars;
        type = "lua";
        config = builtins.readFile ./plugins/treesitter.lua;
      }
      # Syntax aware text-objects, select, move, swap, and peek support.
      nvim-treesitter-textobjects
      # Provides mappings to easily delete, change and add surroundings in pairs.
      vim-surround
      # Simple text alignment plugin.
      {
        plugin = vim-easy-align;
        type = "lua";
        config = builtins.readFile ./plugins/easy-align.lua;
      }
      # Visualize undo history as a tree structure.
      {
        plugin = undotree;
        type = "lua";
        config = builtins.readFile ./plugins/undotree.lua;
      }
      # Highlight and search for todo comments like TODO, HACK, BUG in your code.
      {
        plugin = todo-comments-nvim;
        type = "lua";
        config = builtins.readFile ./plugins/todo-comments.lua;
      }
      # List for showing diagnostics, references, telescope results, quickfix and location lists.
      {
        plugin = trouble-nvim;
        type = "lua";
        config = builtins.readFile ./plugins/trouble.lua;
      }
      # Dims inactive portions of code for better focus using TreeSitter.
      {
        plugin = pkgs.vimUtils.buildVimPlugin {
          pname = "twilight-nvim";
          version = "2024-12-31";
          src = pkgs.fetchFromGitHub {
            owner = "folke";
            repo = "twilight.nvim";
            rev = "8b7b50c0cb2dc781b2f4262a5ddd57571556d1e4";
            sha256 = "sha256-WFE8FZU8E8i6mX9bFl80YkIypGXOOhKvGbLrBoPce0g=";
          };
        };
        type = "lua";
        config = builtins.readFile ./plugins/twilight.lua;
      }
      # Provides Nerd Font icons for use by plugins.
      nvim-web-devicons
      # Minimal telescope for aerial compatibility.
      {
        plugin = telescope-nvim;
        type = "lua";
        config = ''
          require('telescope').setup({})
        '';
      }
      # Helps you remember your keymaps, by showing available keybindings in a popup as you type.
      {
        plugin = which-key-nvim;
        type = "lua";
        config = builtins.readFile ./plugins/which-key.lua;
      }
    ];
  };
}
