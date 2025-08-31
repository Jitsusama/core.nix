local aerial = require('aerial')

-- Configure aerial for automatic code structure visualization
aerial.setup({
  -- Priority list of backends to use
  backends = { 'treesitter', 'lsp', 'markdown', 'man' },

  -- Layout and sizing
  layout = {
    -- These control the width of the aerial window
    max_width = { 40, 0.2 }, -- 40 columns or 20% of window
    width = nil, -- Use default width
    min_width = 20,

    -- Position of the aerial window
    default_direction = 'prefer_right',
    placement = 'window', -- 'window' or 'edge'

    -- Preserve window size when aerial is closed
    preserve_equality = false,
  },

  -- Keymaps in aerial window (set to false to remove default bindings)
  keymaps = {
    ['?'] = 'actions.show_help',
    ['g?'] = 'actions.show_help',
    ['<CR>'] = 'actions.jump',
    ['<2-LeftMouse>'] = 'actions.jump',
    ['<C-v>'] = 'actions.jump_vsplit',
    ['<C-x>'] = 'actions.jump_split',
    ['p'] = 'actions.scroll',
    ['<C-j>'] = 'actions.down_and_scroll',
    ['<C-k>'] = 'actions.up_and_scroll',
    ['{'] = 'actions.prev',
    ['}'] = 'actions.next',
    ['[['] = 'actions.prev_up',
    [']]'] = 'actions.next_up',
    ['q'] = 'actions.close',
    ['o'] = 'actions.tree_toggle',
    ['za'] = 'actions.tree_toggle',
    ['O'] = 'actions.tree_toggle_recursive',
    ['zA'] = 'actions.tree_toggle_recursive',
    ['l'] = 'actions.tree_open',
    ['zo'] = 'actions.tree_open',
    ['L'] = 'actions.tree_open_recursive',
    ['zO'] = 'actions.tree_open_recursive',
    ['h'] = 'actions.tree_close',
    ['zc'] = 'actions.tree_close',
    ['H'] = 'actions.tree_close_recursive',
    ['zC'] = 'actions.tree_close_recursive',
    ['zr'] = 'actions.tree_increase_fold_level',
    ['zR'] = 'actions.tree_open_all',
    ['zm'] = 'actions.tree_decrease_fold_level',
    ['zM'] = 'actions.tree_close_all',
    ['zx'] = 'actions.tree_sync_folds',
    ['zX'] = 'actions.tree_sync_folds',
  },

  -- When true, don't load aerial until a command or function is called
  lazy_load = true,

  -- Disable aerial on files with this many lines
  disable_max_lines = 10000,

  -- Disable aerial on files this size or larger (in bytes)
  disable_max_size = 2000000, -- 2MB

  -- A list of all symbols to display. Set to false to display all symbols
  filter_kind = {
    'Class',
    'Constructor',
    'Enum',
    'Function',
    'Interface',
    'Module',
    'Method',
    'Struct',
    'Variable',
    'Field',
    'Property',
  },

  -- Highlight settings
  highlight_mode = 'split_width',
  highlight_closest = true,
  highlight_on_hover = false,
  highlight_on_jump = 300,

  -- Icons to use
  icons = {
    Class = '󰠱',
    Constructor = '',
    Enum = '',
    Field = '󰜢',
    Function = '󰊕',
    Interface = '',
    Method = '󰆧',
    Module = '',
    Property = '󰜢',
    Struct = '󰙅',
    Variable = '󰀫',
  },

  -- When you fold code with za, zo, or zc, update the aerial tree
  manage_folds = true,
  link_folds_to_tree = false,
  link_tree_to_folds = true,

  -- Automatically open aerial when entering a supported buffer
  open_automatic = false,

  -- Show box drawing characters for the tree hierarchy
  show_guides = true,

  -- Options for the floating aerial window
  float = {
    border = 'rounded',
    relative = 'cursor',
    max_height = 0.9,
    height = nil,
    min_height = { 8, 0.1 },
  },

  -- Options for the inline winbar display
  winbar = {
    enable = true, -- Enable automatic winbar updates
    show_backdrop = true,

    -- How to format the symbols in the winbar
    format = function(symbols, depth, separator)
      local result = {}
      local last = nil
      for _, symbol in ipairs(symbols) do
        if last then
          table.insert(result, separator)
        end
        table.insert(result, symbol.icon .. ' ' .. symbol.name)
        last = symbol
      end
      return table.concat(result)
    end,
  },

  -- Options for the navigation section of aerial
  nav = {
    border = 'rounded',
    max_height = 0.9,
    min_height = { 10, 0.1 },
    max_width = 0.5,
    min_width = { 0.2, 20 },
    win_opts = {
      cursorline = true,
      winblend = 10,
    },
    -- Jump to symbol in source window when the cursor moves
    autojump = false,
    -- Show a preview of the code in the right column
    preview = false,
    -- Keymaps in the nav window
    keymaps = {
      ['<CR>'] = 'actions.jump',
      ['<2-LeftMouse>'] = 'actions.jump',
      ['<C-v>'] = 'actions.jump_vsplit',
      ['<C-x>'] = 'actions.jump_split',
      ['h'] = 'actions.left',
      ['l'] = 'actions.right',
      ['<C-c>'] = 'actions.close',
    },
  },

  -- Options for treesitter backend
  treesitter = {
    -- How long to wait before updating the symbols
    update_delay = 300,
  },

  -- Options for LSP backend
  lsp = {
    -- How long to wait before updating the symbols
    update_delay = 300,
    -- Set to false to not update the symbols when there are LSP errors
    diagnostics_trigger_update = true,
    -- Set to false to update symbols on LSP diagnostics update
    update_when_errors = true,
  },
})

-- NEW KEYBINDING SYSTEM: Code outline under <leader>co

-- Main outline operations
vim.keymap.set('n', '<leader>coo', '<cmd>AerialToggle<CR>', { desc = 'Code outline' })
vim.keymap.set('n', '<leader>coO', '<cmd>AerialOpen<CR>', { desc = 'Code Outline (focus)' })
vim.keymap.set('n', '<leader>con', '<cmd>AerialNext<CR>', { desc = 'Code outline next' })
vim.keymap.set('n', '<leader>cop', '<cmd>AerialPrev<CR>', { desc = 'Code outline previous' })
vim.keymap.set('n', '<leader>coN', '<cmd>AerialNavToggle<CR>', { desc = 'Code outline Navigate' })

-- Symbol search with telescope integration
vim.keymap.set('n', '<leader>cos', function()
  local telescope_ok = pcall(require, 'telescope')
  if telescope_ok then
    require('telescope').extensions.aerial.aerial()
  else
    vim.cmd('AerialToggle')
  end
end, { desc = 'Code outline symbols' })

-- SEQUENTIAL NAVIGATION: Next/previous symbols
vim.keymap.set('n', ']s', '<cmd>AerialNext<CR>', { desc = 'Next symbol' })
vim.keymap.set('n', '[s', '<cmd>AerialPrev<CR>', { desc = 'Previous symbol' })

-- Load Telescope extension if available
local telescope_ok = pcall(require, 'telescope')
if telescope_ok then
  require('telescope').load_extension('aerial')
end