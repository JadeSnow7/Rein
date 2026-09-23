"""B01-B05 behavior contract for the three Hello World chapters.

The tests deliberately describe the public, small tutorial API.  They do not
call a real model service: model.py must expose a local scripted adapter for
these checks, while the live adapter is exercised only with an injected fake
client.
"""

from __future__ import annotations

import json
import shutil
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[3]
HELLO = ROOT / "python" / "hello_world"
sys.path.insert(0, str(ROOT / "python"))


def load_modules():
    """Load the planned public modules while keeping baseline failures readable."""

    try:
        from hello_world import core, model
    except ModuleNotFoundError as exc:  # expected before implementation starts
        raise AssertionError(f"hello_world implementation not started: {exc}") from exc
    return core, model


def make_workspace(source: str, compiler_log: str | None = "") -> tempfile.TemporaryDirectory:
    directory = tempfile.TemporaryDirectory()
    root = Path(directory.name)
    (root / "hello.cpp").write_text(source, encoding="utf-8")
    if compiler_log is not None:
        (root / "compiler.log").write_text(compiler_log, encoding="utf-8")
    return directory


BROKEN_CPP = "#include <iostream>\nint main() { std::cout << \"Hello, world!\\n\" }\n"
FIXED_CPP = "#include <iostream>\nint main() { std::cout << \"Hello, world!\\n\"; }\n"


class HelloWorldBehaviorTests(unittest.TestCase):
    def test_b01_offline_hello_is_non_empty_and_live_config_is_checked_before_client(self):
        core, model = load_modules()

        offline = core.hello("请你好并做自我介绍", mode="offline")
        self.assertTrue(offline.text.strip())

        class ExplodingClient:
            def __init__(self):
                raise AssertionError("network/client must not be created for missing config")

        with self.assertRaises(core.HelloError) as caught:
            core.hello(
                "你好",
                mode="live",
                environ={},
                client_factory=ExplodingClient,
            )
        self.assertEqual(caught.exception.code, "config_missing")
        self.assertNotIn("secret", str(caught.exception))

    def test_b01_injected_live_client_handles_empty_timeout_and_http_error_as_categories(self):
        core, model = load_modules()

        class FakeClient:
            def __init__(self, result):
                self.result = result
                self.calls = []

            def complete(self, **kwargs):
                self.calls.append(kwargs)
                if isinstance(self.result, Exception):
                    raise self.result
                return self.result

        env = {"REIN_BASE_URL": "https://mock.local/v1", "REIN_API_KEY": "secret", "REIN_MODEL": "mock"}
        response = {"choices": [{"message": {"content": "你好，我是测试模型。"}}]}
        client = FakeClient(response)
        result = core.hello("你好", mode="live", environ=env, client=client)
        self.assertEqual(result.text, "你好，我是测试模型。")
        self.assertEqual(len(client.calls), 1)

        for error, code in ((TimeoutError(), "timeout"), (model.HTTPError(401, "unauthorized"), "auth_error")):
            with self.subTest(code=code), self.assertRaises(core.HelloError) as caught:
                core.hello("你好", mode="live", environ=env, client=FakeClient(error))
            self.assertEqual(caught.exception.code, code)
            self.assertNotIn("secret", str(caught.exception))

        for response in ({"choices": []}, {"choices": [{"message": {"content": ""}}]}, {"bad": "shape"}):
            with self.subTest(response=response), self.assertRaises(core.HelloError) as caught:
                core.hello("你好", mode="live", environ=env, client=FakeClient(response))
            self.assertEqual(caught.exception.code, "response_invalid")

    def test_b02_direct_and_tool_diagnosis_include_current_source_log_and_environment(self):
        core, model = load_modules()
        directory = make_workspace(BROKEN_CPP, "error: expected ';' before '}' token\n")
        self.addCleanup(directory.cleanup)
        root = Path(directory.name)
        scripted = model.ScriptedModel([{"candidate": {"code": FIXED_CPP, "reason": "补上缺失的分号"}}])

        direct = core.diagnose(root, read_mode="direct", model=scripted)
        self.assertEqual(direct.candidate_code, FIXED_CPP)
        direct_payload = json.loads(direct.messages[-1]["content"])
        self.assertEqual(direct_payload["source"], BROKEN_CPP)
        self.assertEqual(direct_payload["compiler_log"], "error: expected ';' before '}' token\n")
        self.assertIn("python", direct_payload["environment"])

        scripted = model.ScriptedModel([
            {"tool_call": {"id": "read_source_1", "name": "read_file", "arguments": {"path": "hello.cpp"}}},
            {"tool_call": {"id": "read_log_1", "name": "read_file", "arguments": {"path": "compiler.log"}}},
            {"tool_call": {"id": "read_env_1", "name": "read_environment", "arguments": {}}},
            {"candidate": {"code": FIXED_CPP, "reason": "补上缺失的分号"}},
        ])
        tool = core.diagnose(root, read_mode="tool", model=scripted)
        self.assertEqual(tool.candidate_code, FIXED_CPP)
        tool_messages = [message for message in scripted.messages if message.get("role") == "tool"]
        self.assertEqual([message["tool_call_id"] for message in tool_messages], ["read_source_1", "read_log_1", "read_env_1"])
        self.assertEqual(json.loads(tool_messages[0]["content"])["text"], BROKEN_CPP)
        self.assertEqual(json.loads(tool_messages[1]["content"])["text"], "error: expected ';' before '}' token\n")
        self.assertIn("python", json.loads(tool_messages[2]["content"]))

    def test_b02_diagnosis_digest_changes_when_source_changes_and_missing_log_is_honest(self):
        core, model = load_modules()
        directory = make_workspace(BROKEN_CPP, None)
        self.addCleanup(directory.cleanup)
        root = Path(directory.name)
        with self.assertRaises(core.HelloError) as caught:
            core.diagnose(root, read_mode="direct", model=model.ScriptedModel([]))
        self.assertEqual(caught.exception.code, "missing_log")

        (root / "compiler.log").write_text("", encoding="utf-8")
        first = core.diagnose(root, read_mode="direct", model=model.ScriptedModel([{"candidate": {"code": FIXED_CPP, "reason": "fix"}}]))
        (root / "hello.cpp").write_text("changed\n", encoding="utf-8")
        second = core.diagnose(root, read_mode="direct", model=model.ScriptedModel([{"candidate": {"code": FIXED_CPP, "reason": "fix"}}]))
        self.assertNotEqual(first.source_digest, second.source_digest)

    def test_offline_repair_preserves_return_zero_after_adding_semicolon(self):
        core, model = load_modules()
        source = '#include <iostream>\nint main() {\n  std::cout << "Hello, world!\\n"\n  return 0;\n}\n'
        directory = make_workspace(source, "error: expected ';'\n")
        self.addCleanup(directory.cleanup)
        result = core.diagnose(Path(directory.name), read_mode="direct", model=model.OfflineModel())
        self.assertEqual(result.candidate_code, '#include <iostream>\nint main() {\n  std::cout << "Hello, world!\\n";\n  return 0;\n}\n')

    def test_b03_safe_context_rejects_traversal_symlink_and_oversized_input(self):
        core, _ = load_modules()
        directory = make_workspace(FIXED_CPP, "ok\n")
        self.addCleanup(directory.cleanup)
        root = Path(directory.name)
        with self.assertRaises(core.HelloError) as caught:
            core.safe_read(root, "../outside")
        self.assertEqual(caught.exception.code, "path_invalid")
        (root / "large.txt").write_bytes(b"x" * (core.MAX_READ_BYTES + 1))
        with self.assertRaises(core.HelloError) as caught:
            core.safe_read(root, "large.txt")
        self.assertEqual(caught.exception.code, "path_invalid")
        (root / "hello.cpp").write_bytes(b"x" * (core.MAX_READ_BYTES + 1))
        with self.assertRaises(core.HelloError) as caught:
            core.safe_read(root, "hello.cpp")
        self.assertEqual(caught.exception.code, "file_too_large")
        (root / "hello.cpp").write_text(FIXED_CPP, encoding="utf-8")
        try:
            (root / "hello.cpp").unlink()
            (root / "hello.cpp").symlink_to(root / "outside.cpp")
        except (OSError, NotImplementedError):
            self.skipTest("symlinks unavailable")
        with self.assertRaises(core.HelloError) as caught:
            core.safe_read(root, "hello.cpp")
        self.assertEqual(caught.exception.code, "path_invalid")

    def test_b03_unknown_tool_bad_candidate_and_request_budget_stop_cleanly(self):
        core, model = load_modules()
        directory = make_workspace(BROKEN_CPP, "error\n")
        self.addCleanup(directory.cleanup)
        root = Path(directory.name)
        with self.assertRaises(core.HelloError) as caught:
            core.dispatch_tool(root, {"name": "read_everything", "arguments": {}})
        self.assertEqual(caught.exception.code, "tool_unknown")
        with self.assertRaises(core.HelloError) as caught:
            model.validate_candidate({"reason": "missing code"})
        self.assertEqual(caught.exception.code, "response_invalid")
        limited = model.ScriptedModel([
            {"tool_call": {"id": f"call_{index}", "name": "read_file", "arguments": {"path": "hello.cpp"}}}
            for index in range(7)
        ])
        with self.assertRaises(core.HelloError) as caught:
            core.diagnose(root, read_mode="tool", model=limited, max_requests=6)
        self.assertEqual(caught.exception.code, "request_budget")

    def test_b03_unknown_environment_shape_and_eof_choice_default_to_safe_failure(self):
        core, _ = load_modules()
        with self.assertRaises(core.HelloError) as caught:
            core.dispatch_tool(Path(tempfile.gettempdir()), {"name": "read_environment", "arguments": {"secret": True}})
        self.assertEqual(caught.exception.code, "tool_invalid")
        self.assertFalse(core.accept_choice(""))
        self.assertFalse(core.accept_choice("unexpected"))
        self.assertTrue(core.accept_choice("y"))

    def test_b03_bash_allows_fixed_read_only_inspection_and_rejects_shell_text(self):
        core, _ = load_modules()
        directory = make_workspace(BROKEN_CPP, "error: expected ';'\n")
        self.addCleanup(directory.cleanup)
        root = Path(directory.name)
        before = (root / "hello.cpp").read_bytes(), (root / "compiler.log").read_bytes()
        self.assertEqual(core.run_bash(root, "ls -1")["returncode"], 0)
        self.assertIn("hello.cpp", core.run_bash(root, "ls -1")["stdout"])
        self.assertIn("expected", core.run_bash(root, "cat compiler.log")["stdout"])
        self.assertIn(str(root), core.run_bash(root, "pwd")["stdout"])
        for command in ("echo nope", "cat hello.cpp; touch hello.cpp", "pwd > compiler.log", "rm hello.cpp", "ls .."):
            with self.subTest(command=command), self.assertRaises(core.HelloError) as caught:
                core.run_bash(root, command)
            self.assertEqual(caught.exception.code, "bash_command_invalid")
        self.assertEqual((root / "hello.cpp").read_bytes(), before[0])
        self.assertEqual((root / "compiler.log").read_bytes(), before[1])

    def test_b03_offline_tool_example_inspects_workspace_before_source_and_log(self):
        core, model = load_modules()
        directory = make_workspace(BROKEN_CPP, "error: expected ';'\n")
        self.addCleanup(directory.cleanup)
        result = core.diagnose(Path(directory.name), read_mode="tool", model=model.OfflineModel())
        self.assertEqual(result.candidate_code, FIXED_CPP)
        self.assertEqual(result.tools_used, ("bash", "read_file", "read_file", "read_environment"))
        tool_messages = [message for message in result.messages if message.get("role") == "tool"]
        self.assertEqual(json.loads(tool_messages[0]["content"])["command"], "ls -1")
        self.assertEqual(json.loads(tool_messages[1]["content"])["text"], BROKEN_CPP)

    def test_b04_render_review_contains_full_files_diff_and_is_readable_without_color(self):
        core, _ = load_modules()
        rendered = core.render_review(BROKEN_CPP, FIXED_CPP, color="never")
        self.assertIn("=== original file ===", rendered)
        self.assertIn("=== candidate file ===", rendered)
        self.assertIn('std::cout << "Hello, world!\\n" }', rendered)
        self.assertIn('std::cout << "Hello, world!\\n"; }', rendered)
        self.assertIn("-", rendered)
        self.assertIn("+", rendered)
        self.assertIn("1", rendered)
        self.assertEqual(core.render_review(BROKEN_CPP, FIXED_CPP, color="never"), core.render_review(BROKEN_CPP, FIXED_CPP, color="always").replace("\x1b[31m", "").replace("\x1b[32m", "").replace("\x1b[0m", ""))

    def test_b04_reject_is_side_effect_free_accept_creates_backup_and_source_change_is_rejected(self):
        core, _ = load_modules()
        directory = make_workspace(BROKEN_CPP, "error\n")
        self.addCleanup(directory.cleanup)
        root = Path(directory.name)
        original_bytes = (root / "hello.cpp").read_bytes()
        source = core.safe_read(root, "hello.cpp")
        rejected = core.apply_candidate(root, FIXED_CPP, expected_digest=source.digest, accept=False)
        self.assertEqual(rejected.status, "rejected")
        self.assertEqual((root / "hello.cpp").read_bytes(), original_bytes)
        self.assertFalse(list(root.glob("hello.cpp.bak-*")))

        accepted = core.apply_candidate(root, FIXED_CPP, expected_digest=source.digest, accept=True)
        self.assertEqual(accepted.status, "accepted")
        self.assertEqual((root / "hello.cpp").read_text(encoding="utf-8"), FIXED_CPP)
        self.assertEqual(len(list(root.glob("hello.cpp.bak-*"))), 1)

        stale_source = core.safe_read(root, "hello.cpp")
        (root / "hello.cpp").write_text(BROKEN_CPP, encoding="utf-8")
        with self.assertRaises(core.HelloError) as caught:
            core.apply_candidate(root, FIXED_CPP, expected_digest=stale_source.digest, accept=True)
        self.assertEqual(caught.exception.code, "source_changed")

    def test_b05_check_compiles_and_runs_fixed_program_with_stable_command(self):
        if shutil.which("c++") is None:
            self.skipTest("c++ compiler unavailable")
        core, _ = load_modules()
        directory = make_workspace(FIXED_CPP, "")
        self.addCleanup(directory.cleanup)
        result = core.check_cpp(Path(directory.name))
        self.assertEqual(result.compile_returncode, 0)
        self.assertEqual(result.run_returncode, 0)
        self.assertEqual(result.stdout, "Hello, world!\n")
        self.assertTrue(result.passed)
        self.assertEqual(result.command[:3], ["c++", "-std=c++17", "hello.cpp"])
        self.assertTrue((Path(directory.name) / "compiler.log").read_text(encoding="utf-8"))

    def test_b05_compile_failure_does_not_run_and_wrong_output_is_not_pass(self):
        if shutil.which("c++") is None:
            self.skipTest("c++ compiler unavailable")
        core, _ = load_modules()
        broken = make_workspace(BROKEN_CPP, "")
        self.addCleanup(broken.cleanup)
        failed = core.check_cpp(Path(broken.name))
        self.assertNotEqual(failed.compile_returncode, 0)
        self.assertIsNone(failed.run_returncode)
        self.assertFalse(failed.passed)

        wrong = make_workspace(FIXED_CPP.replace("Hello, world!", "hello"), "")
        self.addCleanup(wrong.cleanup)
        wrong_result = core.check_cpp(Path(wrong.name))
        self.assertEqual(wrong_result.compile_returncode, 0)
        self.assertEqual(wrong_result.run_returncode, 0)
        self.assertFalse(wrong_result.passed)

    def test_b05_run_timeout_is_reported_and_not_passed(self):
        if shutil.which("c++") is None:
            self.skipTest("c++ compiler unavailable")
        core, _ = load_modules()
        directory = make_workspace("int main() { for (;;) {} }\n", "")
        self.addCleanup(directory.cleanup)
        result = core.check_cpp(Path(directory.name), run_timeout=0.1)
        self.assertEqual(result.compile_returncode, 0)
        self.assertTrue(result.run_timed_out)
        self.assertFalse(result.passed)


if __name__ == "__main__":
    unittest.main()
