-- Neovim's status line and tab line, in the prompt's language
-- (docs/design.md, Surfaces). One line along the bottom, on Neovim's own
-- background, holds the window's facts. On the left: where the file is,
-- told from its root as the prompt tells a folder, with a mark while it's
-- unsaved or can't be changed; what git is in the middle of; and the mode,
-- in the accent, whenever the keys aren't going to Normal mode. On the
-- right: what's selected or found, the file's errors and warnings, a
-- language server at work, an agent paired with the editor, and the branch
-- with its marks. Each shows only while it's true, and they give way a
-- step at a time as the window narrows.
--
-- Git's state comes from git-engine, the prompt's own, started once for
-- the editor and asked again when something may have changed it: a buffer
-- entered or written, the editor coming back into focus, a shell command
-- or a terminal left, a change made through fugitive. Nothing polls.
local theme = require('theme')
local where = require('where')
local glyph = theme.marks

local M = {}

local groups = {
  text = 'StatusText',
  muted = 'StatusMuted',
  strong = 'StatusStrong',
  accent = 'StatusAccent',
  alert = 'StatusAlert',
  warning = 'StatusWarning',
  program = 'StatusProgram',
}

local function paint()
  for role, group in pairs(groups) do
    vim.api.nvim_set_hl(0, group, { fg = theme[role] })
  end
end

-- What a line is made of: pieces of text, each in one of the theme's roles.
local function put(pieces, role, text)
  table.insert(pieces, { role, text })
end

local function width(pieces)
  local total = 0
  for _, piece in ipairs(pieces) do
    total = total + vim.api.nvim_strwidth(piece[2])
  end
  return total
end

local function render(pieces)
  local out = {}
  for _, piece in ipairs(pieces) do
    table.insert(out, '%#' .. groups[piece[1]] .. '#' .. piece[2]:gsub('%%', '%%%%'))
  end
  return table.concat(out)
end

-- A name kept to a number of columns by cutting its middle, as the prompt
-- shortens a branch: the start and the end stay, the more mark between.
local function middle(name, keep)
  local chars = vim.fn.strchars(name)
  if chars <= keep then
    return name
  end
  local head = math.floor((keep - 1) / 2)
  local tail = keep - 1 - head
  return vim.fn.strcharpart(name, 0, head) .. glyph.more .. vim.fn.strcharpart(name, chars - tail)
end

-- Git ---------------------------------------------------------------------

local git = { answers = {}, asked = {}, heard = '', next = 0 }

local function first_line(path)
  local file = io.open(path, 'r')
  if not file then
    return nil
  end
  local line = file:read('*l')
  file:close()
  return line
end

local function exists(path)
  return vim.uv.fs_stat(path) ~= nil
end

-- Where a work tree's repository keeps its state: .git itself, or for a
-- linked work tree, the folder its .git file names.
local function git_dir(workdir)
  local dot = workdir .. '/.git'
  local stat = vim.uv.fs_stat(dot)
  if not stat then
    return nil
  elseif stat.type == 'directory' then
    return dot
  end
  local named = (first_line(dot) or ''):match('^gitdir: (.+)$')
  if not named then
    return nil
  end
  return named:sub(1, 1) == '/' and named or workdir .. '/' .. named
end

-- What git is in the middle of, from the files it keeps while it's there,
-- read as the prompt reads them.
local function operation(dir)
  if not dir then
    return nil
  end
  local name, at, of
  if exists(dir .. '/rebase-merge') then
    name = 'rebase'
    at, of = first_line(dir .. '/rebase-merge/msgnum'), first_line(dir .. '/rebase-merge/end')
  elseif exists(dir .. '/rebase-apply') then
    name = exists(dir .. '/rebase-apply/applying') and 'am' or 'rebase'
    at, of = first_line(dir .. '/rebase-apply/next'), first_line(dir .. '/rebase-apply/last')
  elseif exists(dir .. '/MERGE_HEAD') then
    name = 'merge'
  elseif exists(dir .. '/CHERRY_PICK_HEAD') then
    name = 'pick'
  elseif exists(dir .. '/REVERT_HEAD') then
    name = 'revert'
  elseif exists(dir .. '/BISECT_LOG') then
    name = 'bisect'
  else
    return nil
  end
  at, of = tonumber(at), tonumber(of)
  if not (at and of and at >= 1 and of >= 1) then
    at, of = nil, nil
  end
  return { name = name, at = at, of = of }
end

-- One answer, in gitstatusd's fields: the id, whether the folder is in a
-- repository, then the work tree, the commit, the local branch and the
-- rest, the counts from the index's size on.
local function answered(record)
  local fields = vim.split(record, '\31', { plain = true })
  local dir = git.asked[fields[1]]
  git.asked[fields[1]] = nil
  if not dir then
    return
  end
  if fields[2] ~= '1' then
    git.answers[dir] = false
    return
  end
  local count = function(at)
    return tonumber(fields[at]) or 0
  end
  git.answers[dir] = {
    workdir = fields[3],
    branch = fields[5] ~= '' and fields[5] or fields[4]:sub(1, 7),
    unsaved = count(11) + count(12) + count(14) > 0,
    conflicted = count(13) > 0,
    ahead = count(15) > 0,
    op = operation(git_dir(fields[3])),
  }
end

local function heard(data)
  git.heard = git.heard .. data
  while true do
    local stop = git.heard:find('\30', 1, true)
    if not stop then
      break
    end
    answered(git.heard:sub(1, stop - 1))
    git.heard = git.heard:sub(stop + 1)
  end
  vim.cmd.redrawstatus()
end

-- The engine, started the first time it's needed. Like the prompt's, it
-- counts nothing past one, since only whether there are any is drawn.
local function engine()
  if git.job then
    return git.job
  end
  local ok, job = pcall(vim.system, { where.engine, '-s', '1', '-u', '1', '-c', '1', '-d', '1' }, {
    stdin = true,
    stdout = function(_, data)
      if data then
        vim.schedule(function()
          heard(data)
        end)
      end
    end,
  }, function()
    vim.schedule(function()
      git.job, git.heard, git.asked = nil, '', {}
    end)
  end)
  if ok then
    git.job = job
  end
  return git.job
end

-- The folder a git folder belongs to: its parent, or for a worktree's, the
-- folder its gitdir file names.
local function work_tree(gitdir)
  if vim.fs.basename(gitdir) == '.git' then
    return vim.fs.dirname(gitdir)
  end
  local file = first_line(gitdir .. '/gitdir')
  return file and vim.fs.dirname(file)
end

-- Where a file fugitive opened at another version lives in its work tree.
local function versioned(name)
  local gitdir, object, inside = name:match('^fugitive://(.-)//(%w+)/(.*)$')
  local tree = gitdir and work_tree(gitdir)
  return tree and tree .. '/' .. inside, object
end

-- The folder a buffer's git state is asked for: its file's, or for a
-- buffer that's no file, the editor's.
local function folder(buf)
  local name = vim.api.nvim_buf_get_name(buf)
  local dir = name:match('^oil://(.*)$')
  local file = vim.startswith(name, 'fugitive://') and versioned(name)
  if file then
    return vim.fs.dirname(file)
  elseif dir then
    return vim.fs.normalize(dir)
  elseif vim.bo[buf].buftype == '' and name ~= '' then
    return vim.fs.dirname(name)
  end
  return vim.fn.getcwd()
end

-- Asks for a folder's state. Nothing waits for the answer: the line
-- redraws when it comes, and since the branch sits at the right end,
-- nothing else moves when it arrives.
local function ask(dir)
  local job = engine()
  if not job or not exists(dir) then
    return
  end
  git.next = git.next + 1
  local id = tostring(git.next)
  git.asked[id] = dir
  pcall(job.write, job, id .. '\31' .. dir .. '\31\30')
end

-- Where -------------------------------------------------------------------

local home = vim.fs.normalize('~')

-- The nearest folder holding one of the prompt's root markers.
local function root_of(dir)
  while dir do
    for _, marker in ipairs(where.roots) do
      if exists(dir .. '/' .. marker) then
        return dir
      end
    end
    local up = vim.fs.dirname(dir)
    if up == dir then
      return nil
    end
    dir = up
  end
end

local function parts_of(path)
  return vim.split(path, '/', { plain = true, trimempty = true })
end

-- A path told as the prompt tells a folder: from its root, whose name is
-- in text; outside one, from home, or from / in muted.
local function told(path, dir)
  local root = root_of(dir)
  if root and root ~= '/' then
    local parts = parts_of(path:sub(#root + 1))
    table.insert(parts, 1, vim.fs.basename(root))
    return { lead = '', parts = parts, first = 'text' }
  elseif path == home or vim.startswith(path, home .. '/') then
    local parts = parts_of(path:sub(#home + 1))
    table.insert(parts, 1, '~')
    return { lead = '', parts = parts, first = 'text' }
  end
  return { lead = '/', parts = parts_of(path), first = 'muted' }
end

-- The versions fugitive opens a file at, by the stage git gives them; any
-- other is a commit, told by the start of its id.
local stages = { ['0'] = 'index', ['1'] = 'base', ['2'] = 'ours', ['3'] = 'theirs' }

-- A file fugitive opened at another version, told from its root as the
-- file itself is, with the version after it.
local function version_of(name)
  local path, object = versioned(name)
  if not path then
    return nil
  end
  return { path = told(path, vim.fs.dirname(path)), after = stages[object] or object:sub(1, 7) }
end

-- A terminal by the title its program gave it, or by the program's name
-- until it gives one, rather than the whole term:// address.
local function terminal_name(buf, name)
  local title = vim.b[buf].term_title
  if title and title ~= '' and not vim.startswith(title, 'term://') then
    return title
  end
  return vim.fs.basename(name:match(':([^:]*)$') or name)
end

-- What a buffer is: a file or folder told from its root, or a name for
-- the kind of buffer it is, with words before it in muted.
local function place(buf)
  local name = vim.api.nvim_buf_get_name(buf)
  local kind = vim.bo[buf].buftype
  local dir = name:match('^oil://(.*)$')
  local version = vim.startswith(name, 'fugitive://') and version_of(name)
  if version then
    return version
  elseif dir then
    dir = vim.fs.normalize(dir)
    return { path = told(dir, dir) }
  elseif kind == '' and name ~= '' then
    return { path = told(name, vim.fs.dirname(name)), file = true }
  elseif kind == '' then
    return { name = 'unnamed' }
  elseif kind == 'help' then
    return { before = 'help', name = vim.fs.basename(name) }
  elseif kind == 'terminal' then
    return { before = 'terminal', name = terminal_name(buf, name) }
  elseif kind == 'quickfix' then
    local list = vim.fn.getwininfo(vim.api.nvim_get_current_win())[1].loclist == 1
    return { name = list and 'locations' or 'quickfix' }
  end
  local filetype = vim.bo[buf].filetype
  return { name = filetype ~= '' and filetype or 'scratch' }
end

-- A path is kept until the buffer is entered again, renamed or written,
-- since finding its root looks at the disk; any other kind of buffer is
-- named afresh, as a terminal's title and a buffer's kind can change.
local places = {}

local function placed(buf)
  if places[buf] then
    return places[buf]
  end
  local here = place(buf)
  if here.path then
    places[buf] = here
  end
  return here
end

local function put_path(pieces, path, short, name_only)
  local parts, n = path.parts, #path.parts
  if n == 0 then
    put(pieces, 'strong', '/')
  elseif n == 1 then
    put(pieces, 'strong', path.lead .. parts[1])
  elseif name_only then
    put(pieces, 'muted', glyph.more .. '/')
    put(pieces, 'strong', parts[n])
  else
    local between = table.concat(parts, '/', 2, n - 1)
    if short and n > 2 then
      between = glyph.more
    end
    put(pieces, path.first, path.lead .. parts[1])
    put(pieces, 'muted', '/' .. (between ~= '' and between .. '/' or ''))
    put(pieces, 'strong', parts[n])
  end
end

-- The mode, named only when the keys aren't going to Normal mode. The
-- command line says its own, and an operator waits for only a key.
local modes = {
  i = 'insert',
  v = 'visual',
  V = 'visual line',
  ['\22'] = 'visual block',
  s = 'select',
  S = 'select line',
  ['\19'] = 'select block',
  R = 'replace',
  t = 'terminal',
}

-- The right ------------------------------------------------------------

local progress = {}

-- How much is selected, in lines, or in characters within one.
local function selected(mode)
  local first, last = vim.fn.line('v'), vim.fn.line('.')
  local lines = math.abs(last - first) + 1
  if mode == 'v' and lines == 1 then
    local chars = math.abs(vim.fn.charcol('.') - vim.fn.charcol('v')) + 1
    return chars == 1 and '1 character' or chars .. ' characters'
  end
  return lines == 1 and '1 line' or lines .. ' lines'
end

local found = nil

local function count_found()
  found = nil
  if vim.v.hlsearch == 0 then
    return
  end
  local ok, count = pcall(vim.fn.searchcount, { maxcount = 999, timeout = 20 })
  if ok and count.total and count.total > 0 and count.incomplete ~= 1 then
    found = count.current .. ' of ' .. count.total .. (count.total > count.maxcount and '+' or '')
  end
end

-- Asks neovim-pi only once it has loaded, since asking before would load
-- it, or search for it on every draw.
local function paired()
  local pi = package.loaded['neovim-pi']
  return pi ~= nil and pi.is_attached ~= nil and pi.is_attached()
end

-- The steps the line gives way by, in order, each keeping the ones before:
-- the language server's words, the branch's middle, the path's middle, the
-- branch to its glyph, what's under way to its glyph, the path to the
-- file's name, the right side but for the file's problems, and then the
-- name's own middle.
M.steps = {
  'progress',
  'branchshort',
  'pathshort',
  'branchglyph',
  'opshort',
  'nameonly',
  'right',
  'nameshort',
}

-- A float stands above the windows, so while one has the keys the line goes
-- on naming the window under it, and the float's own prompt says where the
-- keys go rather than a mode.
local beneath = nil
local function floating(win)
  return vim.api.nvim_win_get_config(win).relative ~= ''
end
local function shown()
  local win = vim.api.nvim_get_current_win()
  if not floating(win) then
    return vim.api.nvim_win_get_buf(win), false
  end
  if beneath and vim.api.nvim_win_is_valid(beneath) then
    return vim.api.nvim_win_get_buf(beneath), true
  end
  return vim.api.nvim_win_get_buf(win), true
end

local function draw(on)
  local buf, above = shown()
  local left, right = {}, {}
  local lost = false

  local here = placed(buf)
  if here.path then
    if on.nameshort and #here.path.parts > 0 then
      local parts = vim.deepcopy(here.path.parts)
      parts[#parts] = middle(parts[#parts], 16)
      put_path(left, vim.tbl_extend('force', here.path, { parts = parts }), true, true)
    else
      put_path(left, here.path, on.pathshort, on.nameonly)
    end
  else
    if here.before then
      put(left, 'muted', here.before .. ' ')
    end
    put(left, 'strong', on.nameshort and middle(here.name, 16) or here.name)
  end
  if here.after then
    put(left, 'muted', '  ' .. here.after)
  end
  if vim.bo[buf].modified then
    put(left, 'text', ' ' .. glyph.on)
  end
  if here.file and (vim.bo[buf].readonly or not vim.bo[buf].modifiable) then
    put(left, 'muted', ' ' .. glyph.locked)
  end

  local state = git.answers[folder(buf)]
  local op = state and state.op
  if op then
    local hue = state.conflicted and 'alert' or 'warning'
    put(left, 'text', '  ')
    if on.opshort then
      put(left, hue, glyph.active)
    elseif op.of then
      put(left, 'text', op.name .. ' ')
      if op.of > 8 then
        put(left, 'text', op.at .. '/' .. op.of)
      else
        for step = 1, op.of do
          if step < op.at then
            put(left, 'text', glyph.complete)
          elseif step == op.at then
            put(left, hue, glyph.active)
          else
            put(left, 'muted', glyph.pending)
          end
        end
      end
    else
      put(left, hue, glyph.active)
      put(left, 'text', ' ' .. op.name)
    end
  end

  local mode = vim.api.nvim_get_mode().mode
  local named = modes[mode:sub(1, 1)]
  if named and mode ~= 'nt' and not above then
    put(left, 'accent', '  ' .. named)
  end

  local function gap()
    if #right > 0 then
      put(right, 'text', '  ')
    end
  end

  if not on.right then
    if named and mode:match('^[vV\22]') then
      put(right, 'muted', selected(mode:sub(1, 1)))
    elseif found and vim.v.hlsearch == 1 then
      put(right, 'muted', found)
    end
  end

  -- Until something has loaded vim.diagnostic, nothing has reported a
  -- problem, and counting would load it for nothing.
  local diagnostic = package.loaded['vim.diagnostic']
  local problems = diagnostic
      and diagnostic.count(buf, { severity = { min = diagnostic.severity.WARN } })
    or {}
  local errors = diagnostic and problems[diagnostic.severity.ERROR] or 0
  local warnings = diagnostic and problems[diagnostic.severity.WARN] or 0
  if errors > 0 then
    gap()
    put(right, 'alert', glyph.failed)
    put(right, 'text', ' ' .. errors)
  end
  if warnings > 0 then
    gap()
    put(right, 'warning', glyph.caution)
    put(right, 'text', ' ' .. warnings)
  end

  if on.right then
    lost = lost or next(progress) ~= nil or paired() or (state and state.branch ~= nil)
  else
    local latest = nil
    for _, work in pairs(progress) do
      if not latest or work.at > latest.at then
        latest = work
      end
    end
    if latest then
      gap()
      put(right, 'program', glyph.running)
      if not on.progress then
        put(right, 'muted', ' ' .. latest.words)
      end
    end
    if paired() then
      gap()
      put(right, 'program', glyph.agent)
    end
    if state then
      gap()
      put(right, 'muted', glyph.branch)
      if on.branchglyph then
        lost = true
      else
        put(right, 'muted', ' ' .. (on.branchshort and middle(state.branch, 14) or state.branch))
      end
      if state.unsaved then
        put(right, 'text', ' ' .. glyph.on)
      end
      if state.ahead then
        put(right, 'text', ' ' .. glyph.unpushed)
      end
    end
  end
  if lost then
    gap()
    put(right, 'muted', glyph.more)
  end
  return left, right
end

-- The line as it is now, the window's width deciding how much gives way.
-- M.layout gives the pieces themselves, for the checks to read.
function M.layout(columns)
  if vim.bo[(shown())].filetype == 'snacks_dashboard' then
    return {}, {}
  end
  local on = {}
  local left, right = draw(on)
  for _, step in ipairs(M.steps) do
    if width(left) + (#right > 0 and 2 or 0) + width(right) <= columns then
      break
    end
    on[step] = true
    left, right = draw(on)
  end
  return left, right
end

function M.line()
  local left, right = M.layout(vim.o.columns)
  return render(left) .. '%#StatusText#%=' .. render(right)
end

-- The tab line, only while there are two tabs or more: each tab named by
-- the buffer in its window, the one in view in strong, the rest muted, two
-- spaces apart, each one a click away.
function M.tabs()
  local out = {}
  local current = vim.api.nvim_get_current_tabpage()
  for number, tab in ipairs(vim.api.nvim_list_tabpages()) do
    local buf = vim.api.nvim_win_get_buf(vim.api.nvim_tabpage_get_win(tab))
    local here = placed(buf)
    local name = here.path and (here.path.parts[#here.path.parts] or '/') or here.name
    local role = tab == current and 'strong' or 'muted'
    local mark = vim.bo[buf].modified and ' ' .. glyph.on or ''
    table.insert(
      out,
      '%' .. number .. 'T%#' .. groups[role] .. '#' .. name:gsub('%%', '%%%%') .. mark
    )
  end
  return table.concat(out, '%#StatusText#  ') .. '%T%#StatusText#'
end

-- Asks git about the buffer in view.
local function refresh()
  ask(folder((shown())))
end

function M.setup()
  paint()
  vim.o.statusline = "%!v:lua.require'status'.line()"
  vim.o.tabline = "%!v:lua.require'status'.tabs()"
  vim.o.showtabline = 1
  vim.opt.fillchars:append({ stl = ' ', stlnc = ' ' })

  local group = vim.api.nvim_create_augroup('status', { clear = true })
  vim.api.nvim_create_autocmd('ColorScheme', { group = group, callback = paint })
  beneath = vim.api.nvim_get_current_win()
  vim.api.nvim_create_autocmd('WinEnter', {
    group = group,
    callback = function()
      local win = vim.api.nvim_get_current_win()
      if not floating(win) then
        beneath = win
      end
    end,
  })
  vim.api.nvim_create_autocmd({ 'BufEnter', 'BufFilePost', 'BufWritePost' }, {
    group = group,
    callback = function(event)
      places[event.buf] = nil
    end,
  })
  vim.api.nvim_create_autocmd('DirChanged', {
    group = group,
    callback = function()
      places = {}
    end,
  })
  vim.api.nvim_create_autocmd('BufWipeout', {
    group = group,
    callback = function(event)
      places[event.buf] = nil
    end,
  })
  vim.api.nvim_create_autocmd({
    'BufEnter',
    'BufWritePost',
    'FocusGained',
    'DirChanged',
    'ShellCmdPost',
    'TermLeave',
    'VimResume',
  }, { group = group, callback = refresh })
  vim.api.nvim_create_autocmd('User', {
    group = group,
    pattern = 'FugitiveChanged',
    callback = refresh,
  })
  vim.api.nvim_create_autocmd('ModeChanged', {
    group = group,
    callback = function()
      vim.cmd.redrawstatus()
    end,
  })
  vim.api.nvim_create_autocmd({ 'CursorMoved', 'CmdlineLeave', 'OptionSet' }, {
    group = group,
    callback = function(event)
      if event.event ~= 'OptionSet' or event.match == 'hlsearch' then
        count_found()
      end
    end,
  })
  vim.api.nvim_create_autocmd('LspProgress', {
    group = group,
    callback = function(event)
      local value = event.data.params.value
      local key = event.data.client_id .. ':' .. tostring(event.data.params.token)
      if value.kind == 'end' then
        progress[key] = nil
      else
        local before = progress[key] or {}
        local title = value.title or before.title or ''
        local words = vim.trim(title .. ' ' .. (value.message or ''))
        if value.percentage then
          words = words .. ' ' .. value.percentage .. '%'
        end
        progress[key] = { title = title, words = words, at = vim.uv.hrtime() }
      end
      vim.cmd.redrawstatus()
    end,
  })
  vim.api.nvim_create_autocmd('LspDetach', {
    group = group,
    callback = function(event)
      for key in pairs(progress) do
        if vim.startswith(key, event.data.client_id .. ':') then
          progress[key] = nil
        end
      end
    end,
  })
  vim.api.nvim_create_autocmd('DiagnosticChanged', {
    group = group,
    callback = function()
      vim.cmd.redrawstatus()
    end,
  })
end

return M
