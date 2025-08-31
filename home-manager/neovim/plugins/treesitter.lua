-- nvim-treesitter configuration
require('nvim-treesitter.configs').setup({
  ensure_installed = {},
  sync_install = false,
  auto_install = false,
  ignore_install = {},
  modules = {},
  highlight = {
    enable = true,
    additional_vim_regex_highlighting = false,
  },
  incremental_selection = {
    enable = true,
    keymaps = {
      init_selection = '<leader>css',
      node_incremental = '<leader>csi',
      scope_incremental = '<leader>csc',
      node_decremental = '<leader>csd',
    },
  },
  textobjects = {
    select = {
      enable = true,
      lookahead = true, -- Automatically jump forward to textobj
      keymaps = {
        ['af'] = '@function.outer',
        ['if'] = '@function.inner',
        ['ac'] = '@class.outer',
        ['ic'] = '@class.inner',
        ['al'] = '@loop.outer',
        ['il'] = '@loop.inner',
        ['aa'] = '@parameter.outer',
        ['ia'] = '@parameter.inner',
      },
    },
    move = {
      enable = true,
      set_jumps = true, -- Sets jumps in jumplist
      goto_next_start = {
        [']f'] = '@function.outer',
        [']c'] = '@class.outer',
        [']C'] = '@comment.outer',
      },
      goto_next_end = {
        [']F'] = '@function.outer',
      },
      goto_previous_start = {
        ['[f'] = '@function.outer',
        ['[c'] = '@class.outer',
        ['[C'] = '@comment.outer',
      },
      goto_previous_end = {
        ['[F'] = '@function.outer',
      },
    },
  },
  indent = {
    enable = true,
  },
  fold = {
    enable = true,
  },
})

-- Enable folding based on treesitter
vim.opt.foldmethod = 'expr'
vim.opt.foldexpr = 'nvim_treesitter#foldexpr()'
vim.opt.foldenable = false -- Start with folds open
