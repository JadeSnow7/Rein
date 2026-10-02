import unittest
import asyncio
import time


class SDKMockContractTests(unittest.TestCase):
    def test_injected_openai_client_is_used_without_network(self):
        try:
            from rein_core import OpenAIAdapter, Task
        except ModuleNotFoundError:
            self.skipTest("implementation not started")

        class FakeCompletions:
            def __init__(self):
                self.calls = []

            async def create(self, **kwargs):
                self.calls.append(kwargs)
                return {"choices": [{"message": {"content": "offline SDK mock"}}]}

        class FakeClient:
            def __init__(self):
                self.chat = type("Chat", (), {"completions": FakeCompletions()})()

        client = FakeClient()
        adapter = OpenAIAdapter(client=client, model="mock-model", retries=0)
        response = asyncio.run(adapter.request(Task("hello", None, (), "opinion"), [{"role": "user", "content": "hello"}], deadline=time.monotonic() + 1))
        self.assertEqual(response.text, "offline SDK mock")
        self.assertEqual(len(client.chat.completions.calls), 1)

    def test_final_content_with_tool_call_is_rejected_and_wire_strips_trace_fields(self):
        from rein_core import OpenAIAdapter, ReinError, Task

        class Completions:
            async def create(self, **kwargs):
                self.kwargs = kwargs
                return {"choices": [{"message": {"content": "final", "tool_calls": [{"id": "late", "type": "function", "function": {"name": "read_file", "arguments": "{}"}}]}}]}

        class Client:
            def __init__(self):
                self.chat = type("Chat", (), {"completions": Completions()})()

        async def exercise():
            client = Client()
            adapter = OpenAIAdapter(client, "mock", retries=0)
            try:
                await adapter.request(Task("hello", None, (), "opinion"), [{"role": "user", "content": "x", "ok": True, "output": "secret"}], time.monotonic() + 1)
            except ReinError as exc:
                self.assertEqual(exc.code, "response_invalid")
            else:
                self.fail("late tool call was accepted")
            self.assertNotIn("ok", client.chat.completions.kwargs["messages"][0])
            self.assertNotIn("output", client.chat.completions.kwargs["messages"][0])

        asyncio.run(exercise())

    def test_real_async_openai_client_uses_local_httpx_transport(self):
        try:
            import httpx
            from openai import AsyncOpenAI
            from rein_core import OpenAIAdapter, Task
        except ModuleNotFoundError:
            self.skipTest("openai 2.26.0 environment not selected")

        async def handler(request):
            return httpx.Response(200, json={"choices": [{"message": {"content": "local transport"}}]})

        async def exercise():
            client = AsyncOpenAI(api_key="test", http_client=httpx.AsyncClient(transport=httpx.MockTransport(handler), base_url="https://mock.local/v1"))
            try:
                adapter = OpenAIAdapter(client=client, model="mock-model", retries=0)
                response = await adapter.request(Task("hello", None, (), "opinion"), [{"role": "user", "content": "hello"}], deadline=time.monotonic() + 1)
                self.assertEqual(response.text, "local transport")
            finally:
                await client.close()

        asyncio.run(exercise())

    def test_real_async_openai_tool_roundtrip_is_local_and_two_requests(self):
        try:
            import httpx
            from openai import AsyncOpenAI
            from rein_core import OpenAIAdapter, Task, run
        except ModuleNotFoundError:
            self.skipTest("openai 2.26.0 environment not selected")

        async def exercise():
            seen = []

            async def handler(request):
                seen.append(request)
                if len(seen) == 1:
                    body = {"choices": [{"message": {"tool_calls": [{"id": "call_1", "type": "function", "function": {"name": "read_file", "arguments": '{"path":"README.md"}'}}]}}]}
                else:
                    body = {"choices": [{"message": {"content": '{"suggestion":{"path":"README.md","original":"npm run start","suggested":"npm run dev","reason":"test"}}'}}]}
                return httpx.Response(200, json=body)

            client = AsyncOpenAI(api_key="test", http_client=httpx.AsyncClient(transport=httpx.MockTransport(handler), base_url="https://mock.local/v1"))
            try:
                adapter = OpenAIAdapter(client=client, model="mock-model", retries=0)
                task = Task("check npm run dev", Path(__file__).resolve().parents[1] / "fixtures" / "outdated", ("README.md",), "suggest")
                record = await run(task, "suggest", adapter, read_mode="tool", timeout=1)
                self.assertEqual(record.status, "unique")
                self.assertEqual(len(seen), 2)
            finally:
                await client.close()

        from pathlib import Path
        asyncio.run(exercise())


if __name__ == "__main__":
    unittest.main()
