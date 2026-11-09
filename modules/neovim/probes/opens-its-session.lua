-- For neovim-keeps-a-session-of-its-folder: once a session is opened,
-- answers the folder it went to, how many tabs it has, and the names of
-- the files it holds, then leaves.
local names = {}
for _, buffer in ipairs(vim.api.nvim_list_bufs()) do
  if vim.bo[buffer].buflisted and vim.api.nvim_buf_get_name(buffer) ~= '' then
    table.insert(names, vim.fn.fnamemodify(vim.api.nvim_buf_get_name(buffer), ':t'))
  end
end
table.sort(names)
local folder = vim.fn.fnamemodify(vim.fn.getcwd(), ':t')
io.stdout:write(
  table.concat({ folder, tostring(#vim.api.nvim_list_tabpages()), unpack(names) }, ' ')
)
vim.cmd('qa!')
