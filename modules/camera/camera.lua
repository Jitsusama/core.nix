-- The camera's viewfinder, inside mpv (home.nix): the picture mirrored, as a
-- mirror shows Joel to himself, and a panel below saying what the keys do.
-- Return takes a picture, or starts a video and then stops it, and either
-- way the viewfinder goes once it's done, as a screenshot's would; Tab
-- changes which; Escape closes it, finishing a video first. Nothing else is
-- bound, so no
-- stray key seeks or pauses a camera.
--
-- A picture is the frame on screen, turned back the way others see it, as
-- the camera gives it, and is saved and copied. A video is recorded from the
-- camera with the default microphone by a program of its own, which outlives
-- this window, so closing it mid-video loses nothing. A camera gives its
-- frames to one reader at a time, so while it records, that program has the
-- camera and the viewfinder shows what it passes on. Each ends with a notice
-- saying where it went.
local options = {
  mode = 'picture',
  device = '',
  picture = '',
  video = '',
  -- The theme's, as home.nix gives them.
  sans = 'sans-serif',
  mono = 'monospace',
  deepest = '',
  shadow = '',
  edge = '',
  text = '',
  muted = '',
  strong = '',
  action = '',
  alert = '',
  opacity = 0.85,
}
require('mp.options').read_options(options, 'camera')

local mode = options.mode
local camera = 'av://v4l2:' .. options.device
local recording = nil
-- Escape closes once a recording has finished, or at once when pressed again.
local closing = false
local ready = false
local flash = false
local overlay = mp.create_osd_overlay('ass-events')

-- The panel's words are drawn here first, unseen, to learn how wide they
-- are, since the words' face is proportional.
local measure = mp.create_osd_overlay('ass-events')
measure.hidden, measure.compute_bounds = true, true

local function wide(text, width, height)
  measure.res_x, measure.res_y = width, height
  measure.data = '{\\an7\\pos(0,0)}' .. text
  local bounds = measure:update()
  return bounds and bounds.x1 and bounds.x1 - bounds.x0 or 0
end

-- An ASS colour and alpha from the theme's #rrggbb and an opacity.
local function colour(hex)
  return string.format('&H%s%s%s&', hex:sub(6, 7), hex:sub(4, 5), hex:sub(2, 3))
end

local function alpha(opacity)
  return string.format('&H%02X&', math.floor((1 - opacity) * 255 + 0.5))
end

-- A rounded box as an ASS drawing.
local function box(x0, y0, x1, y1, r)
  local k = r * 0.5523
  return string.format(
    'm %d %d l %d %d b %d %d %d %d %d %d l %d %d b %d %d %d %d %d %d '
      .. 'l %d %d b %d %d %d %d %d %d l %d %d b %d %d %d %d %d %d',
    x0 + r,
    y0,
    x1 - r,
    y0,
    x1 - r + k,
    y0,
    x1,
    y0 + r - k,
    x1,
    y0 + r,
    x1,
    y1 - r,
    x1,
    y1 - r + k,
    x1 - r + k,
    y1,
    x1 - r,
    y1,
    x0 + r,
    y1,
    x0 + r - k,
    y1,
    x0,
    y1 - r + k,
    x0,
    y1 - r,
    x0,
    y0 + r,
    x0,
    y0 + r - k,
    x0 + r - k,
    y0,
    x0 + r,
    y0
  )
end

local function elapsed()
  local seconds = math.floor(mp.get_time() - (recording.began or mp.get_time()))
  return string.format('%d:%02d', math.floor(seconds / 60), seconds % 60)
end

-- What the keys do now, as the launcher's footer says it: the key in the
-- text's colour, its word muted.
local function hints()
  if closing then
    return { { 'Esc', 'close now' } }
  elseif recording and recording.stopping then
    return { { 'Esc', 'close' } }
  elseif recording then
    return { { '↵', 'stop' } }
  end
  local other = mode == 'picture' and 'video' or 'picture'
  return {
    { '↵', mode == 'picture' and 'take' or 'record' },
    { '⇥', other },
    { 'Esc', 'close' },
  }
end

local function draw()
  local width, height = mp.get_osd_size()
  if not width or width == 0 then
    return
  end
  local scale = mp.get_property_number('display-hidpi-scale', 1)
  local unit = function(px)
    return math.floor(px * scale + 0.5)
  end
  local size = unit(17.5)
  local events = {}

  -- The shutter: the frame lit for a moment as it's taken.
  if flash then
    table.insert(
      events,
      string.format(
        '{\\an7\\pos(0,0)\\bord0\\shad0\\1c%s\\1a%s\\p1}%s{\\p0}',
        colour(options.strong),
        alpha(0.35),
        box(0, 0, width, height, 0)
      )
    )
  end

  -- On the left, what Return makes and, while recording, for how long;
  -- on the right, the keys. Words, a key's name among them, are in the
  -- interface's face; marks, the time and the spaces between in the
  -- monospaced one, which has the marks, keeps the time's figures from
  -- shifting as they count, and spaces the keys as evenly as before.
  local font = string.format('{\\fs%d\\bord0\\shad0}', size)
  local words = '\\fn' .. options.sans
  local marks = '\\fn' .. options.mono
  local left
  if not ready then
    left = string.format('{%s\\1c%s}Waking the camera…', words, colour(options.muted))
  else
    local mark = recording and options.video
      or (mode == 'picture' and options.picture or options.video)
    local timing = recording and not recording.stopping
    local word = recording and (recording.stopping and 'Saving' or elapsed())
      or (mode == 'picture' and 'Picture' or 'Video')
    left = string.format(
      '{%s\\1c%s}%s  {%s\\1c%s}%s',
      marks,
      colour(recording and options.alert or options.action),
      mark,
      timing and marks or words,
      colour(options.strong),
      word
    )
  end
  local parts = {}
  for _, hint in ipairs(ready and hints() or { { 'Esc', 'close' } }) do
    table.insert(
      parts,
      string.format(
        '{%s\\1c%s}%s{%s} {%s\\1c%s}%s',
        hint[1]:match('^%a') and words or marks,
        colour(options.text),
        hint[1],
        marks,
        words,
        colour(options.muted),
        hint[2]
      )
    )
  end
  local right = table.concat(parts, '{' .. marks .. '}   ')
  left, right = font .. left, font .. right

  local inner = unit(16)
  local panel_width =
    math.floor(inner * 2 + wide(left, width, height) + wide(right, width, height) + unit(24))
  local panel_height = unit(40)
  local x0 = math.floor((width - panel_width) / 2)
  local y1 = height - unit(24)
  local y0 = y1 - panel_height
  local x1 = x0 + panel_width
  local radius = unit(8)
  local middle = math.floor((y0 + y1) / 2)

  -- A floating surface: its shadow, then the panel in the deepest colour
  -- at the theme's opacity, inside the edge's line.
  table.insert(
    events,
    string.format(
      '{\\an7\\pos(0,%d)\\bord0\\shad0\\blur%d\\1c%s\\1a%s\\p1}%s{\\p0}',
      unit(2),
      unit(6),
      colour(options.shadow),
      alpha(0.35),
      box(x0, y0, x1, y1, radius)
    )
  )
  table.insert(
    events,
    string.format(
      '{\\an7\\pos(0,0)\\bord%d\\shad0\\1c%s\\1a%s\\3c%s\\p1}%s{\\p0}',
      math.max(1, unit(1)),
      colour(options.deepest),
      alpha(options.opacity),
      colour(options.edge),
      box(x0, y0, x1, y1, radius)
    )
  )
  table.insert(events, string.format('{\\an4\\pos(%d,%d)}%s', x0 + inner, middle, left))
  table.insert(events, string.format('{\\an6\\pos(%d,%d)}%s', x1 - inner, middle, right))

  overlay.res_x, overlay.res_y = width, height
  overlay.data = table.concat(events, '\n')
  overlay:update()
end

-- Started by niri, with this program's PATH, since what mpv starts keeps the
-- camera open: a picture's wl-copy, staying to serve the clipboard, would
-- hold it, its light on, until something else was copied.
local path = os.getenv('PATH') or ''

local function run(script, ...)
  mp.commandv(
    'run',
    'niri',
    'msg',
    'action',
    'spawn',
    '--',
    'env',
    'PATH=' .. path,
    'sh',
    '-c',
    script,
    'camera',
    ...
  )
end

local function stamp()
  return os.date('%Y-%m-%d %H-%M-%S')
end

local home = os.getenv('HOME')
local runtime = os.getenv('XDG_RUNTIME_DIR') or '/tmp'

local shots = 0

-- Saves the frame on screen, turned back, then copies it and says where it
-- went. The camera counts as awake once mpv knows the picture's size, which
-- can come before there's a frame to save, so a frame that isn't saved is
-- asked for again a moment later, for two seconds at most, and then saved
-- as it is, for the notice to say why it failed.
local function save(frame, file, tries)
  -- A screenshot that saved returns nothing; one that didn't, an error.
  local _, failed = mp.command_native({ 'screenshot-to-file', frame, 'video' })
  if failed and tries < 40 then
    mp.add_timeout(0.05, function()
      save(frame, file, tries + 1)
    end)
    return
  end
  flash = true
  draw()
  -- The shutter's moment shown, the viewfinder goes: the picture is saved
  -- by a program of its own, which outlives this window.
  mp.add_timeout(0.12, function()
    mp.command('quit')
  end)
  run(
    table.concat({
      'failed() {',
      '    rm -f "$1"',
      '    notify-send --app-name=Camera "The picture could not be saved" "$2"',
      '    exit 1',
      '}',
      'folder=$(dirname "$2")',
      'mkdir -p "$folder" || failed "$1" "${folder#"$HOME"/} could not be made"',
      -- A second picture in the same second is numbered, not lost; the
      -- name is claimed before it's written, so two at once can't share it.
      'file=$2.jpg n=2',
      'until (set -C; : > "$file") 2> /dev/null; do',
      '    [ -e "$file" ] || failed "$1" "${folder#"$HOME"/} could not be written to"',
      '    file="$2 $n.jpg" n=$((n + 1))',
      'done',
      'said=$(ffmpeg -loglevel error -y -i "$1" -vf hflip -q:v 2 "$file" 2>&1) ||',
      '    { rm -f "$file"; failed "$1" "$(printf %s "$said" | tail -n 1)"; }',
      'rm -f "$1"',
      'wl-copy --type image/jpeg < "$file"',
      'notify-send --app-name=Camera --transient "Picture copied" "${file#"$HOME"/}"',
    }, '\n'),
    frame,
    file
  )
end

local function take()
  local file = string.format('%s/Pictures/Camera/%s', home, stamp())
  shots = shots + 1
  local frame = string.format('%s/camera-%s-%d.png', runtime, mp.get_property('pid'), shots)
  save(frame, file, 0)
end

-- What the viewfinder plays, and what to do once it has let the last one go.
local playing = camera
local released = nil

local function play(source, after)
  playing, released = source, after
  mp.commandv('loadfile', source, 'replace')
end

-- Starts recording into a new file, with the microphone, once the viewfinder
-- has let the camera go for the pipe the recording feeds it through.
--
-- The recording is told to stop by a file saying so, then a signal to the
-- program its pid file names. Each side checks after the other has acted, so
-- a stop asked for as it starts is never lost. The pipe stays open until the
-- recording has let the camera go, so the viewfinder can take it back, and
-- opening the pipe frees a recording left waiting for a viewfinder that's
-- gone.
local function start()
  local file = string.format('%s/Videos/Camera/%s', home, stamp())
  local feed = string.format('%s/camera-%d.nut', runtime, math.floor(mp.get_time() * 1000))
  local own = string.format('%s/camera-%s', runtime, mp.get_property('pid'))
  recording = { pid = own .. '.pid', stop = own .. '.stop', feed = feed }
  mp.command_native({
    name = 'subprocess',
    args = {
      'sh',
      '-c',
      'rm -f "$1" "$2" && mkfifo "$3"',
      'camera',
      recording.stop,
      recording.pid,
      feed,
    },
    playback_only = false,
  })
  play(feed, function()
    run(
      table.concat({
        'device=$1 file=$2 pid=$3 feed=$4 log=$5 stop=$6 state=$7',
        -- Waits for the viewfinder to open the pipe.
        'exec 3> "$feed" || exit',
        'rm -f "$feed"',
        '[ -e "$stop" ] && exit',
        'failed() {',
        '    notify-send --app-name=Camera "The video could not be recorded" "$1"',
        '    exit 1',
        '}',
        'folder=$(dirname "$file")',
        'mkdir -p "$folder" || failed "${folder#"$HOME"/} could not be made"',
        'file=$2.mp4 n=2',
        'until (set -C; : > "$file") 2> /dev/null; do',
        '    [ -e "$file" ] || failed "${folder#"$HOME"/} could not be written to"',
        '    file="$2 $n.mp4" n=$((n + 1))',
        'done',
        -- Intel's and AMD's drivers encode through VA-API.
        'case $(readlink /sys/class/drm/renderD128/device/driver) in',
        '    */i915 | */xe | */amdgpu)',
        '        set -- -vaapi_device /dev/dri/renderD128 -vf format=nv12,hwupload',
        '        set -- "$@" -c:v h264_vaapi -qp 20 ;;',
        '    *) set -- -c:v libx264 -preset veryfast -crf 20 -pix_fmt yuv420p ;;',
        'esac',
        -- Fragments, so a recording cut short still plays; the frames as
        -- they came for the viewfinder, which may close before the end.
        'ffmpeg -loglevel error -y -use_wallclock_as_timestamps 1 -thread_queue_size 64 \\',
        '    -f v4l2 -i "$device" -thread_queue_size 1024 -f pulse -i default \\',
        '    -map 0:v -map 1:a "$@" -c:a aac -b:a 160k \\',
        '    -movflags +frag_keyframe+empty_moov+default_base_moof "$file" \\',
        '    -map 0:v -c:v rawvideo -f nut pipe:3 < /dev/null 2> "$log" &',
        'echo $! > "$pid"',
        -- Said for the shell while it records, which it shows as recording
        -- and can stop as Return here does (Recorder.qml), and taken back
        -- however this ends.
        [[trap 'rm -f "$state"' EXIT]],
        [[printf '{"pid":%s,"stop":"%s","file":"%s"}\n' $! "$stop" "$file" > "$state.new"]],
        'mv "$state.new" "$state"',
        '[ -e "$stop" ] && kill -INT $!',
        'wait $!',
        'rm -f "$pid" "$stop" "$state"',
        'exec 3>&-',
        'if [ ! -s "$file" ]; then',
        '    rm -f "$file"',
        '    failed "$(tail -n 1 "$log")"',
        'fi',
        'asked=$(notify-send --app-name=Camera --action=play="Play it" \\',
        '    "Video saved" "${file#"$HOME"/}")',
        '[ "$asked" = play ] && exec xdg-open "$file"',
      }, '\n'),
      options.device,
      file,
      recording.pid,
      feed,
      -- What the last recording said, kept until the next.
      runtime .. '/camera-recording.log',
      recording.stop,
      runtime .. '/camera-recording.json'
    )
  end)
end

local ticking = mp.add_periodic_timer(1, draw)
ticking:kill()

-- Asks the recording to stop; the viewfinder takes the camera back once its
-- pipe ends.
local function finish()
  if not recording or recording.stopping then
    return
  end
  recording.stopping = true
  ticking:kill()
  run(
    table.concat({
      ': > "$1"',
      '[ -f "$2" ] && kill -INT "$(cat "$2")"',
      '[ -p "$3" ] && exec 3<> "$3"',
      'true',
    }, '\n'),
    recording.stop,
    recording.pid,
    recording.feed
  )
end

local function close()
  if recording and not closing then
    closing = true
    finish()
    draw()
  else
    mp.command('quit')
  end
end

-- Return pressed before the camera wakes is done once it has, so a picture
-- asked for at once is taken as soon as there is one.
local waiting = false

local function act()
  if not ready then
    waiting = true
    return
  end
  if mode == 'picture' then
    take()
  elseif recording then
    close()
  else
    start()
  end
  draw()
end

mp.add_forced_key_binding('ENTER', 'camera-act', act)
mp.add_forced_key_binding('KP_ENTER', 'camera-act-pad', act)
mp.add_forced_key_binding('SPACE', 'camera-act-space', act)
mp.add_forced_key_binding('TAB', 'camera-mode', function()
  if recording then
    return
  end
  mode = mode == 'picture' and 'video' or 'picture'
  draw()
end)
mp.add_forced_key_binding('ESC', 'camera-close', close)
mp.add_forced_key_binding('q', 'camera-quit', close)

-- Once the viewfinder has let a source go: the camera handed to the
-- recording, or taken back once the recording's pipe ends. The camera
-- itself ending means it's gone.
mp.register_event('end-file', function(event)
  if event.reason == 'quit' then
    return
  end
  local after = released
  released = nil
  if after then
    after()
  elseif playing ~= camera then
    recording = nil
    ticking:kill()
    if closing then
      mp.command('quit')
    else
      play(camera, nil)
      draw()
    end
  else
    mp.command('quit')
  end
end)

-- Ready once the camera's first frame is shown, and still ready while the
-- viewfinder changes source. A recording's time starts with its first frame.
mp.observe_property('video-params', 'native', function(_, params)
  ready = ready or params ~= nil
  if params and recording and playing == recording.feed and not recording.began then
    recording.began = mp.get_time()
    ticking:resume()
  end
  draw()
  if ready and waiting then
    waiting = false
    act()
  end
end)
mp.observe_property('osd-dimensions', 'native', draw)
-- The window closed some other way finishes a video too.
mp.register_event('shutdown', finish)
