local flash = require('flash')

-- Configure Flash.nvim for efficient navigation
flash.setup({
  -- Don't override default search behavior
  modes = {
    search = {
      enabled = false, -- Don't override / and ?
    },
    char = {
      enabled = false, -- Don't override f, F, t, T
    },
  },

  -- Label configuration
  labels = 'asdfghjklqwertyuiopzxcvbnm',

  -- Appearance
  label = {
    uppercase = false,
    after = false, -- Show label after match
    before = { 0, 0 }, -- Show label at beginning of match
    style = 'overlay', -- overlay, inline, eol
    reuse = 'lowercase', -- Reuse lowercase labels
    distance = true, -- Show labels based on distance
  },

  -- Highlight configuration
  highlight = {
    backdrop = true, -- Dim the backdrop
    matches = true, -- Highlight matches
    priority = 5000,
    groups = {
      match = 'FlashMatch',
      current = 'FlashCurrent',
      backdrop = 'FlashBackdrop',
      label = 'FlashLabel',
    },
  },

  -- Jump behavior
  jump = {
    jumplist = true, -- Save jumps to jumplist
    pos = 'start', -- Jump to start of match
    history = false, -- Don't add to search history
    register = false, -- Don't save to register
    nohlsearch = false, -- Don't clear hlsearch
    autojump = false, -- Don't auto-jump to single match
  },

  -- Prompt configuration
  prompt = {
    enabled = true,
    prefix = { { '⚡', 'FlashPromptIcon' } },
    win_config = {
      relative = 'editor',
      width = 1,
      height = 1,
      row = -1,
      col = 0,
      zindex = 1000,
    },
  },
})

-- NEW KEYBINDING SYSTEM: Flash uses g prefix for "go to" navigation

-- Main Flash navigation
vim.keymap.set({ 'n', 'x', 'o' }, 'gs', function()
  flash.jump()
end, { desc = 'Go search (Flash)' })

vim.keymap.set({ 'n', 'x', 'o' }, 'gS', function()
  flash.treesitter()
end, { desc = 'Go structural (Flash)' })

-- Additional Flash modes
vim.keymap.set('n', 'gl', function()
  flash.jump({
    search = { mode = 'search', max_length = 0 },
    label = { after = { 0, 0 } },
    pattern = '^',
  })
end, { desc = 'Go to line (Flash)' })

vim.keymap.set('n', 'gw', function()
  flash.jump({
    pattern = '.', -- any character
    search = {
      mode = function(str)
        return '\\<' .. str
      end,
    },
  })
end, { desc = 'Go to word (Flash)' })

-- Register with which-key if available
local wk_ok, which_key = pcall(require, 'which-key')
if wk_ok then
  which_key.add({
    { 'gs', desc = 'Go search (Flash)', mode = { 'n', 'x', 'o' } },
    { 'gS', desc = 'Go structural (Flash)', mode = { 'n', 'x', 'o' } },
    { 'gl', desc = 'Go to line (Flash)' },
    { 'gw', desc = 'Go to word (Flash)' },
  })
end