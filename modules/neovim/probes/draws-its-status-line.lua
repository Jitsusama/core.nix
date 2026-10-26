local status = require('status')
local g = require('theme').glyphs
local wrong = {}
local function plain(pieces)
  local text = {}
  for _, piece in ipairs(pieces) do
    table.insert(text, piece[2])
  end
  return table.concat(text)
end
local function expect(what, columns, left, right)
  local l, r = status.layout(columns)
  l, r = plain(l), plain(r)
  if l ~= left or r ~= right then
    table.insert(wrong, what .. ': "' .. l .. '" and "' .. r .. '"')
  end
end
-- A busy machine can take seconds to draw it; the wait ends once it has.
vim.wait(60000, function()
  return #select(2, status.layout(200)) > 0
end)
expect('at rest', 120, 'project/sub/notes.txt', g.branch .. ' main ' .. g.on)
vim.api.nvim_buf_set_lines(0, -1, -1, false, { 'more' })
expect('changed', 120, 'project/sub/notes.txt ' .. g.on, g.branch .. ' main ' .. g.on)
vim.diagnostic.set(vim.api.nvim_create_namespace('probe'), 0, {
  { lnum = 0, col = 0, severity = vim.diagnostic.severity.ERROR, message = 'e' },
})
expect(
  'an error',
  120,
  'project/sub/notes.txt ' .. g.on,
  g.failed .. ' 1  ' .. g.branch .. ' main ' .. g.on
)
expect(
  'narrow',
  35,
  'project/' .. g.more .. '/notes.txt ' .. g.on,
  g.failed .. ' 1  ' .. g.branch .. ' ' .. g.on .. '  ' .. g.more
)
vim.api.nvim_feedkeys('i', 'n', false)
vim.defer_fn(function()
  expect(
    'typing',
    120,
    'project/sub/notes.txt ' .. g.on .. '  insert',
    g.failed .. ' 1  ' .. g.branch .. ' main ' .. g.on
  )
  vim.cmd('stopinsert')
  vim.defer_fn(function()
    -- The file as fugitive opens it in the index: told from its
    -- root as the file is, the version after it, git's state its
    -- folder's.
    vim.cmd('enew')
    vim.bo.buftype = 'nowrite'
    vim.api.nvim_buf_set_name(
      0,
      'fugitive://' .. vim.fn.getcwd() .. '/project/.git//0/sub/notes.txt'
    )
    expect('a version', 120, 'project/sub/notes.txt  index', g.branch .. ' main ' .. g.on)
    -- A terminal by its program, not its whole term:// address.
    vim.cmd('terminal sleep 5')
    expect('a terminal', 120, 'terminal sleep 5', '')
    io.stdout:write(#wrong == 0 and 'as designed' or table.concat(wrong, '\n'))
    vim.cmd('qa!')
  end, 100)
end, 200)
