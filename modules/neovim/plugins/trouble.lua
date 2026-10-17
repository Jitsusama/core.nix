-- Trouble lists problems and the quickfix list, as Space e q asks, and its
-- keys inside are in keys.nix too.
local trouble = require('trouble')
local keys = require('keys')

-- The two keys.nix names that aren't Trouble's own actions.
local own = {
  ['only.file'] = function(view)
    view:filter({ buf = 0 }, { toggle = true })
  end,
  ['only.worse'] = function(view)
    local filter = view:get_filter('severity')
    local severity = ((filter and filter.filter.severity or 0) + 1) % 5
    view:filter({ severity = severity }, {
      id = 'severity',
      template = '{hl:Title}Filter:{hl} {severity}',
      del = severity == 0,
    })
  end,
}

-- Trouble adds its own keys to whatever it's given, so each one it has is
-- turned off before the ones from keys.nix go on. A double click jumps too.
local theirs = {
  '?',
  'r',
  'R',
  'q',
  'o',
  '<esc>',
  '<cr>',
  '<2-leftmouse>',
  '<c-s>',
  '<c-v>',
  '}',
  ']]',
  '{',
  '[[',
  'dd',
  'd',
  'i',
  'p',
  'P',
  'zo',
  'zO',
  'zc',
  'zC',
  'za',
  'zA',
  'zm',
  'zM',
  'zr',
  'zR',
  'zx',
  'zX',
  'zn',
  'zN',
  'zi',
  'gb',
  's',
}
local window = {}
for _, lhs in ipairs(theirs) do
  window[lhs] = false
end
window = vim.tbl_extend(
  'force',
  window,
  keys.window('problems', function(key)
    -- Trouble trades an action it keeps as a table for that table, words
    -- and all, so those are taken out of it here.
    local action = own[key.plugin] or require('trouble.config.actions')[key.plugin]
    if type(action) == 'table' then
      action = action.action
    end
    return { action = action or key.plugin, mode = key.modes, desc = key.desc }
  end)
)
window['<2-LeftMouse>'] = 'jump'

trouble.setup({
  -- The list is opened to be worked through, so it takes the focus.
  focus = true,
  keys = window,
  -- A flat list under each file, told from where Neovim is as the finder
  -- tells it, rather than the whole path above it and lines between.
  indent_guides = false,
  modes = {
    diagnostics = {
      groups = { { 'filename', format = '{file_icon} {filename} {count}' } },
    },
  },
})
