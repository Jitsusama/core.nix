# What the Home Network's Devices Answer

What each source net reads answers, asked once of real equipment and kept
here, redacted, so the collector is tested against what it will meet.
eero's answers aren't here: net reads the eero daemon's record, whose
golden copy is `../../eero/home-network.json`.

- `modem/`: a DOCSIS 3.1 cable modem's nine status pages, as
  `http://192.168.100.1/data/<page>.asp` answered them on 2026-11-14
  without signing in, one file per page. Its MAC addresses, serial and
  public address are stand-ins (`02:00:00:00:10:01` for its own,
  `02:00:00:00:10:02` for the provider's, `SERIAL0010`, `100.64.0.98`);
  every figure is as the modem gave it, in its own strings.
- `switch/`: an 8-port TP-Link Easy Smart switch's four status pages, as
  `http://<switch>/<page>.htm` answered them on 2026-11-14 once signed
  in, one file per page, byte for byte apart from its name, hardware
  address and addresses (`Closet`, `02:00:00:00:00:30`, `10.0.0.x`). Its
  cable page holds no test, since none had run; the cases in
  `src/sources/switch.rs` give it one.
- `sony/`: a Sony STR-DN1050 receiver's nine answers, asked on
  2026-11-15 without registering as its remote: its UPnP description,
  five SOAP reads (`GetVolume`, `GetMute`, `GetTransportInfo`,
  `GetMediaInfo`, `GetPositionInfo`), its `RenderingControl` service's
  description, its CERS action list and
  `getSystemInformation`, one file each, byte for byte apart from its
  address, the hardware address in its UDN and its name's suffix
  (`10.0.0.60`, `020000000060`, `000060`).
- `hp/`: an HP OfficeJet 3830's four LEDM pages, as
  `http://<printer>/DevMgmt/<page>.xml` answered them on 2026-11-15, one
  file per page, byte for byte apart from its addresses (`10.0.0.70`,
  `10.0.1.1`, `FE80::FF:FE00:70`), its hardware address
  (`020000000070`), its serial (`SERIAL00000070`), its UUID, its country
  and the ids of its firmware and cartridges (`0000`).
- `airplay/`: an Apple TV 4K's answer to `http://<apple-tv>:7000/info`,
  asked on 2026-11-15 without pairing, as `info.plist`, the binary
  property list it sent, read and written again with its name, its
  hardware address and device id (`Den`, `02:00:00:00:00:80`), the
  address it saw the ask from (`10.0.0.41`) and its pairing keys (zeros
  and stand-in UUIDs) replaced; every other field is as it answered.
- `samsung/`: a Samsung Q70C TV's answer to
  `http://<tv>:8001/api/v2/`, asked on 2026-11-20 without pairing, as
  `api-v2.json`, laid out again with its name, its address, its
  hardware address, the hardware address of the radio it was on and its
  ids replaced (`Living Room`, `10.0.0.90`, `02:00:00:00:00:90`,
  `02:00:00:00:20:01`, a zero UUID); every other field is as it
  answered.
