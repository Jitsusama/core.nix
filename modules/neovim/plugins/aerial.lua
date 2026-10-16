-- Aerial shows the outline, and walks it in a float, for Space c o's keys;
-- the keys inside both are in keys.nix.
local aerial = require('aerial')
local keys = require('keys')

-- Aerial adds its own keys to whatever it's given, so each one it has is
-- turned off before the ones from keys.nix go on, each with keys.nix's
-- words for it. A double click jumps too.
local function window(scope, actions, theirs)
  local off = {}
  for _, lhs in ipairs(theirs) do
    off[lhs] = false
  end
  local ours = vim.tbl_extend(
    'force',
    off,
    keys.window(scope, function(key)
      local action = require(actions)[(key.plugin:gsub('^actions%.', ''))]
      return { callback = action.callback, desc = key.desc }
    end)
  )
  ours['<2-LeftMouse>'] = 'actions.jump'
  return ours
end

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

  -- The outline's keys, and none of aerial's own.
  keymaps = window('outline', 'aerial.actions', {
    '?',
    'g?',
    '<CR>',
    '<2-LeftMouse>',
    '<C-v>',
    '<C-s>',
    'p',
    '<C-j>',
    '<C-k>',
    '{',
    '}',
    '[[',
    ']]',
    'q',
    'o',
    'za',
    'O',
    'zA',
    'l',
    'zo',
    'L',
    'zO',
    'h',
    'zc',
    'H',
    'zC',
    'zr',
    'zR',
    'zm',
    'zM',
    'zx',
    'zX',
  }),

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
    border = 'solid',
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
  -- Its columns stand apart by the step of shade round each.
  nav = {
    border = 'solid',
    max_height = 0.9,
    min_height = { 10, 0.1 },
    max_width = 0.5,
    min_width = { 0.2, 20 },
    win_opts = {
      cursorline = true,
      winblend = 0,
    },
    -- Jump to symbol in source window when the cursor moves
    autojump = false,
    -- Show a preview of the code in the right column
    preview = false,
    -- The walking float's keys, and none of aerial's own.
    keymaps = window('walk', 'aerial.nav_actions', {
      '<CR>',
      '<2-LeftMouse>',
      '<C-v>',
      '<C-s>',
      'h',
      'l',
      '<C-c>',
    }),
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
