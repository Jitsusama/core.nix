local wrong = {}
vim.wo.wrap = true
vim.api.nvim_win_set_cursor(0, { 61, 0 })
local function column(lnum)
  return vim.api.nvim_eval_statusline(vim.o.statuscolumn, {
    use_statuscol_lnum = lnum,
    highlights = true,
    winid = 0,
  })
end
local above, here, below = column(60), column(61), column(62)
if not here.str:match('^%s*61 $') then
  table.insert(wrong, 'the line here reads "' .. here.str .. '"')
end
if not above.str:match('^%s*1 $') or not below.str:match('^%s*1 $') then
  table.insert(wrong, 'the lines beside it read "' .. above.str .. '" and "' .. below.str .. '"')
end
if #above.str ~= #here.str or #below.str ~= #here.str then
  table.insert(
    wrong,
    'the numbers are not one width: "'
      .. above.str
      .. '", "'
      .. here.str
      .. '", "'
      .. below.str
      .. '"'
  )
end
local marked = false
for _, mark in ipairs(here.highlights) do
  marked = marked or mark.group == 'CursorLineNr'
end
if not marked then
  table.insert(wrong, 'the line here is not marked as the cursor line')
end
vim.wo.relativenumber, vim.wo.number = false, false
if vim.trim(column(60).str) ~= '' then
  table.insert(wrong, 'with numbers off the column says "' .. column(60).str .. '"')
end
io.stdout:write(#wrong == 0 and 'as designed' or table.concat(wrong, '\n'))
vim.cmd('qa!')
