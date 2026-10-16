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

-- Source labels
local source_labels = {
  nvim_lsp = '[LSP]',
  buffer = '[Buffer]',
  path = '[Path]',
  cmdline = '[CMD]',
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
  window = {
    completion = cmp.config.window.bordered(),
    documentation = cmp.config.window.bordered(),
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
  formatting = {
    format = function(entry, vim_item)
      vim_item.kind = string.format('%s %s', kind_icons[vim_item.kind], vim_item.kind)
      vim_item.menu = source_labels[entry.source.name]
      return vim_item
    end,
  },
})

-- Search completion (/ and ?)
cmp.setup.cmdline({ '/', '?' }, {
  mapping = cmp.mapping.preset.cmdline(),
  sources = {
    { name = 'buffer' },
  },
})

-- Command line completion (:)
cmp.setup.cmdline(':', {
  mapping = cmp.mapping.preset.cmdline(),
  sources = cmp.config.sources({
    { name = 'path' },
  }, {
    { name = 'cmdline' },
  }),
})
