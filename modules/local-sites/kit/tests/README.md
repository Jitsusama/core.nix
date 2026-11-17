# The Live Sites' Kit's Tests

Each `*.test.ts` proves one of the kit's scripts against a stand-in socket
(`stand-in.ts`) and a clock moved by hand (`hands.ts`). Node runs them as the kit
builds, which is the `local-sites-kit-holds` check.
