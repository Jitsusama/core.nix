-- Tree-sitter for every buffer whose language has a parser: its highlights,
-- indent and folds, and text objects to select and move by. nvim-treesitter
-- now only installs parsers and queries, and each buffer starts Tree-sitter
-- itself; Neovim selects nodes on its own (an and in in visual mode).

require('nvim-treesitter').setup({})

vim.api.nvim_create_autocmd('FileType', {
  group = vim.api.nvim_create_augroup('treesitter', { clear = true }),
  callback = function(args)
    -- A language with no parser keeps Vim's own syntax.
    if not pcall(vim.treesitter.start, args.buf) then
      return
    end
    vim.bo[args.buf].indentexpr = "v:lua.require'nvim-treesitter'.indentexpr()"
    vim.wo.foldmethod = 'expr'
    vim.wo.foldexpr = 'v:lua.vim.treesitter.foldexpr()'
  end,
})
-- Folds are there to close, not closed to start with.
vim.opt.foldenable = false

-- Selecting by node, from the cursor outwards and back in.
vim.keymap.set('n', '<leader>css', 'van', { remap = true, desc = 'Start treesitter selection' })
vim.keymap.set('x', '<leader>csi', 'an', { remap = true, desc = 'Expand to parent node' })
vim.keymap.set('x', '<leader>csd', 'in', { remap = true, desc = 'Shrink selection' })

require('nvim-treesitter-textobjects').setup({
  select = { lookahead = true },
  move = { set_jumps = true },
})

local select = require('nvim-treesitter-textobjects.select')
for keys, capture in pairs({
  af = '@function.outer',
  ['if'] = '@function.inner',
  ac = '@class.outer',
  ic = '@class.inner',
  al = '@loop.outer',
  il = '@loop.inner',
  aa = '@parameter.outer',
  ia = '@parameter.inner',
}) do
  vim.keymap.set({ 'x', 'o' }, keys, function()
    select.select_textobject(capture, 'textobjects')
  end, { desc = capture })
end

local move = require('nvim-treesitter-textobjects.move')
for keys, step in pairs({
  [']f'] = { 'goto_next_start', '@function.outer' },
  [']c'] = { 'goto_next_start', '@class.outer' },
  [']C'] = { 'goto_next_start', '@comment.outer' },
  [']F'] = { 'goto_next_end', '@function.outer' },
  ['[f'] = { 'goto_previous_start', '@function.outer' },
  ['[c'] = { 'goto_previous_start', '@class.outer' },
  ['[C'] = { 'goto_previous_start', '@comment.outer' },
  ['[F'] = { 'goto_previous_end', '@function.outer' },
}) do
  vim.keymap.set({ 'n', 'x', 'o' }, keys, function()
    move[step[1]](step[2], 'textobjects')
  end, { desc = step[2] })
end
