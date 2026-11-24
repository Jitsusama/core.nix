# The Live Sites' Kit's Tests

Each `*.test.ts` proves one of the kit's scripts against a stand-in socket
(`stand-in.ts`), a clock moved by hand (`hands.ts`) and a list of rows that
takes focus as a page's does (`rows.ts`), which `keyed`'s drawing and focus
tests share. Node runs them as the kit builds, which is the
`local-sites-kit-holds` check.
