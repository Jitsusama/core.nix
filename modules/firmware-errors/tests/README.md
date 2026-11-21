# What firmware-errors Is Tested Against

`three-strike-timeout.crashlog` is Intel's own sample of a Crash Log from a
core's three-strike timeout, `lib/tests/samples/three_strike_timeout_with_xq.crashlog`
in [intel/crashlog][1] at commit 8c924fa75e1a, which its own triage test reads
as `CORE_TIMEOUT.SINGLE_STUCK_TRANSACTION.13014002340H` and
`MCA.BANK3.INTERNAL_TIMER_ERROR.MSCOD_E184H`. `src/crash.rs` reads it as the
kernel would show the region. A machine's own Crash Logs stay off the
repository, since one holds its registers and memory addresses. The health
tables the tests read are built in `src/phat.rs`, laid out as the
specification and a real table are.

`a-few-kept-boots.sh` builds the same tables as files, four boots' worth
with error regions empty and cut short, for `commands-agree` to set the
command's list and one boot's facts at each width from.

The sample is Intel's, under the MIT licence:

```text
Copyright (C) 2025 Intel Corporation

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"),
to deal in the Software without restriction, including without limitation
the rights to use, copy, modify, merge, publish, distribute, sublicense,
and/or sell copies of the Software, and to permit persons to whom
the Software is furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included
in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS
OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.  IN NO EVENT SHALL
THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES
OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE,
ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE
OR OTHER DEALINGS IN THE SOFTWARE.
```

[1]: https://github.com/intel/crashlog
