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

-- Flash's jumps for g l and g w (keys.nix).
local keys = require('keys')

keys.act('jump.line', function()
  flash.jump({
    search = { mode = 'search', max_length = 0 },
    label = { after = { 0, 0 } },
    pattern = '^',
  })
end)

keys.act('jump.word', function()
  flash.jump({
    pattern = '.', -- any character
    search = {
      mode = function(str)
        return '\\<' .. str
      end,
    },
  })
end)
