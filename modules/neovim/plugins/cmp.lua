-- cmp suggests as code is typed, with the keys for its suggestions from
-- keys.nix.
local cmp = require('cmp')
local keys = require('keys')

-- Helper function to check if cursor has text before it
local function has_words_before()
  unpack = unpack or table.unpack
  local line, col = unpack(vim.api.nvim_win_get_cursor(0))

  if col == 0 then
    return false
  end

  local current_line = vim.api.nvim_buf_get_lines(0, line - 1, line, true)[1]
  local char_before_cursor = current_line:sub(col, col)

  return char_before_cursor:match('%s') == nil
end

-- Kind icons for completion items
local kind_icons = {
  Text = '󰉿',
  Method = '󰆧',
  Function = '󰊕',
  Constructor = '',
  Field = '󰜢',
  Variable = '󰀫',
  Class = '󰠱',
  Interface = '',
  Module = '',
  Property = '󰜢',
  Unit = '󰑭',
  Value = '󰎠',
  Enum = '',
  Keyword = '󰌋',
  Snippet = '',
  Color = '󰏘',
  File = '󰈙',
  Reference = '󰈇',
  Folder = '󰉋',
  EnumMember = '',
  Constant = '󰏿',
  Struct = '󰙅',
  Event = '',
  Operator = '󰆕',
  TypeParameter = '',
}

-- Tab completion behavior
local function tab_complete(fallback)
  if cmp.visible() then
    cmp.select_next_item()
  elseif has_words_before() then
    cmp.complete()
  else
    fallback()
  end
end

-- Shift-Tab completion behavior
local function shift_tab_complete(fallback)
  if cmp.visible() then
    cmp.select_prev_item()
  else
    fallback()
  end
end

-- Main completion setup
cmp.setup({
  -- Set apart by their shade, with a step of it round the words, as every
  -- float is; the chosen one in the selection's colour.
  window = {
    completion = cmp.config.window.bordered({
      border = 'solid',
      winhighlight = 'Normal:Pmenu,FloatBorder:Pmenu,CursorLine:PmenuSel,Search:None',
    }),
    documentation = cmp.config.window.bordered({
      border = 'solid',
      winhighlight = 'Normal:NormalFloat,FloatBorder:FloatBorder,Search:None',
    }),
  },
  -- The suggestions' keys, by keys.nix's names for what they do.
  mapping = keys.window('completion', function(key)
    local does = {
      next = tab_complete,
      before = shift_tab_complete,
      suggest = cmp.mapping.complete(),
      take = cmp.mapping.confirm({ select = true }),
      away = cmp.mapping.abort(),
      ['notes.up'] = cmp.mapping.scroll_docs(-4),
      ['notes.down'] = cmp.mapping.scroll_docs(4),
    }
    return cmp.mapping(does[key.plugin], key.modes)
  end),
  sources = cmp.config.sources({
    { name = 'nvim_lsp' },
  }, {
    { name = 'buffer' },
    { name = 'path' },
  }),
  -- A suggestion is its word and the mark of its kind. The kind's name and
  -- where it came from would say the same on every row. On the command
  -- line every suggestion is of one kind, so it goes unmarked there.
  formatting = {
    format = function(entry, vim_item)
      vim_item.kind = entry.source.name == 'cmdline' and '' or (kind_icons[vim_item.kind] or '')
      vim_item.menu = nil
      return vim_item
    end,
  },
})

-- The command line's keys, by keys.nix's names for what they do, for a
-- search and a command alike. With no suggestions showing, Tab and
-- Shift+Tab suggest, and Ctrl+N and Ctrl+P fall back to the history.
local function choose(step)
  return function()
    if cmp.visible() then
      step()
    else
      cmp.complete()
    end
  end
end

local function or_history(step)
  return function(fallback)
    if cmp.visible() then
      step()
    else
      fallback()
    end
  end
end

local command_line = keys.window('commands', function(key)
  local does = {
    next = choose(cmp.select_next_item),
    before = choose(cmp.select_prev_item),
    later = or_history(cmp.select_next_item),
    earlier = or_history(cmp.select_prev_item),
    take = cmp.mapping.confirm({ select = true }),
    away = cmp.mapping.abort(),
  }
  return cmp.mapping(does[key.plugin], key.modes)
end)

-- Search completion (/ and ?)
cmp.setup.cmdline({ '/', '?' }, {
  mapping = command_line,
  sources = {
    { name = 'buffer' },
  },
})

-- Command line completion (:)
cmp.setup.cmdline(':', {
  mapping = command_line,
  sources = cmp.config.sources({
    { name = 'path' },
  }, {
    { name = 'cmdline' },
  }),
})
