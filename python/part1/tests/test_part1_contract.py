import hashlib
import json
import subprocess
import sys
import tempfile
import asyncio
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
PART1 = ROOT / "python" / "part1"
sys.path.insert(0, str(PART1))


class ContractBaselineTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        # These imports intentionally fail before implementation is approved.
        from rein_core import (  # noqa: F401
            ModelAdapter,
            ModelResponse,
            ReinError,
            RunRecord,
            ReplayAdapter,
            Task,
            ToolCall,
            ToolResult,
            run,
            safe_read,
            verify_source,
        )
        globals()["ReinError"] = ReinError

    def test_safe_read_accepts_fixture_and_keeps_raw_digest(self):
        from rein_core import safe_read

        root = PART1 / "fixtures" / "outdated"
        result = safe_read(root, "README.md")
        raw = (root / "README.md").read_bytes()
        self.assertEqual(result.digest, hashlib.sha256(raw).hexdigest())
        self.assertIn("npm run start", result.text)

    def test_safe_read_rejects_boundaries(self):
        from rein_core import safe_read

        root = PART1 / "fixtures"
        with self.assertRaises(ReinError) as caught:
            safe_read(root, "/etc/hosts")
        self.assertEqual(caught.exception.code, "path_invalid")
        with self.assertRaises(ReinError) as caught:
            safe_read(root, "../outdated/README.md")
        self.assertEqual(caught.exception.code, "path_invalid")
        with self.assertRaises(ReinError) as caught:
            safe_read(root, "outdated")
        self.assertEqual(caught.exception.code, "path_invalid")
        with tempfile.TemporaryDirectory() as tmp:
            bad = Path(tmp) / "bad.bin"
            bad.write_bytes(b"ok\xff")
            with self.assertRaises(ReinError) as caught:
                safe_read(Path(tmp), "bad.bin")
        self.assertEqual(caught.exception.code, "invalid_utf8")

        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "allowed").mkdir()
            (root / "allowed" / "ok.txt").write_text("ok", encoding="utf-8")
            try:
                (root / "link.txt").symlink_to(root / "allowed" / "ok.txt")
            except (OSError, NotImplementedError):
                self.skipTest("symlinks unavailable")
            with self.assertRaises(ReinError) as caught:
                safe_read(root, "link.txt")
            self.assertEqual(caught.exception.code, "path_invalid")

    def test_exact_four_kibibyte_limit(self):
        from rein_core import safe_read

        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "exact.txt").write_bytes(b"a" * 4096)
            (root / "large.txt").write_bytes(b"a" * 4097)
            self.assertEqual(safe_read(root, "exact.txt").size, 4096)
            with self.assertRaises(ReinError) as caught:
                safe_read(root, "large.txt")
            self.assertEqual(caught.exception.code, "file_too_large")

    def test_crlf_and_multiline_are_preserved_for_digest(self):
        from rein_core import safe_read, locate_suggestion, verify_source

        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            raw = b"one\r\nrun npm run start here\r\nthree\r\n"
            (root / "x.md").write_bytes(raw)
            source = safe_read(root, "x.md")
            result = locate_suggestion(source, {"path": "x.md", "original": "run npm run start here", "suggested": "run npm run dev here", "reason": "test"})
            self.assertEqual(result["start_line"], 2)
            self.assertEqual(result["source_digest"], hashlib.sha256(raw).hexdigest())
            self.assertEqual(locate_suggestion(source, {"path": "x.md", "original": "npm run start", "suggested": "x", "reason": "test"})["status"], "original_missing")
            multi = b"before\r\nold one\r\nold two\r\nafter\r\n"
            (root / "multi.md").write_bytes(multi)
            multi_source = safe_read(root, "multi.md")
            multi_result = locate_suggestion(multi_source, {"path": "multi.md", "original": "old one\nold two", "suggested": "new", "reason": "test"})
            self.assertEqual(multi_result["start_line"], 2)
            self.assertEqual(multi_result["end_line"], 3)
            (root / "empty.md").write_bytes(b"")
            empty = safe_read(root, "empty.md")
            self.assertEqual(locate_suggestion(empty, {"path": "empty.md", "original": "x", "suggested": "y", "reason": "test"})["status"], "original_missing")

    def test_duplicate_and_no_change_are_explicit(self):
        from rein_core import safe_read, locate_suggestion, verify_source

        root = PART1 / "fixtures"
        duplicate = safe_read(root, "duplicate/README.md")
        ambiguous = locate_suggestion(duplicate, {"path": "duplicate/README.md", "original": "npm run start", "suggested": "npm run dev", "reason": "test"})
        self.assertEqual(ambiguous["status"], "ambiguous_match")
        self.assertEqual(ambiguous["candidates"], [3, 9])
        correct = safe_read(root, "correct/README.md")
        no_change = locate_suggestion(correct, {"path": "correct/README.md", "original": "npm run dev", "suggested": "npm run dev", "reason": "test"})
        self.assertEqual(no_change["status"], "no_change")

    def test_model_line_number_is_rejected_and_missing_original_is_honest(self):
        from rein_core import safe_read, locate_suggestion, verify_source

        source = safe_read(PART1 / "fixtures" / "outdated", "README.md")
        self.assertEqual(locate_suggestion(source, {"path": "README.md", "original": "npm run start", "suggested": "npm run dev", "reason": "x", "start_line": 99})["status"], "response_invalid")
        self.assertEqual(locate_suggestion(source, {"path": "other.md", "original": "npm run start", "suggested": "npm run dev", "reason": "x"})["status"], "path_invalid")
        self.assertEqual(locate_suggestion(source, {"path": "README.md", "original": "missing", "suggested": "x", "reason": "x"})["status"], "original_missing")

    def test_changed_source_is_stale(self):
        from rein_core import safe_read, locate_suggestion, verify_source

        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            target = root / "README.md"
            target.write_text("Run npm run start.\n", encoding="utf-8")
            source = safe_read(root, "README.md")
            target.write_text("Run npm run dev.\n", encoding="utf-8")
            with self.assertRaises(ReinError) as caught:
                verify_source(root, source)
            self.assertEqual(caught.exception.code, "source_changed")

    def test_message_pairing_direct_tool_and_replay_failures(self):
        from rein_core import ModelResponse, OfflineAdapter, ReplayAdapter, Task, load_config, run

        task = Task("check README", PART1 / "fixtures" / "outdated", ("README.md",), "suggest")
        adapter = ReplayAdapter([{"tool_call": {"id": "call_c01", "name": "read_file", "arguments": {"path": "README.md"}}}, {"suggestion": {"path": "README.md", "original": "npm run start", "suggested": "npm run dev", "reason": "current command"}}])
        record = __import__("asyncio").run(run(task, "suggest", adapter, read_mode="tool", timeout=1))
        self.assertEqual(record.status, "unique")
        tool_message = next(message for message in adapter.messages if message.get("role") == "tool")
        assistant_message = next(message for message in adapter.messages if message.get("role") == "assistant")
        self.assertEqual(tool_message["tool_call_id"], "call_c01")
        self.assertEqual(assistant_message["tool_calls"][0]["type"], "function")
        self.assertIn("npm run start", tool_message["content"])

        direct = ReplayAdapter([{"suggestion": {"path": "README.md", "original": "npm run start", "suggested": "npm run dev", "reason": "current command"}}])
        direct_record = __import__("asyncio").run(run(task, "suggest", direct, read_mode="direct", timeout=1))
        self.assertEqual(direct_record.source_digest, record.source_digest)

        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            target = root / "README.md"
            target.write_text("Start with npm run start.\n", encoding="utf-8")
            changing_task = Task("check README", root, ("README.md",), "suggest")
            first = ReplayAdapter([{"tool_call": {"id": "call", "name": "read_file", "arguments": {"path": "README.md"}}}, {"suggestion": {"path": "README.md", "original": "Start with npm run start.", "suggested": "Start with npm run dev.", "reason": "x"}}])
            __import__("asyncio").run(run(changing_task, "suggest", first, read_mode="tool", timeout=1))
            first_tool_text = next(message for message in first.messages if message.get("role") == "tool")["content"]
            target.write_text("Start with npm run dev.\n", encoding="utf-8")
            second = ReplayAdapter([{"tool_call": {"id": "call", "name": "read_file", "arguments": {"path": "README.md"}}}, {"suggestion": {"path": "README.md", "original": "Start with npm run start.", "suggested": "Start with npm run dev.", "reason": "x"}}])
            __import__("asyncio").run(run(changing_task, "suggest", second, read_mode="tool", timeout=1))
            self.assertNotEqual(first_tool_text, next(message for message in second.messages if message.get("role") == "tool")["content"])

            target.write_text("Start with npm run start.\n", encoding="utf-8")
            offline = OfflineAdapter()
            first_result = asyncio.run(run(changing_task, "suggest", offline, read_mode="tool", timeout=1))
            self.assertEqual(first_result.status, "unique")
            target.write_text("Start with npm run dev.\n", encoding="utf-8")
            second_result = asyncio.run(run(changing_task, "suggest", offline, read_mode="tool", timeout=1))
            self.assertEqual(second_result.status, "no_change")

        with self.assertRaises(ReinError) as caught:
            load_config({})
        self.assertEqual(caught.exception.code, "config_missing")

        for event, code in (({"tool_call": {"id": "x", "name": "read_file", "arguments": {}}}, "invalid_tool_call"), ({"tool_call": {"id": "x", "name": "other", "arguments": {"path": "README.md"}}}, "unknown_tool"), ({"error": "non_json"}, "response_invalid"), ({"choices": []}, "response_invalid"), ({"text": ""}, "empty_final"), ({"delay": 2}, "timeout")):
            failing = ReplayAdapter([event])
            result = __import__("asyncio").run(run(task, "suggest", failing, read_mode="tool", timeout=0.01))
            self.assertEqual(result.error, code)

        denied = ReplayAdapter([{"tool_call": {"id": "x", "name": "read_file", "arguments": {"path": "notes.md"}}}])
        denied_task = Task("check README", PART1 / "fixtures" / "outdated", ("README.md",), "suggest")
        denied_record = __import__("asyncio").run(run(denied_task, "suggest", denied, read_mode="tool", timeout=1))
        self.assertEqual(denied_record.error, "path_invalid")

        missing = ReplayAdapter([{"tool_call": {"id": "missing-1", "name": "read_file", "arguments": {"path": "missing.md"}}}])
        missing_record = __import__("asyncio").run(run(task, "suggest", missing, read_mode="tool", timeout=1))
        self.assertEqual(missing_record.error, "path_invalid")
        failed_tool = next(message for message in missing_record.messages if message.get("role") == "tool")
        self.assertEqual((failed_tool["tool_call_id"], failed_tool["ok"], failed_tool["output"], failed_tool["error"]), ("missing-1", False, None, "path_invalid"))

        second_tool = ReplayAdapter([{"tool_call": {"id": "first", "name": "read_file", "arguments": {"path": "README.md"}}}, {"tool_call": {"id": "second", "name": "read_file", "arguments": {"path": "README.md"}}}])
        self.assertEqual(__import__("asyncio").run(run(task, "suggest", second_tool, read_mode="tool", timeout=1)).error, "response_invalid")
        extra = ReplayAdapter([{"tool_call": {"id": "extra", "name": "read_file", "arguments": {"path": "README.md", "extra": 1}}}])
        self.assertEqual(__import__("asyncio").run(run(task, "suggest", extra, read_mode="tool", timeout=1)).error, "invalid_tool_call")

    def test_run_rejects_source_changed_before_final_display(self):
        from rein_core import ModelAdapter, ModelResponse, Task, run

        class MutatingAdapter(ModelAdapter):
            def __init__(self, target):
                self.target = target
                self.calls = 0

            async def request(self, task, messages, deadline):
                self.calls += 1
                if self.calls == 1:
                    return ModelResponse('{"tool_call":{"id":"c","name":"read_file","arguments":{"path":"README.md"}}}', "raw", "test", {"tool_call": {"id": "c", "name": "read_file", "arguments": {"path": "README.md"}}})
                self.target.write_text("npm run dev\n", encoding="utf-8")
                return ModelResponse('{"suggestion":{"path":"README.md","original":"npm run start","suggested":"npm run dev","reason":"x"}}', "raw", "test", {"suggestion": {"path": "README.md", "original": "npm run start", "suggested": "npm run dev", "reason": "x"}})

        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            target = root / "README.md"
            target.write_text("npm run start\n", encoding="utf-8")
            record = asyncio.run(run(Task("suggest", root, ("README.md",), "suggest"), "suggest", MutatingAdapter(target), timeout=1))
            self.assertEqual(record.error, "source_changed")

    def test_cli_is_json_and_read_only(self):
        target = PART1 / "fixtures" / "outdated" / "README.md"
        before = target.read_bytes()
        proc = subprocess.run([sys.executable, str(PART1 / "rein.py"), "suggest", "--workspace", str(target.parent), "--path", "README.md"], cwd=ROOT, text=True, capture_output=True)
        self.assertEqual(proc.returncode, 0, proc.stderr)
        payload = json.loads(proc.stdout)
        self.assertIn("result", payload)
        self.assertEqual(payload["result"]["start_line"], 8)
        required_fields = {"path", "start_line", "end_line", "original", "suggested", "reason", "source_digest"}
        self.assertTrue(required_fields <= set(payload["result"]))
        self.assertEqual(payload["result"]["status"], "unique")
        self.assertEqual(target.read_bytes(), before)

    def test_direct_read_returns_read_result_and_does_not_locate(self):
        from rein_core import OfflineAdapter, Task, run

        task = Task("check the preview command", PART1 / "fixtures" / "outdated", ("README.md",), "suggest")
        record = asyncio.run(run(task, "read", OfflineAdapter(), read_mode="direct", timeout=1))
        self.assertEqual(record.status, "read")
        self.assertIsNone(record.error)

    def test_bad_task_timeout_and_external_path_are_bounded(self):
        from rein_core import OfflineAdapter, Task, run

        base = PART1 / "fixtures" / "outdated"
        for timeout in (0, float("nan"), float("inf")):
            record = asyncio.run(run(Task("x", base, ("README.md",), "suggest"), "suggest", OfflineAdapter(), timeout=timeout))
            self.assertEqual(record.error, "invalid_task")
        record = asyncio.run(run(Task("x", base, ("notes.md",), "suggest"), "suggest", OfflineAdapter(), timeout=1))
        self.assertEqual(record.error, "path_invalid")

    def test_ambiguous_cli_is_failure(self):
        proc = subprocess.run([sys.executable, str(PART1 / "rein.py"), "suggest", "--workspace", str(PART1 / "fixtures" / "duplicate"), "--path", "README.md"], cwd=ROOT, text=True, capture_output=True)
        self.assertEqual(proc.returncode, 1)
        self.assertEqual(json.loads(proc.stdout)["record"]["error"], "ambiguous_match")


if __name__ == "__main__":
    unittest.main()
