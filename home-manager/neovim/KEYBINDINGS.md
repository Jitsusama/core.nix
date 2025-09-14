# Neovim Key Bindings

## System Philosophy

This configuration uses a **multi-modal keybinding system** designed for consistency, memory efficiency, and ergonomic access to functionality. The system is built on clear conceptual patterns that scale logically.

### Design Principles

1. **Pattern-Based**: Consistent patterns across all functionality (not arbitrary memorization)
2. **Progressive Disclosure**: Common operations = short keys, advanced operations = longer sequences
3. **Conceptual Domains**: Operations grouped by what they affect, not how they work
4. **Native Preservation**: Never overrides Neovim's built-in keybindings
5. **Memory-Friendly**: Patterns and logic over memorization
6. **Discoverable**: Which-key integration for all custom mappings

### Multi-Modal Architecture

The system uses **5 distinct input patterns**, each with a clear mental model:

#### 1. **Leader Actions** (`<leader>*`) - "I want to DO something"
- **Pattern**: `<leader>` + domain + action
- **Mental Model**: Deliberate actions within specific areas
- **Examples**: `<leader>cf` (code format), `<leader>gg` (git status)

#### 2. **Sequential Navigation** (`[/]`) - "Step through ordered lists"
- **Pattern**: `[` = previous, `]` = next
- **Mental Model**: Moving through sequences that have clear order
- **Examples**: `[d`/`]d` (prev/next diagnostic), `[b`/`]b` (prev/next buffer)

#### 3. **Direct Jumps** (`g*`) - "Go directly to a specific location"
- **Pattern**: `g` + target type
- **Mental Model**: Teleporting to a known destination
- **Examples**: `gd` (go to definition), `gs` (go search with Flash)

#### 4. **System Controls** (`<C-*>`) - "System-level commands"
- **Pattern**: Control + key
- **Mental Model**: Editor and UI controls
- **Examples**: `<C-e>` (file explorer), `<C-/>` (terminal)

#### 5. **Direct Access** (no prefix) - "So common it needs instant access"
- **Pattern**: Single key or special combo
- **Mental Model**: Muscle memory for frequent operations
- **Examples**: `<BS>` (alternate buffer), `-` (parent directory)

---

## Leader Domain Organization

**Leader Key**: `<space>` (spacebar) - accessible with either thumb

### Primary Work Domains

#### `<leader>a` - **AI/Claude** 🤖
AI assistance and code collaboration operations.

| Pattern | Key | Action | Description |
|---------|-----|--------|-------------|
| Picker | `<leader>aa` | AI focus terminal | Smart terminal focus/toggle |
| Action | `<leader>as` | AI send context | Send buffer/visual selection to Claude |
| Action | `<leader>ad` | AI diff accept | Accept Claude's proposed changes |
| Action | `<leader>ar` | AI diff reject | Reject Claude's proposed changes |
| Action | `<leader>am` | AI model select | Select Claude model (interactive) |

#### `<leader>b` - **Buffers** 🔲
Operations on open file buffers in memory.

| Pattern | Key | Action | Description |
|---------|-----|--------|-------------|
| Picker | `<leader>bb` | Buffer browser | Interactive buffer picker |
| Action | `<leader>bd` | Buffer delete | Close buffer, keep window |
| Action | `<leader>bD` | Buffer Delete (force) | Force close buffer |
| Action | `<leader>bo` | Buffer only | Close all other buffers |
| Action | `<leader>bs` | Buffer scratch | Create scratch buffer |
| Action | `<leader>bS` | Buffer Scratch (select) | Select from scratch buffers |
| Action | `<leader>bw` | Buffer write | Save current buffer |

#### `<leader>c` - **Code** 💻
Code intelligence, refactoring, and formatting operations.

##### Core Actions
| Key | Action | Description |
|-----|--------|-------------|
| `<leader>ca` | Code action | Show available code actions |
| `<leader>cf` | Code format | Format current buffer |
| `<leader>cF` | Code Format (injected) | Format injected languages |
| `<leader>ch` | Code hover | Show hover information |
| `<leader>cH` | Code Help | Show signature help |
| `<leader>cn` | Code align | Interactive text alignment (visual/normal) |
| `<leader>cR` | Code Rename | Rename symbol under cursor |

##### Code Structure (`<leader>co`)
| Key | Action | Description |
|-----|--------|-------------|
| `<leader>coo` | Code outline | Toggle symbol outline sidebar |
| `<leader>coO` | Code Outline (focus) | Open outline with focus |
| `<leader>cos` | Code outline symbols | Fuzzy search symbols |
| `<leader>con` | Code outline next | Jump to next symbol |
| `<leader>cop` | Code outline previous | Jump to previous symbol |
| `<leader>coN` | Code outline Navigate | Floating symbol navigator |

##### Code Refactoring (`<leader>cr`)
| Key | Action | Description |
|-----|--------|-------------|
| `<leader>cre` | Code refactor extract (function) | Extract selection to function |
| `<leader>crf` | Code refactor file | Extract function to new file |
| `<leader>crv` | Code refactor variable | Extract to variable |
| `<leader>cri` | Code refactor inline | Inline variable at cursor |
| `<leader>crb` | Code refactor block | Extract to if block |
| `<leader>crB` | Code refactor Block (file) | Extract block to new file |

##### Code Calls (`<leader>cc`)
| Key | Action | Description |
|-----|--------|-------------|
| `<leader>cci` | Code calls incoming | Show incoming calls |
| `<leader>cco` | Code calls outgoing | Show outgoing calls |

##### Code Lens (`<leader>cl`)
| Key | Action | Description |
|-----|--------|-------------|
| `<leader>clr` | Code lens run | Execute code lens |
| `<leader>clf` | Code lens refresh | Refresh code lens |

##### Code Selection (`<leader>cs`)
| Key | Action | Description |
|-----|--------|-------------|
| `<leader>css` | Code selection start | Start treesitter selection |
| `<leader>csi` | Code selection increment | Expand to parent node |
| `<leader>csc` | Code selection scope | Expand to containing scope |
| `<leader>csd` | Code selection decrement | Shrink selection |

##### Code Toggles (`<leader>ct`)
| Key | Action | Description |
|-----|--------|-------------|
| `<leader>ctf` | Code toggle format | Toggle auto-format (buffer) |
| `<leader>ctF` | Code toggle Format (global) | Toggle auto-format (global) |
| `<leader>cti` | Code toggle inlay | Toggle inlay hints |

#### `<leader>f` - **Files** 📁
File system operations with **fzf-lua enhanced interface** for consistent performance.

| Pattern | Key | Action | Description |
|---------|-----|--------|-------------|
| Picker | `<leader>ff` | Find files | **Enhanced file finder** (fzf-lua) |
| Action | `<leader>fr` | Files recent | **Recently opened files** (fzf-lua) |
| Action | `<leader>fe` | Files explorer | File manager (Oil) |
| Action | `<leader>fo` | Files open (global) | **Global file finder** (fzf-lua) |
| Action | `<leader>fg` | Files git | **Git tracked files** (fzf-lua) |

#### `<leader>g` - **Git** 🌿
Version control operations with **repeating character primary pickers** and enhanced fzf-lua interface.

##### Quick Status
| Key | Action | Description |
|-----|--------|-------------|
| `<leader>gg` | Git status | **Interactive git status** (fzf-lua enhanced) |

##### Archive/Stash Operations (`<leader>ga`)
| Key | Action | Description |
|-----|--------|-------------|
| `<leader>gaa` | Git archive list | **Stash list picker** (fzf-lua enhanced) |
| `<leader>gas` | Git archive save | Save changes to stash |
| `<leader>gap` | Git archive pop | Apply and remove stash |
| `<leader>gal` | Git archive apply | Apply without removing |
| `<leader>gad` | Git archive drop | Delete stash |

##### Branches (`<leader>gb`)
| Key | Action | Description |
|-----|--------|-------------|
| `<leader>gbb` | Git branch browser | **Branch list picker** (fzf-lua enhanced) |
| `<leader>gbc` | Git branch checkout | Switch branch |
| `<leader>gb-` | Git branch previous | Switch to previous branch |
| `<leader>gbn` | Git branch new | Create and switch to branch |
| `<leader>gbd` | Git branch delete | Delete branch |
| `<leader>gbD` | Git branch Delete (force) | Force delete branch |

##### Commits (`<leader>gc`)
| Key | Action | Description |
|-----|--------|-------------|
| `<leader>gcc` | Git commit commits | **Commit history picker** (fzf-lua enhanced) |
| `<leader>gcf` | Git commit file | **File commit history** (fzf-lua enhanced) |
| `<leader>gca` | Git commit amend | Amend previous commit |
| `<leader>gc!` | Git commit amend! | Amend without editing |
| `<leader>gcg` | Git commit graph | View commit graph |
| `<leader>gcb` | Git commit blame | Show line blame |
| `<leader>gcn` | Git commit new | Create new commit |

##### Staging (`<leader>gs`)
| Key | Action | Description |
|-----|--------|-------------|
| `<leader>gsa` | Git stage add | Stage current hunk |
| `<leader>gsA` | Git stage Add (file) | Stage entire file |
| `<leader>gsu` | Git stage undo hunk | Unstage hunk |
| `<leader>gsv` | Git stage view | View staged changes |

##### Tags (`<leader>gt`)
| Key | Action | Description |
|-----|--------|-------------|
| `<leader>gtt` | Git tag tags | **Tag list picker** (fzf-lua enhanced) |
| `<leader>gtn` | Git tag new | Create new tag |
| `<leader>gtd` | Git tag delete | Delete tag |

##### Working Directory (`<leader>gw`)
| Key | Action | Description |
|-----|--------|-------------|
| `<leader>gww` | Git working hunks | **Hunk list picker** (fzf-lua enhanced) |
| `<leader>gwb` | Git working blame | **Blame navigator** (fzf-lua enhanced) |
| `<leader>gwh` | Git working hunk | Preview hunk changes |
| `<leader>gwv` | Git working view | View file diff |
| `<leader>gwH` | Git working vs HEAD | Compare to HEAD |
| `<leader>gwd` | Git working discard | Discard hunk changes |
| `<leader>gwD` | Git working Discard (file) | Discard all file changes |

##### GitHub (`<leader>gh`)

Complete GitHub integration with context-aware operations. Commands are organized into **global operations** (available everywhere) and **buffer-specific operations** (only in GitHub buffers).

---

#### **Global GitHub Operations** (Available Everywhere)

**Quick Access**
| Key | Action | Description |
|-----|--------|-------------|
| `<leader>ghh` | GitHub home | Quick PR list (most common operation) |

**Search Operations** (`<leader>ghs`)
| Key | Action | Description |
|-----|--------|-------------|
| `<leader>ghs` | GitHub search | Search current repository |
| `<leader>ghS` | GitHub Search (global) | Search across all GitHub |

**Issues** (`<leader>ghi`)
| Key | Action | Description |
|-----|--------|-------------|
| `<leader>ghii` | Issue picker | Browse issues (double-letter picker pattern) |
| `<leader>ghin` | Issue new | Create new issue |
| `<leader>ghis` | Issue search | Search issues with prompt |

**Pull Requests** (`<leader>ghp`)
| Key | Action | Description |
|-----|--------|-------------|
| `<leader>ghpp` | PR picker | Browse pull requests (double-letter picker pattern) |
| `<leader>ghpn` | PR new | Create new PR |
| `<leader>ghpc` | PR current | Find PR for current branch |
| `<leader>ghps` | PR search | Search PRs with prompt |
| `<leader>ghpy` | PR yank url | Copy PR URL for current branch |
| `<leader>ghpo` | PR open browser | Open current branch's PR in browser |
| `<leader>ghpC` | PR Checks | View CI status for current branch |

**Repository** (`<leader>ghr`)
| Key | Action | Description |
|-----|--------|-------------|
| `<leader>ghrr` | Repo picker | Browse repositories (double-letter picker pattern) |
| `<leader>ghrf` | Repo fork | Fork current repository |
| `<leader>ghry` | Repo yank url | Copy repository URL |
| `<leader>ghro` | Repo open browser | Open repository in browser |

**File/Code Operations** (`<leader>ghf`) - GitHub Permalinks
| Key | Action | Description |
|-----|--------|-------------|
| `<leader>ghfy` | File yank permalink | Copy GitHub URL (line or visual range) |
| `<leader>ghfo` | File open browser | Open in GitHub (line or visual range) |

**Gists** (`<leader>ghg`)
| Key | Action | Description |
|-----|--------|-------------|
| `<leader>ghgg` | Gist picker | Browse gists (double-letter picker pattern) |
| `<leader>ghgn` | Gist new | Create gist from buffer/selection |

**Notifications** (`<leader>ghn`)
| Key | Action | Description |
|-----|--------|-------------|
| `<leader>ghnn` | Notification picker | Browse notifications (double-letter picker pattern) |

---

#### **Buffer-Specific Operations** (GitHub Buffers Only)

These operations only appear in which-key and only work when you're in a GitHub issue, PR, or review buffer.

**Actions** (`<leader>gha`) - Assignees & Reviewers
| Key | Action | Description |
|-----|--------|-------------|
| `<leader>ghaa` | Action assignee add | Add assignee to issue/PR |
| `<leader>ghad` | Action assignee delete | Remove assignee from issue/PR |
| `<leader>ghaR` | Action Reviewer add | Add reviewer to PR |
| `<leader>ghar` | Action reviewer remove | Remove reviewer from PR |

**Comments** (`<leader>ghc`)
| Key | Action | Description |
|-----|--------|-------------|
| `<leader>ghca` | Comment add | Add comment to issue/PR |
| `<leader>ghcr` | Comment reply | Reply to comment |
| `<leader>ghcd` | Comment delete | Delete comment |
| `<leader>ghcs` | Comment suggest | Add suggestion (PR review only) |

**Labels** (`<leader>ghl`)
| Key | Action | Description |
|-----|--------|-------------|
| `<leader>ghlc` | Label create | Create new label |
| `<leader>ghla` | Label add | Add label to issue/PR |
| `<leader>ghld` | Label delete | Remove label from issue/PR |

**Reactions** (`<leader>ght`) - Emoji Reactions
| Key | Action | Description |
|-----|--------|-------------|
| `<leader>ght+` | Thumbs up 👍 | Add/remove thumbs up reaction |
| `<leader>ght-` | Thumbs down 👎 | Add/remove thumbs down reaction |
| `<leader>ghtp` | Party 🎉 | Add/remove party reaction |
| `<leader>ghth` | Heart ❤️ | Add/remove heart reaction |
| `<leader>ghte` | Eyes 👀 | Add/remove eyes reaction |
| `<leader>ghtr` | Rocket 🚀 | Add/remove rocket reaction |
| `<leader>ghtl` | Laugh 😄 | Add/remove laugh reaction |
| `<leader>ghtc` | Confused 😕 | Add/remove confused reaction |

**Merge Operations** (`<leader>ghm`) - PR Buffers Only
| Key | Action | Description |
|-----|--------|-------------|
| `<leader>ghmo` | Merge checkout | Checkout PR locally |
| `<leader>ghmm` | Merge commit | Merge with commit |
| `<leader>ghms` | Merge squash | Squash and merge |
| `<leader>ghmr` | Merge rebase | Rebase and merge |

**Review Operations** (`<leader>ghv`) - PR Buffers Only
| Key | Action | Description |
|-----|--------|-------------|
| `<leader>ghvs` | reView start | Start a review |
| `<leader>ghvr` | reView resume | Resume pending review |
| `<leader>ghvS` | reView Submit | Submit completed review |
| `<leader>ghvd` | reView discard | Discard pending review |
| `<leader>ghvc` | reView commits | List PR commits |
| `<leader>ghvf` | reView files | List changed files |
| `<leader>ghvD` | reView Diff | Show full PR diff |
| `<leader>ghve` | reView focus files | Focus file panel |
| `<leader>ghvb` | reView toggle files | Toggle file panel |
| `<leader>ghvt` | reView toggle viewed | Toggle file viewed status |

**Buffer Operations** (`<leader>ghb`) - All GitHub Buffers
| Key | Action | Description |
|-----|--------|-------------|
| `<leader>ghbr` | Buffer reload | Reload current GitHub buffer |
| `<leader>ghby` | Buffer yank url | Copy URL of current issue/PR |
| `<leader>ghbo` | Buffer open browser | Open current issue/PR in browser |
| `<leader>ghbc` | Buffer close | Close issue/PR |
| `<leader>ghbR` | Buffer Reopen | Reopen issue/PR |

**Thread Operations** (`<leader>ghT`) - Review Buffers Only
| Key | Action | Description |
|-----|--------|-------------|
| `<leader>ghTr` | Thread resolve | Resolve PR thread |
| `<leader>ghTu` | Thread unresolve | Unresolve PR thread |

**Notification Operations** - Notification Buffers Only
| Key | Action | Description |
|-----|--------|-------------|
| `<leader>ghnr` | Notification read | Mark notification as read |
| `<leader>ghnd` | Notification done | Mark notification as done |
| `<leader>ghnu` | Notification unsubscribe | Unsubscribe from notifications |

##### Network Operations (`<leader>gn`)
| Key | Action | Description |
|-----|--------|-------------|
| `<leader>gnp` | Git network push | Push to remote |
| `<leader>gnl` | Git network pull | Pull from remote |
| `<leader>gnf` | Git network fetch | Fetch from remote |


#### `<leader>s` - **Search** 🔍
All search and content discovery operations with **fzf-lua enhanced interface**.

| Pattern | Key | Action | Description |
|---------|-----|--------|-------------|
| Action | `<leader>sb` | Search buffer | Current buffer fuzzy find (fzf-lua) |
| Action | `<leader>sB` | Search all Buffers | Search across ALL open buffers (fzf-lua) |
| Action | `<leader>sp` | Search project | **Live grep with real-time updates** (fzf-lua) |
| Action | `<leader>sw` | Search word | Search word under cursor (fzf-lua) |
| Action | `<leader>sy` | Search symbols | **LSP workspace symbols** (fzf-lua) |
| Action | `<leader>sh` | Search history | Browse search history (fzf-lua) |
| Action | `<leader>sc` | Search clear | Clear search highlighting |
| Action | `<leader>sH` | Search help | Search help documentation (fzf-lua) |
| Action | `<leader>sr` | Search replace | Search and replace |
| Action | `<leader>st` | Search todos (quickfix) | Search TODO/FIXME/NOTE comments with quickfix |
| Action | `<leader>sq` | Search quickfix | **Quickfix list browser** (fzf-lua) |
| Action | `<leader>sQ` | Search location list | **Location list browser** (fzf-lua) |

#### `<leader>t` - **Tests** 🧪
Test execution, results, and debugging.

##### Test Execution
| Key | Action | Description |
|-----|--------|-------------|
| `<leader>tt` | Test nearest | Run test at cursor |
| `<leader>tf` | Test file | Run all tests in file |
| `<leader>ts` | Test suite (all) | Run entire test suite |
| `<leader>tl` | Test last | Re-run last test |

##### Test Output
| Key | Action | Description |
|-----|--------|-------------|
| `<leader>to` | Test output | Show test output |
| `<leader>tO` | Test Output (panel) | Persistent output panel |
| `<leader>tS` | Test Summary | Test results sidebar |

##### Test Control
| Key | Action | Description |
|-----|--------|-------------|
| `<leader>tx` | Test stop | Stop running tests |
| `<leader>tw` | Test watch file | Watch current file |
| `<leader>tW` | Test Watch suite | Watch entire suite |
| `<leader>td` | Test debug | Debug nearest test |

### Supporting Domains

#### `<leader>i` - **Introspection** 🔍
Examining Neovim's internal state and discovering functionality.

| Key | Action | Description |
|-----|--------|-------------|
| `<leader>ik` | Introspect keymaps | Browse active keybindings |
| `<leader>ic` | Introspect commands | Browse available commands |
| `<leader>io` | Introspect options | Browse vim settings |
| `<leader>ih` | Introspect history | Command history |
| `<leader>is` | Introspect search | Search history |
| `<leader>ir` | Introspect registers | Register contents |
| `<leader>im` | Introspect marks | Active marks |
| `<leader>iH` | Introspect Help | Search help docs |
| `<leader>in` | Introspect notifications | Notification history |
| `<leader>iN` | Introspect Notifications clear | Clear notifications |
| `<leader>il` | Introspect LSP | LSP server status |
| `<leader>it` | Introspect treesitter | Parser information |

#### `<leader>k` - **Workspace** 📦
LSP workspace folder management and configuration.

| Key | Action | Description |
|-----|--------|-------------|
| `<leader>ka` | Workspace add folder | Add folder to LSP workspace |
| `<leader>kr` | Workspace remove folder | Remove folder from LSP workspace |
| `<leader>kl` | Workspace list folders | List current LSP workspace folders |

#### `<leader>T` - **Tabs** 📑
Tab management operations.

| Pattern | Key | Action | Description |
|---------|-----|--------|-------------|
| Picker | `<leader>TT` | Tab picker | Browse and switch tabs |
| Action | `<leader>Tn` | Tab new | Create new tab |
| Action | `<leader>Tc` | Tab close | Close current tab |
| Action | `<leader>To` | Tab only | Close all other tabs |
| Action | `<leader>Tf` | Tab first | Go to first tab |
| Action | `<leader>Tl` | Tab last | Go to last tab |

#### `<leader>u` - **UI/Settings** 🎨
Visual appearance and editor settings.

| Key | Action | Description |
|-----|--------|-------------|
| `<leader>ub` | UI background | Toggle light/dark background |
| `<leader>uc` | UI colorscheme | Browse colorschemes |
| `<leader>ul` | UI line numbers | Toggle line numbers |
| `<leader>ut` | UI twilight | Toggle focus mode (dims inactive code) |
| `<leader>uw` | UI wrap | Toggle line wrapping |

#### `<leader>w` - **Windows** 🪟
Window management and layout operations.

| Pattern | Key | Action | Description |
|---------|-----|--------|-------------|
| Picker | `<leader>ww` | Window picker | Interactive window selection |
| Action | `<leader>ws` | Window split | Horizontal split |
| Action | `<leader>wv` | Window vertical | Vertical split |
| Action | `<leader>wc` | Window close | Close current window |
| Action | `<leader>wo` | Window only | Close other windows |
| Action | `<leader>w=` | Window equalize | Equal window sizes |
| Action | `<leader>w+` | Window height+ | Increase height |
| Action | `<leader>w-` | Window height- | Decrease height |
| Action | `<leader>w>` | Window width+ | Increase width |
| Action | `<leader>w<` | Window width- | Decrease width |

---

## Sequential Navigation (`[` / `]`)

**Mental Model**: "Step through ordered lists" - `[` = previous, `]` = next

### Core Sequences
| Pattern | Previous | Next | Description |
|---------|----------|------|-------------|
| Buffers | `[b` | `]b` | Navigate through buffer list |
| Diagnostics | `[d` | `]d` | Navigate through errors/warnings |
| Git Hunks | `[g` | `]g` | Navigate through changed code blocks |
| Tests | `[t` | `]t` | Navigate through test definitions |
| Tabs | `[T` | `]T` | Navigate through tab list |
| Windows | `[w` | `]w` | Navigate through window list |
| Symbols | `[s` | `]s` | Navigate through symbol list |
| Notes | `[n` | `]n` | Navigate through TODO/FIXME/NOTE comments |

### Code Structure
| Pattern | Previous | Next | Description |
|---------|----------|------|-------------|
| Functions | `[f` | `]f` | Navigate through function definitions |
| Classes | `[c` | `]c` | Navigate through class definitions |
| Comments | `[C` | `]C` | Navigate through comment blocks |

### Lists and Results
| Pattern | Previous | Next | Description |
|---------|----------|------|-------------|
| Quickfix | `[q` | `]q` | Navigate through quickfix items |
| Location | `[l` | `]l` | Navigate through location list |

---

## Direct Jump Navigation (`g*`)

**Mental Model**: "Go directly to a specific location"

### LSP Jumps
| Key | Target | Description |
|-----|--------|-------------|
| `gd` | Definition | Jump to symbol definition |
| `gD` | Declaration | Jump to symbol declaration |
| `gi` | Implementation | Jump to implementation |
| `gr` | References | Jump to symbol references |
| `gy` | Type definition | Jump to type definition |

### Flash Navigation
| Key | Target | Description |
|-----|--------|-------------|
| `gs` | Search | Flash 2-char search |
| `gS` | Structural | Flash treesitter search |
| `gl` | Line | Flash jump to line |
| `gw` | Word | Flash jump to word |

### Buffer Navigation
| Key | Target | Description |
|-----|--------|-------------|
| `ga` | Alternate | Jump to alternate buffer |

### Native Vim (preserved)
| Key | Target | Description |
|-----|--------|-------------|
| `gf` | File | Go to file under cursor |
| `gg` | Top | Go to top of file |
| `G` | Bottom | Go to bottom of file |
| `gt` | Next tab | Next tab (native) |
| `gT` | Previous tab | Previous tab (native) |

---

## System Controls (`<C-*>`)

**Mental Model**: "System-level commands and UI controls"

### File and Explorer
| Key | Action | Description |
|-----|--------|-------------|
| `<C-e>` | Explorer | Toggle file explorer |
| `<C-p>` | Palette | Command palette (which-key) |

### Terminal and Shell
| Key | Action | Description |
|-----|--------|-------------|
| `<C-/>` | Terminal | Toggle terminal |

### Window Operations (extends `<C-w>`)
| Key | Action | Description |
|-----|--------|-------------|
| `<C-w>d` | Delete buffer | Close buffer, keep window |
| `<C-w>D` | Delete buffer! | Force close buffer |

### Lists and Results  
| Key | Action | Description |
|-----|--------|-------------|
| `<C-q>` | Quickfix | Toggle quickfix list |
| `<C-l>` | Clear/Refresh | Clear search/refresh |

---

## Direct Access Keys

**Mental Model**: "So common it needs instant access"

### Super-Common Operations
| Key | Action | Description |
|-----|--------|-------------|
| `<BS>` | Alternate buffer | Toggle between two most recent buffers |
| `-` | Parent directory | Navigate up one directory (Oil) |

### Special Access
| Key | Action | Description |
|-----|--------|-------------|
| `<leader><leader>` | Global picker | **Global picker (VS Code-like unified search)** |
| `<leader>.` | Resume last | Resume last fzf-lua operation |
| `<leader>/` | Quick search | Search in current buffer |
| `<leader>Q` | Quit all | Quit all buffers with confirmation |
| `<leader>U` | Undo tree | Toggle undotree visualization |

---

## Completion (Insert Mode)

**Plugin**: nvim-cmp

| Key | Action | Description |
|-----|--------|-------------|
| `<Tab>` | Smart complete | Auto-trigger completion or navigate to next item |
| `<S-Tab>` | Smart previous | Navigate to previous completion item |
| `<C-Tab>` | Literal tab | Insert actual tab character |
| `<C-Space>` | Complete | Manual completion trigger |
| `<C-e>` | Abort | Cancel completion |
| `<C-y>` | Confirm | Accept completion |
| `<C-b>` | Scroll up | Scroll docs up |
| `<C-f>` | Scroll down | Scroll docs down |

---

## Text Objects

### Treesitter Objects
| Key | Scope | Description |
|-----|-------|-------------|
| `af` / `if` | Function | Around/inside function |
| `ac` / `ic` | Class | Around/inside class |
| `al` / `il` | Loop | Around/inside loop |
| `aa` / `ia` | Argument | Around/inside argument |

### Git Objects
| Key | Scope | Description |
|-----|-------|-------------|
| `ah` / `ih` | Hunk | Around/inside git hunk |

---

## Error/Diagnostic Domain

**Note**: Errors are handled through the sequential navigation system (`[d`/`]d`) and specific leader commands:

### Error Navigation (Sequential)
- `[d` / `]d` - Previous/next diagnostic
- `[D` / `]D` - First/last diagnostic in file

### Error Operations (Leader)
| Key | Action | Description |
|-----|--------|-------------|
| `<leader>ee` | Error details | Show error at cursor |
| `<leader>el` | Error list | Document diagnostics |
| `<leader>eL` | Error List (workspace) | Workspace diagnostics |
| `<leader>eq` | Error quickfix | Quickfix list |
| `<leader>ea` | Error all | Full trouble interface |

### Error Toggles
| Key | Action | Description |
|-----|--------|-------------|
| `<leader>etd` | Error toggle diagnostics | Enable/disable diagnostics |
| `<leader>etv` | Error toggle virtual | Toggle inline errors |
| `<leader>ets` | Error toggle signs | Toggle gutter signs |

---

## Window Navigation Shortcuts

**Alt-based shortcuts for speed**:

| Key | Action | Description |
|-----|--------|-------------|
| `<M-S-Left>` | Window left | Quick window navigation |
| `<M-S-Right>` | Window right | Quick window navigation |
| `<M-S-Up>` | Window up | Quick window navigation |
| `<M-S-Down>` | Window down | Quick window navigation |
| `<A-Up>` | Window taller | Increase window height |
| `<A-Down>` | Window shorter | Decrease window height |

---

## Tab Navigation Shortcuts

**Control-based shortcuts for tabs**:

| Key | Action | Description |
|-----|--------|-------------|
| `<C-Left>` | Previous tab | Quick tab switching |
| `<C-Right>` | Next tab | Quick tab switching |

---

## Implementation Notes for LLMs

When working with this keybinding system:

1. **Pattern Recognition**: Always identify which pattern applies:
   - Leader action: `<leader>` + domain + action
   - Sequential nav: `[`/`]` + type
   - Direct jump: `g` + target
   - System control: `<C-*>`

2. **Domain Assignment**: New functionality should map to logical domains:
   - File operations → `<leader>f*`
   - Code operations → `<leader>c*` 
   - Search operations → `<leader>s*`
   - Buffer operations → `<leader>b*`

3. **Consistency Rules**:
   - Pickers use double letters (`<leader>bb`, `<leader>ff`)
   - Navigation is `g` for jumps, `[/]` for sequences  
   - All custom mappings must have which-key descriptions
   - Icons should be meaningful and consistent

4. **Never Override**:
   - Native vim bindings
   - Existing patterns without explicit redesign
   - The core modal philosophy

This system prioritizes **discoverability**, **consistency**, and **ergonomics** over arbitrary key assignments.

---

## Plugin-Specific Keybindings

Some plugins have their own keybinding systems that work within specific buffer contexts. These are documented here for completeness.

### vim-surround (Global)

Operators for managing surrounding pairs (quotes, brackets, tags, etc.):

#### Operators
| Key | Action | Description | Example |
|-----|--------|-------------|---------|
| `ds` | Delete surrounding | Remove surrounding pair | `ds"` → Remove quotes |
| `cs` | Change surrounding | Change one pair to another | `cs"'` → Change " to ' |
| `ys` | Add surrounding | Add pair around motion | `ysiw"` → Quote word |
| `yss` | Surround line | Add pair around entire line | `yss)` → Parenthesize line |

#### Visual Mode
| Key | Action | Description |
|-----|--------|-------------|
| `S` | Surround selection | Wrap selection with pair |
| `gS` | Surround (new lines) | Wrap selection with pair on new lines |

#### Text Objects
| Key | Scope | Description |
|-----|-------|-------------|
| `is` | Inner surrounding | Inside the surrounding pair |
| `as` | A surrounding | Around the surrounding pair (with spaces) |

#### Common Targets
| Target | Result | Notes |
|--------|--------|-------|
| `"` `'` `` ` `` | Quotes | Various quote types |
| `(` `)` `b` | Parentheses | `(` adds spaces, `)` or `b` doesn't |
| `[` `]` `r` | Brackets | `[` adds spaces, `]` or `r` doesn't |
| `{` `}` `B` | Braces | `{` adds spaces, `}` or `B` doesn't |
| `<` `>` `a` | Angle brackets | For generic brackets |
| `t` | HTML/XML tags | Prompts for tag name |

**Tip**: Use `:SurroundHelp` command for quick reference.

### Oil.nvim (File Explorer Buffers)

When in an Oil buffer (file explorer), these additional mappings are available:

| Key | Action | Description |
|-----|--------|-------------|
| `g?` | Help | Show Oil help |
| `<CR>` | Select | Open file/directory |
| `<C-s>` | Split | Open in horizontal split |
| `<C-h>` | Split horiz | Open in horizontal split |
| `<C-t>` | Tab | Open in new tab |
| `<C-p>` | Preview | Preview file |
| `<C-c>` | Close | Close Oil buffer |
| `<C-l>` | Refresh | Refresh directory |
| `-` | Parent | Go to parent directory |
| `_` | CWD | Open current working directory |
| `` ` `` | CD | Change directory |
| `~` | CD (tab) | Change directory for tab |
| `gx` | External | Open with system app |
| `gh` | Hidden | Toggle hidden files |
| `g\` | Trash | Toggle trash view |

**Note**: The `gs` mapping has been disabled to avoid conflicts with Flash navigation.

### Fugitive Git Buffers

When in a Git status buffer (`:Git`), these buffer-local mappings are available:

| Key | Action | Description |
|-----|--------|-------------|
| `-` | Stage/unstage | Toggle staging for file/hunk |
| `=` | Diff | Toggle inline diff |
| `>` / `<` | Navigate | Next/previous file |
| `X` | Discard | Discard changes |
| `cc` | Commit | Create commit |
| `ca` | Amend | Amend previous commit |
| `ce` | Amend (no edit) | Amend without editing message |
| `dd` | Diff | Show diff |
| `dv` | Vertical diff | Show vertical diff split |
| `pp` | Push | Push to remote |
| `pf` | Push force | Force push with lease |
| `pu` | Push upstream | Push and set upstream |

### GitHub (Octo.nvim) Buffers

**Important**: GitHub keybindings only work when in GitHub issue/PR buffers opened through Octo commands.

All GitHub operations use the `<leader>gh*` prefix and are documented in the main GitHub section above.
