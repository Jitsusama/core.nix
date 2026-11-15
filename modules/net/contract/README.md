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
- `sony/`: a Sony STR-DN1050 receiver's eight answers, asked on
  2026-11-15 without registering as its remote: its UPnP description,
  five SOAP reads (`GetVolume`, `GetMute`, `GetTransportInfo`,
  `GetMediaInfo`, `GetPositionInfo`), its CERS action list and
  `getSystemInformation`, one file each, byte for byte apart from its
  address, the hardware address in its UDN and its name's suffix
  (`10.0.0.60`, `020000000060`, `000060`).
