# `rein-extension/0.2`

The 0.2 extension is a JSON-lines protocol between the Rust core and a Node
tool host. Every frame is one UTF-8 JSON object followed by `\n`; frames larger
than 256 KiB are rejected. The identity tuple `(sessionId, requestId, taskId,
callId)` is required on `invoke`, `started`, and `terminal` frames.

The host first sends `ready` with `tools`, each containing a generic
`argumentsSchema` and `resultSchema`, plus `capabilities` and
`maxMessageBytes`. The default declaration contains only `read_file` and
`search_files`. An explicitly enabled maintenance profile adds
`run_verification` and `apply_patch`; its core policy binds the target and
verification rule independently of the ready declaration. A declaration is not permission: the core validates the tool,
arguments, workspace policy, and evidence target before dispatch.

An invocation has this stable shape:

```json
{"protocol":"rein-extension/0.2","type":"invoke","sessionId":"s","requestId":"r","taskId":"t","callId":"c","tool":"read_file","arguments":{"path":"README.md"},"evidenceTarget":"sha256:..."}
```

`terminal.status` is `succeeded`, `tool_failed`, or `cancelled`. Successful
and failed calls carry a generic `result` object; successful calls also carry
evidence binding the identity to the requested target. A `cancel` frame uses
the same identity tuple. The core allows one in-flight invocation and treats
missing or ambiguous terminal output as `outcome_unknown`; it must release
the child process before returning. Controlled verification and patch application reuse this envelope and add
declarations rather than changing wire fields. See `maintenance-session-v1.md`
for the caller approval and post-state adoption boundary.

Version 0.1 remains available under its original read-file-only meaning for
historical examples and is not silently upgraded.
