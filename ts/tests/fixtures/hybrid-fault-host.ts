import { appendFile, readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
const protocol = 'rein-extension/0.1', mode = process.env.REIN_HYBRID_FAULT_MODE ?? 'normal', workspace = process.env.REIN_HYBRID_WORKSPACE ?? process.cwd(), ledger = process.env.REIN_HYBRID_LEDGER, marker = process.env.REIN_HYBRID_FAULT_MARKER;
type M = Record<string, unknown>;
type I = M & {
    sessionId: string;
    requestId: string;
    taskId: string;
    callId: string;
    path: string;
    ruleId: string;
    targetVersion: string;
};
let active: I | undefined, tail = '', terminal = false;
const out = (v: M) => process.stdout.write(JSON.stringify(v) + '\n');
const ids = (r: I): M => ({ sessionId: mode === 'wrong_session' ? 'wrong' : r.sessionId, requestId: r.requestId, taskId: r.taskId, callId: r.callId });
const ev = (r: I): M => ({ taskId: r.taskId, callId: r.callId, path: r.path, ruleId: r.ruleId, targetVersion: r.targetVersion });
const close = (code = 0) => { process.stdin.pause(); process.stdin.destroy(); process.stdout.end(() => process.exit(code)); };
const term = (r: I, status: string, x: M = {}) => { if (terminal)
    return; terminal = true; out({ protocol, type: 'terminal', ...ids(r), status, ...x }); close(); };
async function invoke(r: I) { active = r; if (mode === 'crash_after_invoke')
    process.exit(17); out({ protocol, type: 'started', ...(mode === 'wrong_started' ? { ...ids(r), callId: 'wrong-started' } : ids(r)) }); if (mode === 'partial_frame_cancel') {
    const s = JSON.stringify({ protocol, type: 'terminal', ...ids(r), status: 'cancelled' });
    tail = s.slice(Math.floor(s.length / 2));
    process.stdout.write(s.slice(0, Math.floor(s.length / 2)));
    return;
} if (mode === 'oversize_no_newline') {
    process.stdout.write('x'.repeat(300 * 1024));
    return;
} if (mode === 'terminal_hang') {
    const c = await readFile(resolve(workspace, r.path), 'utf8');
    out({ protocol, type: 'terminal', ...ids(r), status: 'succeeded', output: c, evidence: ev(r) });
    return;
} if (['cancellation_wait', 'ignore_cancel', 'wrong_cancel_ack_ids', 'cancel_ack_trailing_frame', 'cancel_ack_nonzero_exit'].includes(mode))
    return; if (mode === 'disconnect_after_effect' && ledger) {
    await appendFile(ledger, `${r.requestId}\n`);
    process.exit(17);
} if (mode === 'stale_target' && marker)
    await writeFile(marker, 'stale-target\n'); const content = await readFile(resolve(workspace, r.path), 'utf8'); const e = ev(r); if (mode.startsWith('wrong_evidence_')) {
    const k = mode.slice(15);
    if (k === 'task')
        e.taskId = 'wrong';
    if (k === 'call')
        e.callId = 'wrong';
    if (k === 'path')
        e.path = 'wrong';
    if (k === 'rule')
        e.ruleId = 'wrong';
    if (k === 'version')
        e.targetVersion = 'wrong';
} if (mode === 'invalid_json') {
    process.stdout.write('{invalid-json\n');
    close();
    return;
} if (mode === 'wrong_id') {
    out({ protocol, type: 'terminal', ...ids(r), requestId: 'wrong', status: 'succeeded', output: content, evidence: e });
    close();
    return;
} if (mode === 'wrong_version') {
    out({ protocol: 'other/9', type: 'terminal', ...ids(r), status: 'succeeded', output: content, evidence: e });
    close();
    return;
} if (mode === 'contradictory') {
    out({ protocol, type: 'terminal', ...ids(r), status: 'succeeded', output: content, error: { code: 'bad', message: 'bad' }, evidence: e });
    close();
    return;
} if (mode === 'duplicate') {
    const m = { protocol, type: 'terminal', ...ids(r), status: 'succeeded', output: content, evidence: e };
    out(m);
    out(m);
    close();
    return;
} if (mode === 'wrong_terminal_session') {
    out({ protocol, type: 'terminal', ...ids(r), sessionId: 'wrong', requestId: r.requestId, taskId: r.taskId, callId: r.callId, status: 'succeeded', output: content, evidence: e });
    close();
    return;
} term(r, 'succeeded', { output: content, evidence: e }); }
if (mode === 'crash_before_ready')
    process.exit(17);
if (mode !== 'ready_hang')
    out({ protocol, type: 'ready', capabilities: ['read_file'], maxMessageBytes: 262144 });
let pending = '';
process.stdin.setEncoding('utf8');
process.stdin.on('data', (chunk: string) => { pending += chunk; for (let i = pending.indexOf('\n'); i >= 0; i = pending.indexOf('\n')) {
    const line = pending.slice(0, i);
    pending = pending.slice(i + 1);
    try {
        const m = JSON.parse(line) as M;
        if (m.type === 'invoke')
            void invoke(m as I);
        else if (m.type === 'cancel' && active && m.sessionId === active.sessionId && m.requestId === active.requestId && m.taskId === active.taskId && m.callId === active.callId) {
            if (mode === 'ignore_cancel')
                continue;
            if (mode === 'partial_frame_cancel') {
                process.stdout.write(tail + '\n', () => close());
                continue;
            }
            if (mode === 'wrong_cancel_ack_ids') {
                out({ protocol, type: 'terminal', ...ids({ ...active, callId: 'wrong' } as I), status: 'cancelled' });
                close();
                continue;
            }
            if (mode === 'cancel_ack_nonzero_exit') {
                out({ protocol, type: 'terminal', ...ids(active), status: 'cancelled' });
                close(17);
                continue;
            }
            if (mode === 'cancel_ack_trailing_frame') {
                out({ protocol, type: 'terminal', ...ids(active), status: 'cancelled' });
                out({ protocol, type: 'terminal', ...ids(active), status: 'cancelled' });
                close();
                continue;
            }
            if (mode === 'cancellation_wait')
                term(active, 'cancelled');
        }
    }
    catch {
        out({ protocol, type: 'error', code: 'malformed_json', message: 'invalid JSON' });
    }
} });
