-- which-key shows what can follow a key when you pause after it, and Space ?
-- shows every key. What each key does is said once, in keys.nix; this file
-- only names the groups the keys gather in.
local ok, which_key = pcall(require, 'which-key')
if not ok then
  vim.notify('Failed to load which-key', vim.log.levels.ERROR)
  return
end

which_key.setup({
  preset = 'modern',
  delay = 200, -- in ms
  show_help = false,
  -- Only keys that say what they do.
  filter = function(mapping)
    return mapping.desc and mapping.desc ~= ''
  end,
  -- Words, not pictures, in the list, each after its key with nothing
  -- between: the key in the accent and its words beside it read as a pair.
  icons = { mappings = false, separator = '' },
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

local function group(prefix, name)
  return { prefix, group = name, mode = { 'n', 'x' } }
end

which_key.add({
  group('<leader>b', 'open files'),
  group('<leader>c', 'code'),
  group('<leader>cc', 'calls'),
  group('<leader>cl', 'lens'),
  group('<leader>co', 'outline'),
  group('<leader>cr', 'refactor'),
  group('<leader>cs', 'selection'),
  group('<leader>ct', 'toggles'),
  group('<leader>e', 'problems'),
  group('<leader>et', 'toggles'),
  group('<leader>f', 'files'),
  group('<leader>g', 'git'),
  group('<leader>ga', 'stash'),
  group('<leader>gb', 'branches'),
  group('<leader>gc', 'commits'),
  group('<leader>gct', 'toggles'),
  group('<leader>gn', 'remote'),
  group('<leader>gs', 'stage'),
  group('<leader>gt', 'tags'),
  group('<leader>gw', 'working changes'),
  group('<leader>gh', 'GitHub'),
  group('<leader>ghf', 'file links'),
  group('<leader>ghg', 'gists'),
  group('<leader>ghi', 'issues'),
  group('<leader>ghn', 'notifications'),
  group('<leader>ghp', 'pull requests'),
  group('<leader>ghr', 'repository'),
  group('<leader>i', "Neovim's insides"),
  group('<leader>k', 'language server folders'),
  group('<leader>s', 'search'),
  group('<leader>t', 'tests'),
  group('<leader>T', 'tabs'),
  group('<leader>u', 'looks'),
  group('[', 'before'),
  group(']', 'after'),
  -- Octo's own keys, in its issue and pull request buffers.
  group('<leader>gha', 'people'),
  group('<leader>ghb', 'this issue or pull request'),
  group('<leader>ghc', 'comments'),
  group('<leader>ghl', 'labels'),
  group('<leader>ghm', 'merge'),
  group('<leader>ght', 'reactions'),
  group('<leader>ghT', 'threads'),
  group('<leader>ghv', 'review'),
})

vim.api.nvim_create_user_command('WhichKey', function()
  which_key.show()
end, { desc = 'Show which-key popup' })
