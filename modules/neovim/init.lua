-- Core Vim settings
vim.opt.number = true
vim.opt.relativenumber = true
vim.opt.signcolumn = 'yes'
-- The line's own number, in strong, stands in the column with the counts
-- above and below it: Neovim pushes it left, even through the column's own
-- number item, so the column says the number itself; and Neovim marks it
-- only with 'cursorline', here on the number alone. A wrapped line's
-- rows after its first, and a window without numbers, show none.
vim.opt.statuscolumn =
  '%s%=%{v:virtnum ? "" : &rnu && v:relnum ? v:relnum .. " " : &nu || &rnu ? v:lnum .. " " : ""}'
vim.opt.cursorline = true
vim.opt.cursorlineopt = 'number'
-- Nothing drawn where there's nothing: no tildes past the end, no fill
-- after a fold or in a diff's gaps, and the more mark, not @@@, where a
-- long line runs off the foot of a window.
vim.opt.fillchars = { eob = ' ', fold = ' ', diff = ' ', lastline = '…' }
vim.opt.updatetime = 300 -- Faster cursor hold events (default is 4000ms)
vim.opt.scrolloff = 4 -- Lines of context when scrolling
vim.opt.sidescrolloff = 8 -- Columns of context

-- Terminal title settings
vim.opt.title = true
vim.opt.titlestring = '📝 %f'

-- Set leader key (needs to be in init.lua for other plugins to use)
vim.g.mapleader = ' '
vim.g.maplocalleader = ' '

-- Cursor: a calm blink, block in normal, bar in insert. A cursor blinks
-- only when blinkon and blinkoff are both non-zero; the actual cadence is
-- kitty's to own. This is a taste choice, not a fix: drop the blink
-- markers (or this whole line) for a steady block instead.
vim.opt.guicursor = table.concat({
  'n-v-c-sm:block-blinkwait700-blinkon400-blinkoff250',
  'i-ci-ve:ver25-blinkwait700-blinkon400-blinkoff250',
  'r-cr-o:hor20',
  't:block-blinkon500-blinkoff500-TermCursor',
}, ',')

-- UI Polish - Clean and sleek interface
-- A float is set apart by its shade rather than a line round it, with a
-- step of the shade round its words, and nothing behind it shows through.
vim.o.winborder = 'solid'
vim.opt.pumblend = 0
vim.opt.pumheight = 10 -- Maximum popup menu height
vim.opt.showmode = false -- Don't show mode in command line (shown in statusline)
vim.opt.laststatus = 3 -- Global statusline
vim.opt.cmdheight = 1 -- Command line height
vim.opt.ruler = false -- Don't show cursor position in command line

-- Better list characters
vim.opt.list = true
vim.opt.listchars = {
  tab = '  ',
  trail = '·',
  -- The more mark, either way a line runs past the window's edge.
  extends = '…',
  precedes = '…',
  nbsp = '␣',
}

-- Search settings
vim.opt.ignorecase = true
vim.opt.smartcase = true -- Override ignorecase if search contains capitals
vim.opt.inccommand = 'split' -- Preview substitutions live
vim.opt.shortmess:append('S') -- The status line shows the search count instead
vim.opt.showcmdloc = 'statusline' -- Move partial commands to statusline instead of command line
vim.opt.showcmd = false -- Don't show partial commands at all (including selection size)

-- Indentation
vim.opt.expandtab = true
vim.opt.shiftwidth = 2
vim.opt.tabstop = 2
vim.opt.smartindent = true
vim.opt.shiftround = true -- Round indent to multiple of shiftwidth

-- Better splitting
vim.opt.splitbelow = true
vim.opt.splitright = true
vim.opt.splitkeep = 'screen' -- Keep the same relative cursor position

-- File handling
vim.opt.autowrite = true -- Auto save before commands like :next
vim.opt.confirm = true -- Confirm to save changes before exiting modified buffer
vim.opt.undofile = true -- Persistent undo
vim.opt.undolevels = 10000

-- Performance
vim.opt.timeoutlen = 300 -- Faster which-key popup
vim.opt.ttimeoutlen = 10 -- Faster key sequence completion

-- Clipboard
vim.opt.clipboard = vim.env.SSH_TTY and '' or 'unnamedplus' -- System clipboard unless in SSH

-- Better wrapping
vim.opt.wrap = false
vim.opt.linebreak = true -- Wrap at word boundaries when wrap is enabled
vim.opt.breakindent = true -- Preserve indentation in wrapped lines

-- Fold settings
vim.opt.foldlevel = 99
vim.opt.foldlevelstart = 99
vim.opt.foldenable = true
-- A closed fold shows its first line as it reads open, coloured as code,
-- on the fold's shade, rather than a count and a row of dots.
vim.opt.foldtext = ''

-- Session options
vim.opt.sessionoptions =
  { 'buffers', 'curdir', 'tabpages', 'winsize', 'help', 'globals', 'skiprtp', 'folds' }

-- The status line, which holds the window's facts (status.lua).
require('status').setup()

-- How problems look, set as vim.diagnostic first loads, since nothing
-- needs it until something reports one. The sign column marks a problem as
-- the status line counts it, with the theme's marks.
local marks = require('theme').marks
local signs
require('later').when('vim.diagnostic', function(diagnostic)
  signs = {
    text = {
      [diagnostic.severity.ERROR] = marks.failed,
      [diagnostic.severity.WARN] = marks.caution,
      [diagnostic.severity.INFO] = marks.note,
      [diagnostic.severity.HINT] = marks.suggestion,
    },
  }
  diagnostic.config({
    float = {
      source = 'always',
      header = '',
      prefix = '',
    },
    severity_sort = true,
    signs = signs,
    underline = true,
    update_in_insert = false,
    virtual_text = {
      spacing = 4,
      source = 'if_many',
      prefix = '●',
    },
  })
end)

-- Essential Autocmds
local function augroup(name)
  return vim.api.nvim_create_augroup('custom_' .. name, { clear = true })
end

-- Highlight on yank
vim.api.nvim_create_autocmd('TextYankPost', {
  group = augroup('highlight_yank'),
  callback = function()
    vim.highlight.on_yank({ timeout = 200 })
  end,
})

-- Restore cursor position
vim.api.nvim_create_autocmd('BufReadPost', {
  group = augroup('restore_cursor'),
  callback = function(event)
    local exclude = { 'gitcommit', 'gitrebase' }
    local buf = event.buf
    if vim.tbl_contains(exclude, vim.bo[buf].filetype) or vim.b[buf].cursor_restored then
      return
    end
    vim.b[buf].cursor_restored = true
    local mark = vim.api.nvim_buf_get_mark(buf, '"')
    local lcount = vim.api.nvim_buf_line_count(buf)
    if mark[1] > 0 and mark[1] <= lcount then
      pcall(vim.api.nvim_win_set_cursor, 0, mark)
    end
  end,
})

-- Each file Neovim opens goes on the desktop's list of files opened lately,
-- where the desktop keeps one (open-with's recent-files), for the
-- launcher's Recent inside Neovim. It's said as each comes into view, so
-- going back to one already open counts too, and not again while it's the
-- one said last. It's said in the background, so nothing waits on it, and
-- one at a time, since each writes the whole list anew and two at once
-- would lose one: the files opened meanwhile are said once it's done.
local said_last
local saying = false
local waiting = {}
local function say()
  if saying or #waiting == 0 then
    return
  end
  saying = true
  local files = waiting
  waiting = {}
  vim.system(vim.list_extend({ 'recent-files', 'add', 'nvim' }, files), {}, function()
    saying = false
    vim.schedule(say)
  end)
end
vim.api.nvim_create_autocmd('BufWinEnter', {
  group = augroup('recent_files'),
  callback = function(event)
    if vim.bo[event.buf].buftype ~= '' or vim.fn.executable('recent-files') == 0 then
      return
    end
    local path = vim.api.nvim_buf_get_name(event.buf)
    if path == '' or path == said_last or vim.fn.filereadable(path) == 0 then
      return
    end
    said_last = path
    table.insert(waiting, path)
    say()
  end,
})

-- Check if file changed outside of Neovim
vim.api.nvim_create_autocmd({ 'FocusGained', 'TermClose', 'TermLeave' }, {
  group = augroup('checktime'),
  callback = function()
    if vim.o.buftype ~= 'nofile' then
      vim.cmd('checktime')
    end
  end,
})

-- Resize splits when window is resized
vim.api.nvim_create_autocmd('VimResized', {
  group = augroup('resize_splits'),
  callback = function()
    local current_tab = vim.fn.tabpagenr()
    vim.cmd('tabdo wincmd =')
    vim.cmd('tabnext ' .. current_tab)
  end,
})

-- Close certain filetypes with q
vim.api.nvim_create_autocmd('FileType', {
  group = augroup('close_with_q'),
  pattern = {
    'help',
    'lspinfo',
    'man',
    'notify',
    'qf',
    'query',
    'startuptime',
    'tsplayground',
    'checkhealth',
  },
  callback = function(event)
    vim.bo[event.buf].buflisted = false
    vim.keymap.set('n', 'q', '<cmd>close<cr>', { buffer = event.buf, silent = true })
  end,
})

-- Auto create directories when saving
vim.api.nvim_create_autocmd('BufWritePre', {
  group = augroup('auto_create_dir'),
  callback = function(event)
    if event.match:match('^%w%w+:[\\/][\\/]') then
      return
    end
    local file = vim.uv.fs_realpath(event.match) or event.match
    vim.fn.mkdir(vim.fn.fnamemodify(file, ':p:h'), 'p')
  end,
})

-- Spelling is checked against English and the names English doesn't know
-- (names.add), so Neovim, niri and a YubiKey aren't marked as mistakes.
vim.opt.spelllang = { 'en', 'names' }

-- Wrap and spell check in text filetypes
vim.api.nvim_create_autocmd('FileType', {
  group = augroup('wrap_spell'),
  pattern = { 'gitcommit', 'markdown', 'text', 'tex' },
  callback = function()
    vim.opt_local.wrap = true
    vim.opt_local.spell = true
  end,
})

-- Disable line numbers in terminal
vim.api.nvim_create_autocmd('TermOpen', {
  group = augroup('terminal_setup'),
  callback = function()
    vim.opt_local.number = false
    vim.opt_local.relativenumber = false
    vim.opt_local.signcolumn = 'no'
    vim.opt_local.statuscolumn = ''
  end,
})

-- Every key is in keys.nix, which lua/keys.lua binds. The functions too
-- long to say there are here, under the names it gives them; plugins'
-- files register their own.
local keys = require('keys')

keys.act('split.close', function()
  -- The last split can't close, so it's the file that goes.
  if not pcall(vim.cmd, 'confirm close') then
    Snacks.bufdelete()
  end
end)

keys.act('file.last', function()
  if not pcall(vim.cmd.buffer, '#') then
    vim.notify('No file was open before this one', vim.log.levels.INFO)
  end
end)

keys.act('quickfix.toggle', function()
  for _, win in pairs(vim.fn.getwininfo()) do
    if win.quickfix == 1 then
      vim.cmd('cclose')
      return
    end
  end
  vim.cmd('copen')
end)

keys.act('look.background', function()
  if vim.o.background == 'dark' then
    vim.o.background = 'light'
    vim.notify('Background: light', vim.log.levels.INFO)
  else
    vim.o.background = 'dark'
    vim.notify('Background: dark', vim.log.levels.INFO)
  end
end)

-- Line numbers go from none, to counted from here, to counted from the top.
keys.act('look.numbers', function()
  if not vim.wo.number and not vim.wo.relativenumber then
    -- Currently no numbers -> enable relative
    vim.wo.number = true
    vim.wo.relativenumber = true
    vim.notify('Line numbers: relative', vim.log.levels.INFO)
  elseif vim.wo.relativenumber then
    -- Currently relative -> switch to absolute
    vim.wo.relativenumber = false
    vim.notify('Line numbers: absolute', vim.log.levels.INFO)
  else
    -- Currently absolute -> disable all
    vim.wo.number = false
    vim.notify('Line numbers: disabled', vim.log.levels.INFO)
  end
end)

keys.act('look.wrap', function()
  vim.wo.wrap = not vim.wo.wrap
  if vim.wo.wrap then
    vim.notify('Line wrap: enabled', vim.log.levels.INFO)
  else
    vim.notify('Line wrap: disabled', vim.log.levels.INFO)
  end
end)

keys.act('tab.find', function()
  local tabs = {}
  for i = 1, vim.fn.tabpagenr('$') do
    local tabid = vim.fn.tabpagebuflist(i)[1]
    local name = vim.fn.bufname(tabid)
    if name == '' then
      name = '[No Name]'
    else
      name = vim.fn.fnamemodify(name, ':t') -- Just filename
    end
    table.insert(tabs, string.format('%d: %s', i, name))
  end

  vim.ui.select(tabs, {
    prompt = 'Select tab:',
    format_item = function(item)
      return item
    end,
  }, function(choice)
    if choice then
      local tab_num = choice:match('^(%d+):')
      if tab_num then
        vim.cmd('tabnext ' .. tab_num)
      end
    end
  end)
end)

keys.act('github.copyPullRequest', function()
  local result = vim.fn.system('gh pr view --json url -q .url 2>/dev/null')
  if vim.v.shell_error == 0 then
    local url = result:gsub('\n', '')
    vim.fn.setreg('+', url)
    print('Copied PR URL: ' .. url)
  else
    print('No PR found for current branch')
  end
end)

keys.act('github.openPullRequest', function()
  vim.cmd('!gh pr view --web 2>/dev/null || echo "No PR found for current branch"')
end)

keys.act('github.copyRepository', function()
  local url = vim.fn.system('gh repo view --json url -q .url'):gsub('\n', '')
  vim.fn.setreg('+', url)
  print('Copied repo URL: ' .. url)
end)

keys.act('github.gistFile', function()
  local filename = vim.fn.expand('%:t')
  if filename == '' then
    filename = 'untitled'
  end
  vim.cmd('write !gh gist create -f ' .. vim.fn.shellescape(filename))
end)

keys.act('search.replace', function()
  -- Simple search and replace using built-in substitution
  local search_term = vim.fn.input('Search for: ')
  if search_term ~= '' then
    local replace_term = vim.fn.input('Replace with: ')
    local confirm = vim.fn.input('Replace all occurrences? (y/n): ')
    if confirm:lower() == 'y' then
      vim.cmd(
        string.format(
          '%%s/%s/%s/g',
          vim.fn.escape(search_term, '/'),
          vim.fn.escape(replace_term, '/')
        )
      )
    else
      vim.cmd(
        string.format(
          '%%s/%s/%s/gc',
          vim.fn.escape(search_term, '/'),
          vim.fn.escape(replace_term, '/')
        )
      )
    end
  end
end)

keys.act('errors.toggle', function()
  if vim.diagnostic.is_enabled() then
    vim.diagnostic.enable(false)
    vim.notify('Diagnostics disabled', vim.log.levels.INFO)
  else
    vim.diagnostic.enable()
    vim.notify('Diagnostics enabled', vim.log.levels.INFO)
  end
end)

keys.act('errors.toggleBeside', function()
  local current = vim.diagnostic.config().virtual_text
  vim.diagnostic.config({ virtual_text = not current })
  vim.notify('Virtual text ' .. (current and 'disabled' or 'enabled'), vim.log.levels.INFO)
end)

keys.act('errors.toggleSigns', function()
  local current = vim.diagnostic.config().signs
  vim.diagnostic.config({ signs = not current and signs })
  vim.notify('Diagnostic signs ' .. (current and 'disabled' or 'enabled'), vim.log.levels.INFO)
end)

-- Each step to a problem shows it in full where it lands.
local function show(diagnostic)
  if diagnostic then
    vim.diagnostic.open_float()
  end
end

-- The file's problems from the top down.
local function problems()
  local all = vim.diagnostic.get(0)
  table.sort(all, function(a, b)
    return a.lnum < b.lnum or (a.lnum == b.lnum and a.col < b.col)
  end)
  return all
end

keys.act('errors.before', function()
  vim.diagnostic.jump({ count = -1, on_jump = show })
end)

keys.act('errors.after', function()
  vim.diagnostic.jump({ count = 1, on_jump = show })
end)

keys.act('errors.first', function()
  local all = problems()
  if all[1] then
    vim.diagnostic.jump({ diagnostic = all[1], on_jump = show })
  end
end)

keys.act('errors.last', function()
  local all = problems()
  if all[#all] then
    vim.diagnostic.jump({ diagnostic = all[#all], on_jump = show })
  end
end)
