# Your Home Network, Now

Every device on the home network this minute, drawn floor by floor. The
page watches net's records live over the sites' server, and redraws the
moment net has read the network again (about once a minute). The line
under the title says whether it's live, and since when. Nothing on it
changes the network or any device on it.

## How to Read It

- **One band per floor.** Each floor is named at the left with how many
  devices are on it, and holds that floor's eero.
- **An eero's box** gives its model, what it does (router, access point)
  and its address, then a row for each radio: its band, its channel, how
  busy the air on that channel is, and how many devices are on it. Its
  last line says how it reaches the eero it hangs from, or, for the
  router, its firmware and how long it has been up.
- **Devices sit beside the eero they're on**, in columns, each with its
  name, its address and its signal in dBm (nearer 0 is stronger), or the
  switch port or cable it comes in on.
- **The switch and the modem sit with the router.** The switch shows its
  ports, lit when linked; the modem shows its cable's channels, their
  power and signal-to-noise, and how many codewords it couldn't correct.
  The internet's provider and its last measured speeds sit left of the
  modem.
- **Links:** a solid line is a cable, a dashed one is Wi-Fi. Lines between
  eeros run down the left, one lane each.
- **Gold with a diamond means worth a look:** a radio busy half the time
  or more, a device heard at −70 dBm or weaker, a link slower than the
  rest of its route, one modem channel losing far more data than the
  others, or a device using 80% or more of the internet's speed one way.
  Each is also named in words in the panel under Worth a Look,
  so nothing is told by colour alone. The internet being the narrowest
  point is said under The Internet, since it isn't a fault.

## A Device's Route to the Internet

Opening a device (Return, or a click) lights its way to the internet on
the map and lists it in the panel, hop by hop: each hop's medium, speed
and signal. Below it, for each way (down and up), the hop with the least
room, as a rate. A Wi-Fi hop's room is its speed less the time its busier
radio is busy; a hop with no figure is counted and never guessed.

Opening a device puts its name after the address's #, so the address
opens the page on it, with the map holding the keys there:
`#kitchen-printer` opens the printer. Where two devices share a name, it
takes net's id instead, and a link by id opens the device too, as the
launcher's do.

A device eero counts gets Its Speed: what it carries each way now, and
the most it carried in eero's last hour, with when, each as a rate.

## Keys

| Key               | Does                                         |
| ----------------- | -------------------------------------------- |
| Arrows, h j k l   | Walk the map to the nearest device that way  |
| Return            | Open the device the keys rest on             |
| -                 | Step out to the eero it hangs from           |
| Escape            | Close the details of the opened device       |
| Shift+arrows      | Scroll the map                               |
| i, +, o           | Zoom in and out                              |
| 0                 | Fit the map to the room                      |
| /                 | Find a device by name, address or maker      |
| Tab               | Move between the map, the panel and the tree |
| ? or F1           | List every key                               |

In the tree, j and k (or the arrows) move, l and h open and close a
floor, g and G go to the first and last, and Return opens a device.

## What the Data Holds

The page watches net's daemon at /feed/net, one kind of record at a
time: each source and whether it answered (eero, the modem's status
pages, the switch's own reader, Sony's receivers, HP's printers, the
Apple TVs and the Samsung TVs), every device with its roles, maker, model, addresses, ports,
radios and speed, and its path to the modem with its narrowest hop each
way, every link between two devices with its medium, speed and signal,
the internet's provider and speeds, and the concerns worth a look, each
with its rank, worst first. At a terminal, `net` shows the same record,
and `net devices NAME` one device in full.
