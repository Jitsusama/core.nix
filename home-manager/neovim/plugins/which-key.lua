-- Load which-key with error handling
local ok, which_key = pcall(require, 'which-key')
if not ok then
  vim.notify('Failed to load which-key', vim.log.levels.ERROR)
  return
end

-- Icon definitions using Unicode characters
local icons = {
  -- Primary domain icons
  buffer = '🔲',
  code = '💻',
  file = '📁',
  git = '🌿',
  search = '🔍',
  test = '🧪',

  -- Supporting domain icons
  introspect = '🔍',
  project = '📦',
  tab = '📑',
  ui = '🎨',
  window = '🪟',
  error = '❌',

  -- Sub-domain icons
  branch = '🌳',
  commit = '📝',
  github = '🐙',
  network = '🌐',
  stage = '📤',
  working = '💼',
  stash = '📥',
  merge = '🔀',

  -- Action type icons
  picker = '🔎',
  toggle = '🔄',
  action = '⚡',
  navigate = '🧭',

  -- Special icons
  vim = '📜',
  terminal = '💻',
  help = '❓',
  settings = '⚙️',
}

-- Configure which-key
which_key.setup({
  preset = 'modern',
  delay = 200, -- in ms
  show_help = false,
  -- Filter to only show mappings with descriptions
  filter = function(mapping)
    return mapping.desc and mapping.desc ~= ''
  end,
  plugins = {
    marks = true,
    registers = true,
    spelling = {
      enabled = true,
      suggestions = 20,
    },
    presets = {
      operators = false,
      motions = false,
      text_objects = false,
      windows = true,
      nav = true,
      z = true,
      g = true,
    },
  },
  win = {
    border = 'none',
    no_overlap = false,
    padding = { 1, 2 }, -- Vertical: 1 line, Horizontal: 2 columns
    title = false,
    wo = {
      winblend = 0, -- Solid background to prevent cursor bleed-through
    },
    row = 0.5, -- Center vertically
    col = 0.5, -- Center horizontally
    zindex = 200,
  },
  layout = {
    height = { min = 4, max = 25 },
    width = { min = 20, max = 50 },
    spacing = 1,
    align = 'left',
  },
})

-- Register all keybinding groups
which_key.add({
  -- ========================================
  -- LEADER DOMAINS
  -- ========================================

  -- Primary Work Domains
  { '<leader>b', group = 'buffers', icon = icons.buffer, mode = { 'n', 'v' } },
  { '<leader>c', group = 'code', icon = icons.code, mode = { 'n', 'v' } },
  { '<leader>e', group = 'errors', icon = icons.error, mode = { 'n', 'v' } },
  { '<leader>f', group = 'files', icon = icons.file, mode = { 'n', 'v' } },
  { '<leader>g', group = 'git', icon = icons.git, mode = { 'n', 'v' } },
  { '<leader>s', group = 'search', icon = icons.search, mode = { 'n', 'v' } },
  { '<leader>t', group = 'tests', icon = icons.test, mode = { 'n', 'v' } },

  -- Supporting Domains
  { '<leader>i', group = 'introspect', icon = icons.introspect, mode = { 'n', 'v' } },
  { '<leader>k', group = 'workspace', icon = icons.project, mode = { 'n', 'v' } },
  { '<leader>T', group = 'tabs', icon = icons.tab, mode = { 'n', 'v' } },
  { '<leader>u', group = 'UI/settings', icon = icons.ui, mode = { 'n', 'v' } },
  { '<leader>w', group = 'windows', icon = icons.window, mode = { 'n', 'v' } },

  -- ========================================
  -- CODE SUB-DOMAINS
  -- ========================================
  { '<leader>cc', group = 'calls', icon = icons.navigate, mode = { 'n', 'v' } },
  { '<leader>cl', group = 'lens', icon = icons.action, mode = { 'n', 'v' } },
  { '<leader>co', group = 'outline', icon = icons.navigate, mode = { 'n', 'v' } },
  { '<leader>cr', group = 'refactor', icon = icons.action, mode = { 'n', 'v' } },
  { '<leader>cs', group = 'selection', icon = icons.picker, mode = { 'n', 'v' } },
  { '<leader>ct', group = 'toggles', icon = icons.toggle, mode = { 'n', 'v' } },

  -- ========================================
  -- ERROR SUB-DOMAINS
  -- ========================================
  { '<leader>et', group = 'toggles', icon = icons.toggle, mode = { 'n', 'v' } },

  -- ========================================
  -- GIT SUB-DOMAINS
  -- ========================================
  { '<leader>gb', group = 'branch', icon = icons.branch, mode = { 'n', 'v' } },
  { '<leader>gc', group = 'commit', icon = icons.commit, mode = { 'n', 'v' } },
  { '<leader>gh', group = 'GitHub', icon = icons.github, mode = { 'n', 'v' } },
  { '<leader>gn', group = 'network', icon = icons.network, mode = { 'n', 'v' } },
  { '<leader>gr', group = 'rebase', icon = icons.branch, mode = { 'n', 'v' } },
  { '<leader>gs', group = 'stage', icon = icons.stage, mode = { 'n', 'v' } },
  { '<leader>gt', group = 'stash', icon = icons.stash, mode = { 'n', 'v' } },
  { '<leader>gw', group = 'working', icon = icons.working, mode = { 'n', 'v' } },
  { '<leader>gm', group = 'merge', icon = icons.merge, mode = { 'n', 'v' } },

  -- GitHub sub-groups
  { '<leader>gha', group = 'action', icon = '⚡', mode = { 'n', 'v' } },
  { '<leader>ghb', group = 'buffer', icon = '📋', mode = { 'n', 'v' } },
  { '<leader>ghc', group = 'comment', icon = '💬', mode = { 'n', 'v' } },
  { '<leader>ghf', group = 'file/code', icon = '📄', mode = { 'n', 'v' } },
  { '<leader>ghg', group = 'gist', icon = '📝', mode = { 'n', 'v' } },
  { '<leader>ghi', group = 'issue', icon = '📋', mode = { 'n', 'v' } },
  { '<leader>ghl', group = 'label', icon = '🏷️', mode = { 'n', 'v' } },
  { '<leader>ghm', group = 'merge', icon = '🔀', mode = { 'n', 'v' } },
  { '<leader>ghn', group = 'notification', icon = '🔔', mode = { 'n', 'v' } },
  { '<leader>ghp', group = 'pull request', icon = '🔄', mode = { 'n', 'v' } },
  { '<leader>ghr', group = 'repository', icon = '📚', mode = { 'n', 'v' } },
  { '<leader>ght', group = 'reaction', icon = '😄', mode = { 'n', 'v' } },
  { '<leader>ghT', group = 'thread', icon = '🧵', mode = { 'n', 'v' } },
  { '<leader>ghv', group = 'review', icon = '👁️', mode = { 'n', 'v' } },

  -- ========================================
  -- SPECIAL KEYS (Non-leader patterns)
  -- ========================================

  -- Go-to navigation (g prefix)
  { 'g', group = 'go to', icon = icons.navigate, mode = { 'n', 'v' } },
  { 'gd', desc = 'Go to definition' },
  { 'gD', desc = 'Go to declaration' },
  { 'gi', desc = 'Go to implementation' },
  { 'gr', desc = 'Go to references' },
  { 'gy', desc = 'Go to type definition' },
  { 'gs', desc = 'Go search (Flash)', mode = { 'n', 'x', 'o' } },
  { 'gS', desc = 'Go structural (Flash)', mode = { 'n', 'x', 'o' } },
  { 'gl', desc = 'Go to line (Flash)' },
  { 'gw', desc = 'Go to word (Flash)' },
  { 'ga', desc = 'Go to alternate buffer' },
  { 'gf', desc = 'Go to file under cursor' },

  -- Sequential navigation ([ and ] prefixes)
  { '[', group = 'previous', icon = icons.navigate, mode = { 'n', 'v' } },
  { ']', group = 'next', icon = icons.navigate, mode = { 'n', 'v' } },

  -- Previous navigation
  { '[b', desc = 'Previous buffer' },
  { '[d', desc = 'Previous diagnostic' },
  { '[g', desc = 'Previous git hunk' },
  { '[t', desc = 'Previous test' },
  { '[T', desc = 'Previous tab' },
  { '[w', desc = 'Previous window' },
  { '[f', desc = 'Previous function' },
  { '[c', desc = 'Previous class' },
  { '[C', desc = 'Previous comment' },
  { '[q', desc = 'Previous quickfix' },
  { '[l', desc = 'Previous location' },
  { '[s', desc = 'Previous symbol' },

  -- Next navigation
  { ']b', desc = 'Next buffer' },
  { ']d', desc = 'Next diagnostic' },
  { ']g', desc = 'Next git hunk' },
  { ']t', desc = 'Next test' },
  { ']T', desc = 'Next tab' },
  { ']w', desc = 'Next window' },
  { ']f', desc = 'Next function' },
  { ']c', desc = 'Next class' },
  { ']C', desc = 'Next comment' },
  { ']q', desc = 'Next quickfix' },
  { ']l', desc = 'Next location' },
  { ']s', desc = 'Next symbol' },

  -- ========================================
  -- SPECIAL MAPPINGS
  -- ========================================

  -- Double-leader patterns (pickers)
  { '<leader>bb', desc = 'Buffer browser', icon = icons.picker },
  { '<leader>ff', desc = 'Find files (smart)', icon = icons.picker },
  { '<leader>fo', desc = 'Files open (smart)', icon = icons.picker },
  { '<leader>gg', desc = 'Git status', icon = icons.picker },
  { '<leader>ss', desc = 'Search picker', icon = icons.picker },
  { '<leader>TT', desc = 'Tab picker', icon = icons.picker },
  { '<leader>ww', desc = 'Window picker', icon = icons.picker },

  -- New Tab domain mappings
  { '<leader>Tn', desc = 'Tab new' },
  { '<leader>Tc', desc = 'Tab close' },
  { '<leader>To', desc = 'Tab only' },
  { '<leader>Tf', desc = 'Tab first' },
  { '<leader>Tl', desc = 'Tab last' },

  -- New Window domain mappings
  { '<leader>ws', desc = 'Window split' },
  { '<leader>wv', desc = 'Window vertical' },
  { '<leader>wc', desc = 'Window close' },
  { '<leader>wo', desc = 'Window only' },
  { '<leader>w=', desc = 'Window equalize' },
  { '<leader>w+', desc = 'Window height+' },
  { '<leader>w-', desc = 'Window height-' },
  { '<leader>w>', desc = 'Window width+' },
  { '<leader>w<', desc = 'Window width-' },

  -- New UI domain mappings (migrated from <leader>vu*)
  { '<leader>ub', desc = 'UI background' },
  { '<leader>ul', desc = 'UI line numbers' },
  { '<leader>uw', desc = 'UI wrap' },

  -- Quick access
  { '<leader><leader>', desc = 'Resume last' },
  { '<leader>/', desc = 'Quick search', icon = icons.search },
  {
    '<leader>?',
    function()
      which_key.show({ global = false })
    end,
    desc = 'Buffer keymaps',
    icon = icons.help,
  },

  -- Direct access (no prefix)
  { '<BS>', desc = 'Alternate buffer' },
  { '-', desc = 'Parent directory' },
})

-- Apply custom which-key highlighting to match Telescope aesthetic
-- Using gruvbox colors for consistency
vim.api.nvim_set_hl(0, 'WhichKeyNormal', { bg = '#282828' }) -- Same as Telescope results
vim.api.nvim_set_hl(0, 'WhichKeyBorder', { fg = '#282828', bg = '#282828' }) -- Invisible border
vim.api.nvim_set_hl(0, 'WhichKeyGroup', { fg = '#d79921', bold = true, underline = false }) -- Yellow like Telescope caret
vim.api.nvim_set_hl(0, 'WhichKeyDesc', { fg = '#ebdbb2' }) -- Light text for descriptions
vim.api.nvim_set_hl(0, 'WhichKeySeparator', { fg = '#504945' }) -- Subtle separator
vim.api.nvim_set_hl(0, 'WhichKeyValue', { fg = '#83a598' }) -- Blue for values
vim.api.nvim_set_hl(0, 'WhichKeyFloat', { bg = '#282828' }) -- Float window background
vim.api.nvim_set_hl(0, 'WhichKeyIcon', { fg = '#d79921', underline = false }) -- Icons without underline

-- Style the footer/hint text consistently
vim.api.nvim_set_hl(0, 'WhichKeyFooter', { fg = '#7c6f64', bg = '#282828' }) -- Muted gray for footer

-- Style the breadcrumb/title line with visual separation
vim.api.nvim_set_hl(0, 'WhichKeyTitle', { fg = '#7c6f64', bg = '#1d2021', bold = true }) -- Darker bg for breadcrumb

-- Optional: Add a command to manually trigger which-key
vim.api.nvim_create_user_command('WhichKey', function()
  which_key.show()
end, { desc = 'Show which-key popup' })

-- Command palette binding
vim.keymap.set('n', '<C-p>', function()
  which_key.show()
end, { desc = 'Command palette' })
