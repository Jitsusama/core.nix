-- The machine's theme as Neovim's colour scheme, every colour a role from
-- require('theme'), so changing the theme changes Neovim with everything
-- else. Nothing is drawn behind the text, so the terminal's own background
-- shows through. Code's structure goes by brightness and its meaning by a
-- few hues (the theme's syntax roles); the accent is kept for where the
-- keys go: the current match, a jump's labels and which-key's keys.

local t = require('theme')
local s = t.syntax

vim.cmd.highlight('clear')
if vim.g.syntax_on then
  vim.cmd.syntax('reset')
end
vim.g.colors_name = 'jitsusama'
vim.o.background = t.appearance or 'dark'

-- One colour laid over another at a share of its strength, for the tints
-- behind a diff's lines.
local function mix(over, under, share)
  local function channel(hex, at)
    return tonumber(hex:sub(at, at + 1), 16)
  end
  local out = '#'
  for at = 2, 6, 2 do
    local value = channel(over, at) * share + channel(under, at) * (1 - share)
    out = out .. string.format('%02x', math.floor(value + 0.5))
  end
  return out
end

-- A syntax role as a highlight: its colour, and its italics and bold.
local function code(role)
  return { fg = role.colour, italic = role.italic, bold = role.bold }
end

local groups = {
  -- The editor.
  Normal = { fg = t.text },
  NormalNC = { link = 'Normal' },
  NormalFloat = { fg = t.text, bg = t.sunken },
  -- A float stands apart by its shade, not a line around it.
  FloatBorder = { fg = t.sunken, bg = t.sunken },
  FloatTitle = { fg = t.strong, bg = t.sunken, bold = true },
  FloatFooter = { fg = t.muted, bg = t.sunken },
  Cursor = { fg = t.background, bg = t.strong },
  lCursor = { link = 'Cursor' },
  TermCursor = { link = 'Cursor' },
  CursorLine = { bg = t.sunken },
  CursorColumn = { link = 'CursorLine' },
  ColorColumn = { bg = t.sunken },
  LineNr = { fg = t.muted },
  LineNrAbove = { link = 'LineNr' },
  LineNrBelow = { link = 'LineNr' },
  CursorLineNr = { fg = t.strong },
  SignColumn = { fg = t.muted },
  FoldColumn = { fg = t.muted },
  CursorLineSign = { link = 'SignColumn' },
  CursorLineFold = { link = 'FoldColumn' },
  Folded = { fg = t.muted, bg = t.sunken },
  WinSeparator = { fg = t.rule },
  VertSplit = { link = 'WinSeparator' },
  EndOfBuffer = { fg = t.rule },
  NonText = { fg = t.rule },
  Whitespace = { fg = t.rule },
  SpecialKey = { fg = t.rule },
  Conceal = { fg = t.muted },
  StatusLine = { fg = t.text },
  StatusLineNC = { fg = t.muted },
  TabLine = { fg = t.muted },
  TabLineFill = {},
  TabLineSel = { fg = t.strong },
  WinBar = { fg = t.text },
  WinBarNC = { fg = t.muted },
  Title = { fg = t.strong, bold = true },
  Directory = { fg = t.strong },
  Visual = { bg = t.selection },
  VisualNOS = { link = 'Visual' },
  Search = { fg = t.strong, bg = t.raised },
  CurSearch = { fg = t.on_accent, bg = t.accent },
  IncSearch = { link = 'CurSearch' },
  Substitute = { link = 'CurSearch' },
  MatchParen = { fg = t.strong, bg = t.raised, bold = true },
  QuickFixLine = { bg = t.selection },
  Pmenu = { fg = t.text, bg = t.sunken },
  PmenuSel = { fg = t.strong, bg = t.selection },
  PmenuKind = { fg = t.muted, bg = t.sunken },
  PmenuKindSel = { fg = t.text, bg = t.selection },
  PmenuExtra = { fg = t.muted, bg = t.sunken },
  PmenuExtraSel = { fg = t.text, bg = t.selection },
  PmenuMatch = { fg = t.strong, bg = t.sunken, bold = true },
  PmenuMatchSel = { fg = t.strong, bg = t.selection, bold = true },
  PmenuSbar = { bg = t.sunken },
  PmenuThumb = { bg = t.rule },
  WildMenu = { link = 'PmenuSel' },
  MsgArea = { fg = t.text },
  ModeMsg = { fg = t.strong },
  MoreMsg = { fg = t.success },
  Question = { fg = t.strong },
  ErrorMsg = { fg = t.alert },
  WarningMsg = { fg = t.warning },
  SpellBad = { undercurl = true, sp = t.alert },
  SpellCap = { undercurl = true, sp = t.warning },
  SpellLocal = { undercurl = true, sp = t.program },
  SpellRare = { undercurl = true, sp = t.person },

  -- Code, by the theme's syntax roles.
  Comment = code(s.comment),
  SpecialComment = { link = 'Comment' },
  Todo = { fg = t.warning, bold = true },
  Constant = code(s.constant),
  Number = { link = 'Constant' },
  Float = { link = 'Constant' },
  Boolean = { link = 'Constant' },
  String = code(s.string),
  Character = { link = 'String' },
  Identifier = { fg = t.text },
  Function = code(s['function']),
  Statement = code(s.keyword),
  Keyword = { link = 'Statement' },
  Conditional = { link = 'Statement' },
  Repeat = { link = 'Statement' },
  Label = { link = 'Statement' },
  Exception = { link = 'Statement' },
  Operator = code(s.operator),
  PreProc = { link = 'Statement' },
  Include = { link = 'Statement' },
  Define = { link = 'Statement' },
  PreCondit = { link = 'Statement' },
  Macro = code(s.special),
  Type = code(s.type),
  StorageClass = { link = 'Statement' },
  Structure = { link = 'Type' },
  Typedef = { link = 'Type' },
  Special = code(s.special),
  SpecialChar = { link = 'Special' },
  Tag = { link = 'Statement' },
  Delimiter = code(s.punctuation),
  Debug = { link = 'Special' },
  Underlined = { underline = true },
  Bold = { bold = true },
  Italic = { italic = true },
  Ignore = { fg = t.muted },
  Error = { fg = t.alert },

  -- Tree-sitter's captures, where Neovim's own links don't say it: what's
  -- built in reads as what it is, names are plain text, and what the
  -- language reads specially is special.
  ['@variable'] = { fg = t.text },
  ['@variable.builtin'] = { link = 'Statement' },
  ['@variable.parameter'] = { fg = t.text },
  ['@variable.member'] = { fg = t.text },
  ['@property'] = { fg = t.text },
  ['@module'] = { fg = t.text },
  ['@module.builtin'] = { link = 'Statement' },
  ['@label'] = { link = 'Special' },
  ['@constant.builtin'] = { link = 'Constant' },
  ['@constant.macro'] = { link = 'Macro' },
  ['@function.builtin'] = { link = 'Function' },
  ['@function.macro'] = { link = 'Macro' },
  ['@constructor'] = { link = 'Type' },
  ['@type.builtin'] = { link = 'Type' },
  ['@attribute'] = { link = 'Special' },
  ['@string.escape'] = { link = 'Special' },
  ['@string.regexp'] = { link = 'Special' },
  ['@string.special'] = { link = 'Special' },
  ['@string.special.url'] = { link = 'Underlined' },
  ['@punctuation.special'] = { link = 'Special' },
  ['@tag'] = { link = 'Statement' },
  ['@tag.builtin'] = { link = 'Statement' },
  ['@tag.attribute'] = { fg = t.text },
  ['@tag.delimiter'] = { link = 'Delimiter' },
  ['@comment.todo'] = { link = 'Todo' },
  ['@comment.note'] = { fg = t.voice, bold = true },
  ['@comment.warning'] = { fg = t.warning, bold = true },
  ['@comment.error'] = { fg = t.alert, bold = true },
  ['@markup.heading'] = { fg = t.strong, bold = true },
  ['@markup.strong'] = { fg = t.strong, bold = true },
  ['@markup.italic'] = { italic = true },
  ['@markup.strikethrough'] = { strikethrough = true },
  ['@markup.underline'] = { underline = true },
  ['@markup.quote'] = { fg = t.muted, italic = true },
  ['@markup.math'] = { link = 'Special' },
  ['@markup.link'] = { fg = t.text },
  ['@markup.link.label'] = { fg = t.strong },
  ['@markup.link.url'] = { fg = t.muted, underline = true },
  ['@markup.raw'] = { link = 'String' },
  ['@markup.list'] = { link = 'Delimiter' },
  ['@markup.list.checked'] = { fg = t.success },
  ['@markup.list.unchecked'] = { link = 'Delimiter' },
  ['@diff.plus'] = { link = 'Added' },
  ['@diff.minus'] = { link = 'Removed' },
  ['@diff.delta'] = { link = 'Changed' },

  -- The language server's tokens, where they'd say more than Tree-sitter.
  ['@lsp.type.macro'] = { link = 'Macro' },
  ['@lsp.type.namespace'] = { link = '@module' },
  ['@lsp.type.enumMember'] = { link = 'Constant' },
  ['@lsp.mod.deprecated'] = { strikethrough = true },
  LspReferenceText = { bg = t.raised },
  LspReferenceRead = { link = 'LspReferenceText' },
  LspReferenceWrite = { link = 'LspReferenceText' },
  LspInlayHint = { fg = t.muted },
  LspCodeLens = { fg = t.muted },
  LspSignatureActiveParameter = { fg = t.strong, bold = true, underline = true },

  -- Diagnostics, by what they need of Joel.
  DiagnosticError = { fg = t.alert },
  DiagnosticWarn = { fg = t.warning },
  DiagnosticInfo = { fg = t.voice },
  DiagnosticHint = { fg = t.muted },
  DiagnosticOk = { fg = t.success },
  DiagnosticUnderlineError = { undercurl = true, sp = t.alert },
  DiagnosticUnderlineWarn = { undercurl = true, sp = t.warning },
  DiagnosticUnderlineInfo = { undercurl = true, sp = t.voice },
  DiagnosticUnderlineHint = { undercurl = true, sp = t.muted },
  DiagnosticUnderlineOk = { undercurl = true, sp = t.success },
  DiagnosticUnnecessary = { link = 'Comment' },
  DiagnosticDeprecated = { strikethrough = true },

  -- Changes: a line's mark in its hue, and its tint behind a diff.
  Added = { fg = t.success },
  Changed = { fg = t.warning },
  Removed = { fg = t.alert },
  DiffAdd = { bg = mix(t.success, t.background, 0.18) },
  DiffChange = { bg = mix(t.warning, t.background, 0.1) },
  DiffText = { bg = mix(t.warning, t.background, 0.26) },
  DiffDelete = { fg = t.alert, bg = mix(t.alert, t.background, 0.14) },
  diffAdded = { link = 'Added' },
  diffChanged = { link = 'Changed' },
  diffRemoved = { link = 'Removed' },
  GitSignsAdd = { link = 'Added' },
  GitSignsChange = { link = 'Changed' },
  GitSignsDelete = { link = 'Removed' },

  -- The plugins whose own defaults wouldn't draw the design: a jump's labels
  -- and which-key's keys are where the keys go, so they wear the accent.
  FlashLabel = { fg = t.on_accent, bg = t.accent, bold = true },
  FlashMatch = { link = 'Search' },
  FlashCurrent = { link = 'CurSearch' },
  WhichKey = { fg = t.accent },
  WhichKeyDesc = { fg = t.text },
  WhichKeyGroup = { fg = t.strong },
  WhichKeySeparator = { fg = t.rule },
  WhichKeyNormal = { link = 'NormalFloat' },
  WhichKeyBorder = { link = 'FloatBorder' },
  CmpItemAbbr = { fg = t.text },
  CmpItemAbbrMatch = { fg = t.strong, bold = true },
  CmpItemAbbrMatchFuzzy = { link = 'CmpItemAbbrMatch' },
  CmpItemAbbrDeprecated = { fg = t.muted, strikethrough = true },
  CmpItemKind = { fg = t.muted },
  CmpItemMenu = { fg = t.muted },
}

for name, spec in pairs(groups) do
  vim.api.nvim_set_hl(0, name, spec)
end

-- The terminal's sixteen colours, for Neovim's own terminal.
local ansi = {
  'black',
  'red',
  'green',
  'yellow',
  'blue',
  'magenta',
  'cyan',
  'white',
  'bright_black',
  'bright_red',
  'bright_green',
  'bright_yellow',
  'bright_blue',
  'bright_magenta',
  'bright_cyan',
  'bright_white',
}
for at, name in ipairs(ansi) do
  vim.g['terminal_color_' .. (at - 1)] = t.terminal[name]
end
