# Document maintenance boundary v1

Rust core owns task scope, per-dispatch policy, candidates, approval, attempts and adoption. The Node host implements declared handlers under the extension 0.2 generic envelope. A candidate is an exact UTF-8 replacement for one existing workspace-relative regular file, with a SHA-256 baseline and displayed diff. No delete, rename, creation or symlink support in this teaching slice. This restricted representation is still a reviewable patch; it does not claim arbitrary git-patch support.

Approval is obtained by the local caller, bound to candidate digest and current baseline; it is never accepted from model arguments. Each changed candidate requires fresh approval. Before application, core and host both recheck file identity and baseline. Multiple candidates apply serially against the actual current state. Validation checks actual post-state; a tool success is not task acceptance.

Tools: read_file, search_files, run_verification, apply_patch. The latter two are exposed only in a configured maintenance host and enabled core policy. `run_verification` accepts a fixed rule identifier and bound target; never a free-form shell string. The reference runner uses a trusted bundled Node verifier with shell:false, bounded output and deadline, checks command declarations/help/link targets in the fixture. Runtime capability advertisement must exactly match registered handlers.

After known validation failure, the caller may produce at most two repair candidates and must approve each anew. OutcomeUnknown stops for human inspection; no automatic retry. Attempt IDs are distinct, attempts consume budget, and not-dispatched differs from completed failure.

fixtures/document-maintenance/cases.json contains 12 authored cases; expected is an evaluator-only oracle and cannot be input to candidate generation. Offline fixture adapters may be deterministic but must consume actual tool outputs. Model effects, token counts, cost, and live latency cannot be inferred from these runs.
