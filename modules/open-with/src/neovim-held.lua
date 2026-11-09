-- What a running Neovim holds, asked of it over its own msgpack-RPC
-- server by open-now (neovim.rs), for the launcher's Programs › Neovim:
-- its process and the one that started it (the client drawing it in a
-- terminal, which kitty sees in front, from 0.12 on), its folder, its
-- tabs, and each listed buffer that holds a file, never a terminal,
-- help, a plugin's buffer or a folder. Each says which tabs show it, by
-- their number in the tab line, none while it's hidden, whether it's
-- modified, and when it was last used, so the launcher can lead with the
-- file Joel was in. It's sent whole each time
-- rather than kept in Neovim, so any Neovim answers it, the account's
-- config or none.
local tabs = vim.api.nvim_list_tabpages()
local numbers = {}
for index, tab in ipairs(tabs) do
  numbers[tab] = index
end

local files = {}
for _, info in ipairs(vim.fn.getbufinfo({ buflisted = 1 })) do
  local name = info.name
  local file = vim.bo[info.bufnr].buftype == ''
    and name ~= ''
    and not name:find('://', 1, true)
    and vim.fn.isdirectory(name) == 0
  if file then
    local shown = {}
    for _, window in ipairs(info.windows) do
      local number = numbers[vim.api.nvim_win_get_tabpage(window)]
      if number and not vim.tbl_contains(shown, number) then
        table.insert(shown, number)
      end
    end
    table.sort(shown)
    table.insert(files, {
      buffer = info.bufnr,
      path = vim.fn.fnamemodify(name, ':p'),
      modified = info.changed == 1,
      tabs = shown,
      at = info.lastused,
    })
  end
end

return {
  pid = vim.fn.getpid(),
  parent = vim.uv.os_getppid(),
  folder = vim.fn.getcwd(),
  tab = numbers[vim.api.nvim_get_current_tabpage()],
  tabs = #tabs,
  current = vim.api.nvim_get_current_buf(),
  files = files,
}
