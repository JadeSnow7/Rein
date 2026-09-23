from __future__ import annotations

import contextlib
import io
import os
import subprocess
import sys
import tempfile
import types
import unittest
from pathlib import Path
from unittest import mock

try:
    import httpx
except ImportError:  # pragma: no cover - the fixed lesson environment installs it
    httpx = None
try:
    from openai import OpenAI as SDKOpenAI
except ImportError:  # pragma: no cover - the fixed lesson environment installs it
    SDKOpenAI = None


ROOT = Path(__file__).resolve().parents[3]
HELLO = ROOT / "python" / "hello_world" / "hello.py"
CHAT = ROOT / "python" / "hello_world" / "chat.py"
PYTHON = sys.executable


def run_script(path: Path, *, env: dict[str, str] | None = None, stdin: str = "") -> subprocess.CompletedProcess[str]:
    child_env = os.environ.copy()
    for key in ("REIN_BASE_URL", "REIN_API_KEY", "REIN_MODEL"):
        child_env.pop(key, None)
    if env:
        child_env.update(env)
    return subprocess.run(
        [PYTHON, str(path)],
        input=stdin,
        text=True,
        capture_output=True,
        env=child_env,
        timeout=10,
    )


class ChatChapterTests(unittest.TestCase):
    def test_hello_missing_configuration_gives_local_greeting_without_importing_sdk(self):
        fake_openai = tempfile.TemporaryDirectory()
        self.addCleanup(fake_openai.cleanup)
        Path(fake_openai.name, "openai.py").write_text(
            "raise AssertionError('SDK must not be imported when configuration is absent')\n",
            encoding="utf-8",
        )
        result = run_script(
            HELLO,
            env={"PYTHONPATH": fake_openai.name, "REIN_API_KEY": "  "},
        )
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertIn("你好！程序已启动。", result.stdout)
        self.assertIn("API 尚未正确配置，本次没有发送模型请求。", result.stdout)
        self.assertIn("REIN_BASE_URL、REIN_API_KEY、REIN_MODEL", result.stdout)
        self.assertNotIn("请求失败", result.stdout + result.stderr)

    def test_hello_success_uses_sdk_and_prefixes_model_response(self):
        import importlib.util

        captured: list[dict] = []

        class FakeMessage:
            content = "你好，我可以帮助你编写和调试程序。"

        class FakeResponse:
            choices = [types.SimpleNamespace(message=FakeMessage())]

        class FakeCompletions:
            def create(self, **kwargs):
                captured.append(kwargs)
                return FakeResponse()

        class FakeClient:
            def __init__(self, **kwargs):
                self.chat = types.SimpleNamespace(completions=FakeCompletions())

        spec = importlib.util.spec_from_file_location("lesson_hello", HELLO)
        module = importlib.util.module_from_spec(spec)
        assert spec and spec.loader
        spec.loader.exec_module(module)
        fake_openai = types.SimpleNamespace(OpenAI=FakeClient)
        with mock.patch.dict(sys.modules, {"openai": fake_openai}), mock.patch.dict(
            os.environ,
            {
                "REIN_BASE_URL": "https://mock.local/v1",
                "REIN_API_KEY": "secret",
                "REIN_MODEL": "mock-model",
            },
            clear=False,
        ), contextlib.redirect_stdout(io.StringIO()) as output:
            self.assertEqual(module.main(), 0)
        self.assertIn("模型：你好，我可以帮助你编写和调试程序。", output.getvalue())
        self.assertEqual(captured[0]["stream"], False)

    def test_hello_invalid_response_shapes_are_classified_without_model_prefix(self):
        import importlib.util

        responses = [
            types.SimpleNamespace(choices=[]),
            types.SimpleNamespace(),
            types.SimpleNamespace(choices=[types.SimpleNamespace(message=None)]),
            types.SimpleNamespace(choices=[types.SimpleNamespace(message=types.SimpleNamespace(content=None))]),
            types.SimpleNamespace(choices=[types.SimpleNamespace(message=types.SimpleNamespace(content="  "))]),
        ]
        for response in responses:
            with self.subTest(response=response):
                class FakeCompletions:
                    def create(self, **kwargs):
                        return response

                class FakeClient:
                    def __init__(self, **kwargs):
                        self.chat = types.SimpleNamespace(completions=FakeCompletions())

                spec = importlib.util.spec_from_file_location("lesson_hello_invalid", HELLO)
                module = importlib.util.module_from_spec(spec)
                assert spec and spec.loader
                spec.loader.exec_module(module)
                with mock.patch.dict(sys.modules, {"openai": types.SimpleNamespace(OpenAI=FakeClient)}), mock.patch.dict(
                    os.environ,
                    {"REIN_BASE_URL": "https://mock.local/v1", "REIN_API_KEY": "secret", "REIN_MODEL": "mock"},
                    clear=False,
                ), contextlib.redirect_stdout(io.StringIO()) as output, contextlib.redirect_stderr(io.StringIO()) as errors:
                    self.assertEqual(module.main(), 1)
                self.assertIn("未收到有效的回答文本", errors.getvalue())
                self.assertNotIn("模型：", output.getvalue())

    def test_chat_two_successful_turns_keep_history(self):
        import importlib.util

        calls: list[dict] = []
        answers = iter(["第一轮回答", "第二轮回答"])

        class FakeCompletions:
            def create(self, **kwargs):
                calls.append(kwargs)
                return types.SimpleNamespace(
                    choices=[types.SimpleNamespace(message=types.SimpleNamespace(content=next(answers)))]
                )

        class FakeClient:
            def __init__(self, **kwargs):
                self.chat = types.SimpleNamespace(completions=FakeCompletions())

        spec = importlib.util.spec_from_file_location("lesson_chat", CHAT)
        module = importlib.util.module_from_spec(spec)
        assert spec and spec.loader
        spec.loader.exec_module(module)
        with mock.patch.dict(sys.modules, {"openai": types.SimpleNamespace(OpenAI=FakeClient)}), mock.patch.dict(
            os.environ,
            {"REIN_BASE_URL": "https://mock.local/v1", "REIN_API_KEY": "secret", "REIN_MODEL": "mock"},
            clear=False,
        ), mock.patch("builtins.input", side_effect=["你好", "继续", "/exit"]), contextlib.redirect_stdout(io.StringIO()) as output:
            self.assertEqual(module.main(), 0)
        self.assertIn("模型：第一轮回答", output.getvalue())
        self.assertIn("模型：第二轮回答", output.getvalue())
        self.assertEqual(len(calls), 2)
        self.assertEqual([item["role"] for item in calls[1]["messages"]], ["system", "user", "assistant", "user"])
        self.assertEqual(calls[1]["messages"][2]["content"], "第一轮回答")
        self.assertFalse(calls[0]["stream"])
        self.assertEqual(calls[0]["timeout"], 30)

    def test_chat_failed_middle_turn_does_not_pollute_history_and_continues(self):
        import importlib.util

        calls: list[dict] = []
        outcomes = iter(["ok", ConnectionError("secret-connect-details"), "recovered"])

        class FakeCompletions:
            def create(self, **kwargs):
                calls.append(kwargs)
                outcome = next(outcomes)
                if isinstance(outcome, Exception):
                    raise outcome
                return types.SimpleNamespace(choices=[types.SimpleNamespace(message=types.SimpleNamespace(content=outcome))])

        class FakeClient:
            def __init__(self, **kwargs):
                self.chat = types.SimpleNamespace(completions=FakeCompletions())

        spec = importlib.util.spec_from_file_location("lesson_chat_failure", CHAT)
        module = importlib.util.module_from_spec(spec)
        assert spec and spec.loader
        spec.loader.exec_module(module)
        with mock.patch.dict(sys.modules, {"openai": types.SimpleNamespace(OpenAI=FakeClient)}), mock.patch.dict(
            os.environ,
            {"REIN_BASE_URL": "https://mock.local/v1", "REIN_API_KEY": "secret", "REIN_MODEL": "mock"},
            clear=False,
        ), mock.patch("builtins.input", side_effect=["第一问", "失败这一问", "第三问", "/exit"]), contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()) as errors:
            self.assertEqual(module.main(), 0)
        self.assertEqual(len(calls), 3)
        self.assertEqual([message["content"] for message in calls[2]["messages"] if message["role"] == "user"], ["第一问", "第三问"])
        self.assertIn("无法连接模型服务", errors.getvalue())
        self.assertNotIn("secret-connect-details", errors.getvalue())

    def test_chat_blank_input_and_exit_do_not_call_api(self):
        import importlib.util

        spec = importlib.util.spec_from_file_location("lesson_chat_empty", CHAT)
        module = importlib.util.module_from_spec(spec)
        assert spec and spec.loader
        spec.loader.exec_module(module)
        class FakeCompletions:
            def create(self, **kwargs):
                raise AssertionError("blank input must not send a request")

        class FakeClient:
            def __init__(self, **kwargs):
                self.chat = types.SimpleNamespace(completions=FakeCompletions())

        with mock.patch.dict(sys.modules, {"openai": types.SimpleNamespace(OpenAI=FakeClient)}), mock.patch.dict(
            os.environ,
            {"REIN_BASE_URL": "https://mock.local/v1", "REIN_API_KEY": "secret", "REIN_MODEL": "mock"},
            clear=False,
        ), mock.patch("builtins.input", side_effect=["", "   ", "/exit"]), contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(module.main(), 0)

    def test_chat_missing_configuration_and_eof_are_graceful(self):
        result = run_script(CHAT, stdin="/exit\n")
        self.assertEqual(result.returncode, 1)
        self.assertIn("API 尚未正确配置", result.stdout)
        self.assertIn("REIN_BASE_URL、REIN_API_KEY、REIN_MODEL", result.stdout)

    def test_classified_errors_are_safe_and_concise(self):
        import importlib.util

        spec = importlib.util.spec_from_file_location("lesson_chat_errors", CHAT)
        module = importlib.util.module_from_spec(spec)
        assert spec and spec.loader
        spec.loader.exec_module(module)
        cases = [
            (TimeoutError("secret"), "请求超时"),
            (ConnectionError("secret"), "无法连接模型服务"),
            (RuntimeError("secret"), "请求失败"),
        ]
        for error, expected in cases:
            with self.subTest(error=type(error).__name__):
                message = module.classify_error(error)
                self.assertIn(expected, message)
                self.assertNotIn("secret", message)

        class ProviderError(Exception):
            def __init__(self, status_code):
                super().__init__("provider body contains secret")
                self.status_code = status_code

        for status, expected in ((401, "认证失败"), (429, "请求受限"), (503, "HTTP 503")):
            with self.subTest(status=status):
                message = module.classify_error(ProviderError(status))
                self.assertIn(expected, message)
                self.assertNotIn("secret", message)

    @unittest.skipUnless(httpx is not None and SDKOpenAI is not None, "openai and httpx are required for the SDK transport test")
    def test_real_sdk_mock_transport_returns_text_without_network(self):
        import importlib.util
        import json

        spec = importlib.util.spec_from_file_location("lesson_chat_transport", CHAT)
        module = importlib.util.module_from_spec(spec)
        assert spec and spec.loader
        spec.loader.exec_module(module)
        requests: list[dict] = []

        def handler(request: httpx.Request) -> httpx.Response:
            requests.append(json.loads(request.content))
            return httpx.Response(
                200,
                json={"choices": [{"message": {"role": "assistant", "content": "模拟回答"}}]},
                request=request,
            )

        with httpx.Client(transport=httpx.MockTransport(handler)) as http_client:
            client = SDKOpenAI(
                api_key="secret-key",
                base_url="https://mock.local/v1",
                http_client=http_client,
                max_retries=0,
            )
            answer = module.ask(client, [{"role": "user", "content": "你好"}], "mock-model")
        self.assertEqual(answer, "模拟回答")
        self.assertEqual(requests[0]["model"], "mock-model")
        self.assertFalse(requests[0]["stream"])

    def test_invalid_response_shapes_are_reported_and_do_not_enter_history(self):
        import importlib.util

        shapes = [
            types.SimpleNamespace(choices=[]),
            types.SimpleNamespace(choices=[types.SimpleNamespace(message=None)]),
            types.SimpleNamespace(choices=[types.SimpleNamespace(message=types.SimpleNamespace(content=" "))]),
        ]
        for response in shapes:
            with self.subTest(response=response):
                class FakeCompletions:
                    def create(self, **kwargs):
                        return response

                class FakeClient:
                    def __init__(self, **kwargs):
                        self.chat = types.SimpleNamespace(completions=FakeCompletions())

                spec = importlib.util.spec_from_file_location("lesson_chat_shape", CHAT)
                module = importlib.util.module_from_spec(spec)
                assert spec and spec.loader
                spec.loader.exec_module(module)
                with mock.patch.dict(sys.modules, {"openai": types.SimpleNamespace(OpenAI=FakeClient)}), mock.patch.dict(
                    os.environ,
                    {"REIN_BASE_URL": "https://mock.local/v1", "REIN_API_KEY": "secret", "REIN_MODEL": "mock"},
                    clear=False,
                ), mock.patch("builtins.input", side_effect=["问题", "/exit"]), contextlib.redirect_stderr(io.StringIO()) as errors:
                    self.assertEqual(module.main(), 0)
                self.assertIn("未收到有效的回答文本", errors.getvalue())

    def test_http_status_errors_are_safe_and_following_turn_can_succeed(self):
        import importlib.util

        class ProviderError(Exception):
            def __init__(self, status_code):
                super().__init__(f"secret body for {status_code}")
                self.status_code = status_code

        outcomes = iter([ProviderError(401), ProviderError(429), ProviderError(503), "最后回答"])
        calls: list[dict] = []

        class FakeCompletions:
            def create(self, **kwargs):
                calls.append(kwargs)
                outcome = next(outcomes)
                if isinstance(outcome, Exception):
                    raise outcome
                return types.SimpleNamespace(choices=[types.SimpleNamespace(message=types.SimpleNamespace(content=outcome))])

        class FakeClient:
            def __init__(self, **kwargs):
                self.chat = types.SimpleNamespace(completions=FakeCompletions())

        spec = importlib.util.spec_from_file_location("lesson_chat_status", CHAT)
        module = importlib.util.module_from_spec(spec)
        assert spec and spec.loader
        spec.loader.exec_module(module)
        with mock.patch.dict(sys.modules, {"openai": types.SimpleNamespace(OpenAI=FakeClient)}), mock.patch.dict(
            os.environ,
            {"REIN_BASE_URL": "https://mock.local/v1", "REIN_API_KEY": "secret", "REIN_MODEL": "mock"},
            clear=False,
        ), mock.patch("builtins.input", side_effect=["一", "二", "三", "四", "/exit"]), contextlib.redirect_stdout(io.StringIO()) as stdout, contextlib.redirect_stderr(io.StringIO()) as errors:
            self.assertEqual(module.main(), 0)
        output = errors.getvalue()
        self.assertIn("认证失败", output)
        self.assertIn("HTTP 429", output)
        self.assertIn("HTTP 503", output)
        self.assertIn("最后回答", stdout.getvalue())
        self.assertNotIn("secret body", output)
        self.assertEqual([item["content"] for item in calls[-1]["messages"] if item["role"] == "user"], ["四"])

    @unittest.skipUnless(httpx is not None and SDKOpenAI is not None, "openai and httpx are required for the SDK transport test")
    def test_real_sdk_mock_transport_status_errors_then_recovers(self):
        import importlib.util
        import json

        spec = importlib.util.spec_from_file_location("lesson_chat_status_transport", CHAT)
        module = importlib.util.module_from_spec(spec)
        assert spec and spec.loader
        spec.loader.exec_module(module)
        statuses = iter([401, 429, 503, 200])
        requests: list[dict] = []

        def handler(request: httpx.Request) -> httpx.Response:
            requests.append(json.loads(request.content))
            status = next(statuses)
            if status == 200:
                return httpx.Response(200, json={"choices": [{"message": {"content": "恢复成功"}}]}, request=request)
            return httpx.Response(status, json={"error": {"message": "secret provider body"}}, request=request)

        with httpx.Client(transport=httpx.MockTransport(handler)) as http_client:
            sdk_client = SDKOpenAI(
                api_key="secret-key",
                base_url="https://mock.local/v1",
                http_client=http_client,
                max_retries=0,
            )
            fake_openai = types.SimpleNamespace(OpenAI=lambda **kwargs: sdk_client)
            with mock.patch.dict(sys.modules, {"openai": fake_openai}), mock.patch.dict(
                os.environ,
                {"REIN_BASE_URL": "https://mock.local/v1", "REIN_API_KEY": "secret", "REIN_MODEL": "mock"},
                clear=False,
            ), mock.patch("builtins.input", side_effect=["一", "二", "三", "四", "/exit"]), contextlib.redirect_stdout(io.StringIO()) as stdout, contextlib.redirect_stderr(io.StringIO()) as stderr:
                self.assertEqual(module.main(), 0)
        self.assertIn("认证失败", stderr.getvalue())
        self.assertIn("HTTP 429", stderr.getvalue())
        self.assertIn("HTTP 503", stderr.getvalue())
        self.assertIn("模型：恢复成功", stdout.getvalue())
        self.assertNotIn("secret provider body", stderr.getvalue())
        self.assertEqual(len(requests), 4)

    def test_ctrl_c_during_request_and_eof_after_start_are_graceful(self):
        import importlib.util

        class InterruptingCompletions:
            def create(self, **kwargs):
                raise KeyboardInterrupt

        class FakeClient:
            def __init__(self, **kwargs):
                self.chat = types.SimpleNamespace(completions=InterruptingCompletions())

        spec = importlib.util.spec_from_file_location("lesson_chat_interrupt", CHAT)
        module = importlib.util.module_from_spec(spec)
        assert spec and spec.loader
        spec.loader.exec_module(module)
        with mock.patch.dict(sys.modules, {"openai": types.SimpleNamespace(OpenAI=FakeClient)}), mock.patch.dict(
            os.environ,
            {"REIN_BASE_URL": "https://mock.local/v1", "REIN_API_KEY": "secret", "REIN_MODEL": "mock"},
            clear=False,
        ), mock.patch("builtins.input", side_effect=["问题"]):
            self.assertEqual(module.main(), 0)

        with mock.patch.dict(sys.modules, {"openai": types.SimpleNamespace(OpenAI=FakeClient)}), mock.patch.dict(
            os.environ,
            {"REIN_BASE_URL": "https://mock.local/v1", "REIN_API_KEY": "secret", "REIN_MODEL": "mock"},
            clear=False,
        ), mock.patch("builtins.input", side_effect=EOFError):
            self.assertEqual(module.main(), 0)


if __name__ == "__main__":
    unittest.main()
