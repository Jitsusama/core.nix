-- Keeps a session of the folder Neovim was working in as it's left, so the
-- launcher's Recent inside Neovim can open those files again together, in
-- their tabs and windows (NeovimProgram.qml): one file a folder under
-- stdpath('state')/sessions, written by Neovim's own :mksession with the
-- sessionoptions init.lua sets, and opened again by nvim -S. It's kept only
-- when more than one file was open, so a quick edit never leaves one and
-- never replaces the one a longer stay left; and never while two files are
-- compared, as git's difftool and mergetool open Neovim to, nor after
-- leaving with an error, as :cq does to give up on a commit.
local M = {}

-- Where the sessions are kept.
function M.folder()
  return vim.fs.joinpath(vim.fn.stdpath('state'), 'sessions')
end

-- A folder's session's file name: its whole path with each byte but a
-- letter, a digit, a dot, a dash or an underscore written as = and its
-- two hex digits, so two folders of one name never share one, and the
-- launcher reads the folder back from it whole (Neovim.js's folderOf).
-- Not %, as a URL writes them: nvim -S reads its file name as a command
-- line does, where % is the current file, # the one before and a space
-- ends the name, and none of those is left in a name written this way.
function M.name(folder)
  local escaped = folder:gsub('[^%w._-]', function(byte)
    return ('=%02X'):format(byte:byte())
  end)
  return escaped .. '.vim'
end

-- How many listed buffers hold a file, never a terminal, help, a plugin's
-- buffer or a folder, as the launcher counts what's open
-- (../open-with/src/neovim-held.lua).
function M.files()
  local count = 0
  for _, buffer in ipairs(vim.api.nvim_list_bufs()) do
    local name = vim.api.nvim_buf_get_name(buffer)
    if
      vim.bo[buffer].buflisted
      and vim.bo[buffer].buftype == ''
      and name ~= ''
      and not name:find('://', 1, true)
      and vim.fn.isdirectory(name) == 0
    then
      count = count + 1
    end
  end
  return count
end

local function comparing()
  for _, window in ipairs(vim.api.nvim_list_wins()) do
    if vim.wo[window].diff then
      return true
    end
  end
  return false
end

-- Writes the session of the folder Neovim is in, where it's worth
-- keeping. A failure is passed over, since Neovim is already leaving and
-- nothing it said would be seen.
function M.save()
  local exiting = vim.v.exiting
  if (exiting ~= vim.NIL and exiting ~= 0) or M.files() < 2 or comparing() then
    return
  end
  local folder = M.folder()
  vim.fn.mkdir(folder, 'p')
  local path = vim.fs.joinpath(folder, M.name(vim.fn.getcwd(-1, -1)))
  pcall(vim.cmd, 'mksession! ' .. vim.fn.fnameescape(path))
end

function M.setup()
  vim.api.nvim_create_autocmd('VimLeavePre', {
    group = vim.api.nvim_create_augroup('custom_sessions', { clear = true }),
    callback = M.save,
  })
end

return M
