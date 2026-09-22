"""Cross-process CLI and real OpenAI SDK integration contracts for chapters 01-03."""

from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

try:
    import httpx
    from openai import OpenAI
except ModuleNotFoundError:  # The CLI and offline tests remain stdlib-only.
    httpx = None
    OpenAI = None


ROOT = Path(__file__).resolve().parents[3]
CLI = ROOT / "python" / "hello_world" / "cli.py"
PYTHON = sys.executable
sys.path.insert(0, str(ROOT / "python"))
BROKEN = '#include <iostream>\n\nint main() {\n    std::cout << "Hello, world!\\n"\n}\n'
FIXED = '#include <iostream>\n\nint main() {\n    std::cout << "Hello, world!\\n";\n}\n'
BROKEN_RETURN = '#include <iostream>\n\nint main() {\n    std::cout << "Hello, world!\\n"\n    return 0;\n}\n'
FIXED_RETURN = '#include <iostream>\n\nint main() {\n    std::cout << "Hello, world!\\n";\n    return 0;\n}\n'


def make_workspace(source: str = BROKEN, log: str = "error: expected ';'\n") -> tempfile.TemporaryDirectory:
    directory = tempfile.TemporaryDirectory()
    root = Path(directory.name)
    (root / "hello.cpp").write_text(source, encoding="utf-8")
    (root / "compiler.log").write_text(log, encoding="utf-8")
    return directory


def run_cli(*args: str, input_text: str | None = None, cwd: Path = ROOT) -> subprocess.CompletedProcess[str]:
    env = os.environ.copy()
    env.pop("REIN_BASE_URL", None)
    env.pop("REIN_API_KEY", None)
    env.pop("REIN_MODEL", None)
    kwargs = {"input": input_text} if input_text is not None else {"stdin": subprocess.DEVNULL}
    return subprocess.run(
        [PYTHON, str(CLI), *args], cwd=cwd, text=True,
        capture_output=True, env=env, check=False, **kwargs,
    )


class HelloCliAndSdkTests(unittest.TestCase):
    def test_cli_hello_and_generate_keep_source_stdout_clean_and_mark_offline(self):
        hello = run_cli("hello")
        self.assertEqual(hello.returncode, 0, hello.stderr)
        self.assertTrue(hello.stdout.strip())
        self.assertIn("offline", hello.stderr.lower())

        generated = run_cli("generate")
        self.assertEqual(generated.returncode, 0, generated.stderr)
        self.assertTrue(generated.stdout.startswith("#include"), generated.stdout)
        self.assertNotIn("offline", generated.stdout.lower())
        self.assertIn("offline", generated.stderr.lower())

    def test_cli_diagnose_direct_and_tool_are_real_cross_process_paths(self):
        directory = make_workspace()
        self.addCleanup(directory.cleanup)
        workspace = Path(directory.name)
        for read_mode in ("direct", "tool"):
            result = run_cli("diagnose", "--workspace", str(workspace), "--read-mode", read_mode)
            with self.subTest(read_mode=read_mode):
                self.assertEqual(result.returncode, 0, result.stderr)
                payload = json.loads(result.stdout)
                self.assertEqual(payload["code"], FIXED)
                self.assertTrue(payload["reason"])

        return_directory = make_workspace(BROKEN_RETURN)
        self.addCleanup(return_directory.cleanup)
        return_workspace = Path(return_directory.name)
        for read_mode in ("direct", "tool"):
            result = run_cli("diagnose", "--workspace", str(return_workspace), "--read-mode", read_mode)
            with self.subTest(read_mode=read_mode, source="return-0"):
                self.assertEqual(result.returncode, 0, result.stderr)
                payload = json.loads(result.stdout)
                self.assertEqual(payload["code"], FIXED_RETURN)

    def test_cli_edit_reject_and_eof_do_not_write_or_backup_and_never_color_is_readable(self):
        for input_text in ("n\n", None):
            directory = make_workspace()
            self.addCleanup(directory.cleanup)
            workspace = Path(directory.name)
            before = (workspace / "hello.cpp").read_bytes()
            result = run_cli("edit", "--workspace", str(workspace), "--color", "never", input_text=input_text)
            with self.subTest(input_text=input_text):
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertNotIn("\x1b[", result.stdout)
                self.assertIn("=== diff ===", result.stdout)
                self.assertIn("+", result.stdout)
                self.assertIn("rejected", result.stdout)
                self.assertEqual((workspace / "hello.cpp").read_bytes(), before)
                self.assertEqual(list(workspace.glob("hello.cpp.bak-*")), [])

    def test_cli_accept_creates_backup_then_fixed_program_compiles_and_runs(self):
        directory = make_workspace()
        self.addCleanup(directory.cleanup)
        workspace = Path(directory.name)
        edited = run_cli("edit", "--workspace", str(workspace), "--color", "always", input_text="y\n")
        self.assertEqual(edited.returncode, 0, edited.stderr)
        self.assertIn("\x1b[", edited.stdout)
        self.assertIn("accepted", edited.stdout)
        self.assertIn('"passed": true', edited.stdout)
        self.assertIn('"compile_returncode": 0', edited.stdout)
        self.assertIn('"run_returncode": 0', edited.stdout)
        backups = list(workspace.glob("hello.cpp.bak-*"))
        self.assertEqual(len(backups), 1)
        self.assertEqual(backups[0].read_text(encoding="utf-8"), BROKEN)

        checked = run_cli("check", "--workspace", str(workspace))
        self.assertEqual(checked.returncode, 0, checked.stderr)
        report = json.loads(checked.stdout)
        self.assertTrue(report["passed"], report)
        self.assertEqual(report["stdout"], "Hello, world!\n")

    def test_tool_diagnosis_rejects_candidate_before_required_evidence(self):
        from hello_world import core, model

        directory = make_workspace()
        self.addCleanup(directory.cleanup)
        with self.assertRaises(core.HelloError) as caught:
            core.diagnose(
                Path(directory.name), read_mode="tool",
                model=model.ScriptedModel([{"candidate": {"code": FIXED, "reason": "未经读取现场"}}]),
            )
        self.assertIn(caught.exception.code, {"response_invalid", "context_incomplete"})

    def test_tool_diagnosis_requires_compiler_log_before_candidate(self):
        from hello_world import core, model

        directory = make_workspace()
        self.addCleanup(directory.cleanup)
        root = Path(directory.name)
        with self.assertRaises(core.HelloError) as caught:
            core.diagnose(
                root,
                read_mode="tool",
                model=model.ScriptedModel([
                    {"tool_calls": [
                        {"id": "source_1", "name": "read_file", "arguments": {"path": "hello.cpp"}},
                        {"id": "environment_1", "name": "read_environment", "arguments": {}},
                    ]},
                    {"candidate": {"code": FIXED, "reason": "没有读取编译日志"}},
                ]),
            )
        self.assertIn(caught.exception.code, {"response_invalid", "context_incomplete"})

    def test_tool_diagnosis_rejects_duplicate_or_non_string_tool_ids(self):
        from hello_world import core, model

        directory = make_workspace()
        self.addCleanup(directory.cleanup)
        root = Path(directory.name)
        cases = [
            [
                {"tool_calls": [
                    {"id": "same", "name": "read_file", "arguments": {"path": "hello.cpp"}},
                    {"id": "same", "name": "read_file", "arguments": {"path": "compiler.log"}},
                ]},
            ],
            [
                {"tool_call": {"id": 1, "name": "read_file", "arguments": {"path": "hello.cpp"}}},
            ],
            [
                {"tool_calls": [{"id": [], "name": "read_file", "arguments": {"path": "hello.cpp"}}]},
            ],
        ]
        for events in cases:
            with self.subTest(events=events), self.assertRaises(core.HelloError) as caught:
                core.diagnose(root, read_mode="tool", model=model.ScriptedModel(events))
            self.assertIn(caught.exception.code, {"tool_invalid", "response_invalid"})

    def test_check_rejects_dangling_compiler_log_symlink_without_writing_outside_workspace(self):
        from hello_world import core

        workspace_directory = make_workspace(FIXED, "")
        outside_directory = tempfile.TemporaryDirectory()
        self.addCleanup(workspace_directory.cleanup)
        self.addCleanup(outside_directory.cleanup)
        root = Path(workspace_directory.name)
        outside_log = Path(outside_directory.name) / "compiler.log"
        log_path = root / "compiler.log"
        try:
            log_path.unlink()
            log_path.symlink_to(outside_log)
        except (OSError, NotImplementedError):
            self.skipTest("symlinks unavailable")
        with self.assertRaises(core.HelloError) as caught:
            core.check_cpp(root)
        self.assertEqual(caught.exception.code, "path_invalid")
        self.assertFalse(outside_log.exists())

    def test_real_openai_sdk_mock_transport_performs_three_tool_reads_then_candidate(self):
        if httpx is None or OpenAI is None:
            self.skipTest("openai 2.26.0 and httpx are required for SDK integration")
        from hello_world import core, model

        directory = make_workspace()
        self.addCleanup(directory.cleanup)
        requests: list[dict] = []
        responses = [
            {"choices": [{"index": 0, "message": {"role": "assistant", "content": None, "tool_calls": [
                {"id": "src_1", "type": "function", "function": {"name": "read_file", "arguments": '{"path":"hello.cpp"}'}},
                {"id": "log_1", "type": "function", "function": {"name": "read_file", "arguments": '{"path":"compiler.log"}'}},
                {"id": "env_1", "type": "function", "function": {"name": "read_environment", "arguments": "{}"}},
            ]}}], "id": "mock-1", "object": "chat.completion", "created": 0, "model": "mock"},
            {"choices": [{"index": 0, "message": {"role": "assistant", "content": json.dumps({"code": FIXED, "reason": "补上分号"}, ensure_ascii=False)}}], "id": "mock-2", "object": "chat.completion", "created": 0, "model": "mock"},
        ]

        def handler(request: httpx.Request) -> httpx.Response:
            requests.append(json.loads(request.content))
            return httpx.Response(200, json=responses[len(requests) - 1], request=request)

        transport = httpx.MockTransport(handler)
        with httpx.Client(transport=transport) as http_client:
            sdk = OpenAI(api_key="secret", base_url="https://mock.local/v1", http_client=http_client, max_retries=0)
            adapter = model.LiveModel(client=sdk.chat.completions, model="mock")
            diagnosis = core.diagnose(Path(directory.name), read_mode="tool", model=adapter)

        self.assertEqual(diagnosis.candidate_code, FIXED)
        self.assertEqual(len(requests), 2)
        first_tools = {item["function"]["name"] for item in requests[0]["tools"]}
        self.assertEqual(first_tools, {"read_file", "read_environment"})
        second_messages = requests[1]["messages"]
        self.assertEqual(
            [item["tool_call_id"] for item in second_messages if item.get("role") == "tool"],
            ["src_1", "log_1", "env_1"],
        )
        tool_contents = [item["content"] for item in second_messages if item.get("role") == "tool"]
        self.assertEqual(json.loads(tool_contents[0])["text"], BROKEN)
        self.assertIn("expected", json.loads(tool_contents[1])["text"])
        self.assertIn("python", json.loads(tool_contents[2]) )

    def test_real_sdk_http_and_empty_response_failures_are_classified_without_secret(self):
        if httpx is None or OpenAI is None:
            self.skipTest("openai 2.26.0 and httpx are required for SDK integration")
        from hello_world import core, model

        errors = [
            (401, {"error": {"message": "bad secret", "type": "invalid_api_key"}}, "auth_error"),
            (404, {"error": {"message": "missing endpoint", "type": "not_found"}}, "http_error"),
            (429, {"error": {"message": "rate limit", "type": "rate_limit_exceeded"}}, "rate_limit"),
            (429, {"error": {"message": "quota", "code": "insufficient_quota"}}, "quota_exhausted"),
            (429, {"error": {"message": "unknown throttling failure", "type": "other"}}, "http_error"),
        ]
        for status, body, expected_code in errors:
            def handler(request: httpx.Request, *, status=status, body=body) -> httpx.Response:
                return httpx.Response(status, json=body, request=request)

            with httpx.Client(transport=httpx.MockTransport(handler)) as http_client:
                sdk = OpenAI(api_key="secret", base_url="https://mock.local/v1", http_client=http_client, max_retries=0)
                with self.subTest(status=status, body=body), self.assertRaises(core.HelloError) as caught:
                    core.hello("你好", mode="live", environ={"REIN_BASE_URL": "https://mock.local/v1", "REIN_API_KEY": "secret", "REIN_MODEL": "mock"}, client=sdk.chat.completions)
                self.assertNotIn("secret", str(caught.exception))
                self.assertEqual(caught.exception.code, expected_code)

        def empty_handler(request: httpx.Request) -> httpx.Response:
            return httpx.Response(200, json={"choices": []}, request=request)

        with httpx.Client(transport=httpx.MockTransport(empty_handler)) as http_client:
            sdk = OpenAI(api_key="secret", base_url="https://mock.local/v1", http_client=http_client, max_retries=0)
            with self.assertRaises(core.HelloError) as caught:
                core.hello("你好", mode="live", environ={"REIN_BASE_URL": "https://mock.local/v1", "REIN_API_KEY": "secret", "REIN_MODEL": "mock"}, client=sdk.chat.completions)
        self.assertEqual(caught.exception.code, "response_invalid")
        self.assertNotIn("secret", str(caught.exception))

        def timeout_handler(request: httpx.Request) -> httpx.Response:
            raise httpx.ReadTimeout("provider timed out", request=request)

        with httpx.Client(transport=httpx.MockTransport(timeout_handler)) as http_client:
            sdk = OpenAI(api_key="secret", base_url="https://mock.local/v1", http_client=http_client, max_retries=0)
            with self.assertRaises(core.HelloError) as caught:
                core.hello("你好", mode="live", environ={"REIN_BASE_URL": "https://mock.local/v1", "REIN_API_KEY": "secret", "REIN_MODEL": "mock"}, client=sdk.chat.completions)
        self.assertEqual(caught.exception.code, "timeout")
        self.assertNotIn("secret", str(caught.exception))


if __name__ == "__main__":
    unittest.main()
