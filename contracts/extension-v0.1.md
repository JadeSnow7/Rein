# Extension protocol `rein-extension/0.1`

The Rust core owns session, task, request, and call identifiers, the allowed
tool policy, sequencing, budgets, and terminal state. A single Node host is
spawned with one in-flight `invoke`. Messages are JSON Lines on stdout; host
diagnostics belong on stderr. The host first sends `ready`, including a
positive `maxMessageBytes` limit. Core then sends one `invoke`; `started` must
precede `terminal`, and every response repeats the complete identity tuple.

`invoke` carries `sessionId`, `requestId`, `taskId`, `callId`, `tool`, relative
`path`, `ruleId`, and `targetVersion`. The only milestone rule is
`read-file-content-v1`. The host returns one strict `terminal` union:
`succeeded` has output and evidence, `tool_failed` has error, and `cancelled`
has neither. The evidence reference contains task, call, path, rule, and
target version. Core validates
the binding and current target SHA-256 before adopting content.

Malformed JSON, a wrong protocol version or identifier, contradictory or
duplicate terminal messages are fatal and produce `outcome_unknown` after a
request may have arrived. A normal read has a default five-second I/O budget.
Cancellation sends at most one fully bound `cancel`, then allows a one-second
grace period. The host is accepted only after strict terminal union validation,
clean EOF, and exit code 0; otherwise cleanup produces `outcome_unknown`.
Unknown outcomes are recorded in memory and are never replayed automatically.
The records are not a durable journal. A trusted first-party Node process is
not an operating-system sandbox. This milestone has no durable recovery,
approval UI, exactly-once guarantee, or live service.
