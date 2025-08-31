local cmp = require('cmp')

-- Helper function to check if cursor has text before it
local function has_words_before()
  unpack = unpack or table.unpack
  local line, col = unpack(vim.api.nvim_win_get_cursor(0))

  if col == 0 then
    return false
  end

  local current_line = vim.api.nvim_buf_get_lines(0, line - 1, line, true)[1]
  local char_before_cursor = current_line:sub(col, col)

  return char_before_cursor:match("%s") == nil
end

-- Kind icons for completion items
local kind_icons = {
  Text = "󰉿",
  Method = "󰆧",
  Function = "󰊕",
  Constructor = "",
  Field = "󰜢",
  Variable = "󰀫",
  Class = "󰠱",
  Interface = "",
  Module = "",
  Property = "󰜢",
  Unit = "󰑭",
  Value = "󰎠",
  Enum = "",
  Keyword = "󰌋",
  Snippet = "",
  Color = "󰏘",
  File = "󰈙",
  Reference = "󰈇",
  Folder = "󰉋",
  EnumMember = "",
  Constant = "󰏿",
  Struct = "󰙅",
  Event = "",
  Operator = "󰆕",
  TypeParameter = "",
}

-- Source labels
local source_labels = {
  nvim_lsp = "[LSP]",
  buffer = "[Buffer]",
  path = "[Path]",
  cmdline = "[CMD]",
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
  mapping = cmp.mapping.preset.insert({
    -- Documentation scrolling
    ['<C-b>'] = cmp.mapping.scroll_docs(-4),
    ['<C-f>'] = cmp.mapping.scroll_docs(4),

    -- Completion control
    ['<C-Space>'] = cmp.mapping.complete(),
    ['<C-e>'] = cmp.mapping.abort(),
    ['<C-y>'] = cmp.mapping.confirm({ select = true }),

    -- Smart Tab navigation (primary completion interface)
    ['<Tab>'] = cmp.mapping(tab_complete, { 'i', 's' }),
    ['<S-Tab>'] = cmp.mapping(shift_tab_complete, { 'i', 's' }),
    
    -- Literal tab insertion
    ['<C-Tab>'] = cmp.mapping(function()
      vim.api.nvim_put({'\t'}, 'c', false, true)
    end, { 'i' }),
  }),
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
    end
  },
})

-- Search completion (/ and ?)
cmp.setup.cmdline({ '/', '?' }, {
  mapping = cmp.mapping.preset.cmdline(),
  sources = {
    { name = 'buffer' }
  }
})

-- Command line completion (:)
cmp.setup.cmdline(':', {
  mapping = cmp.mapping.preset.cmdline(),
  sources = cmp.config.sources({
    { name = 'path' }
  }, {
    { name = 'cmdline' }
  })
})
