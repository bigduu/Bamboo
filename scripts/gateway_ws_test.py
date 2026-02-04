#!/usr/bin/env python3
"""
Gateway WebSocket test script.

Covers:
- Basic WebSocket connection
- Connect handshake (new + reconnect)
- Client Ping -> Gateway Pong
- Command ping/status/unknown
- Chat -> real LLM response (AgentToken)
- Error handling (NOT_CONNECTED, INVALID_MESSAGE)
- Optional auth checks

Usage:
  python scripts/gateway_ws_test.py \
    --ws-url ws://127.0.0.1:18790 \
    --llm-message "Hello" \
    --llm-timeout 60

Requires:
  pip install websockets
"""

import argparse
import asyncio
import json
import sys
import time
import uuid
from dataclasses import dataclass
from typing import Any, Callable, Dict, List, Optional, Tuple

try:
    import websockets
except ImportError as exc:  # pragma: no cover
    raise SystemExit(
        "Missing dependency: websockets. Install with `pip install websockets`."
    ) from exc


DEFAULT_WS_URL = "ws://127.0.0.1:18790"


@dataclass
class TestResult:
    name: str
    passed: bool
    duration: float
    detail: str = ""


class TestFailure(Exception):
    pass


def normalize_type(value: Any) -> str:
    if not isinstance(value, str):
        return ""
    v = value.replace("-", "_")
    out = []
    for i, ch in enumerate(v):
        if ch.isupper():
            if i and (v[i - 1].islower() or (i + 1 < len(v) and v[i + 1].islower())):
                out.append("_")
            out.append(ch.lower())
        else:
            out.append(ch.lower())
    return "".join(out)


def type_is(payload: Dict[str, Any], expected: str) -> bool:
    return normalize_type(payload.get("type")) == expected


def maybe_strip_none(payload: Dict[str, Any]) -> Dict[str, Any]:
    return {k: v for k, v in payload.items() if v is not None}


async def recv_json(ws, timeout: float) -> Tuple[Dict[str, Any], str]:
    raw = await asyncio.wait_for(ws.recv(), timeout=timeout)
    if isinstance(raw, bytes):
        raw = raw.decode("utf-8", errors="replace")
    try:
        return json.loads(raw), raw
    except json.JSONDecodeError as exc:
        raise TestFailure(f"Invalid JSON from gateway: {raw}") from exc


async def wait_for_event(
    ws,
    predicate: Callable[[Dict[str, Any]], bool],
    timeout: float,
    collected: Optional[List[Dict[str, Any]]] = None,
) -> Dict[str, Any]:
    deadline = time.monotonic() + timeout
    while True:
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise TestFailure("Timed out waiting for expected event")
        payload, _ = await recv_json(ws, timeout=remaining)
        if collected is not None:
            collected.append(payload)
        if predicate(payload):
            return payload


class GatewayTester:
    def __init__(
        self,
        ws_url: str,
        auth_token: Optional[str],
        expect_auth: bool,
        llm_message: str,
        timeout: float,
        llm_timeout: float,
        verbose: bool,
    ) -> None:
        self.ws_url = ws_url
        self.auth_token = auth_token
        self.expect_auth = expect_auth
        self.llm_message = llm_message
        self.timeout = timeout
        self.llm_timeout = llm_timeout
        self.verbose = verbose

    def log(self, message: str) -> None:
        if self.verbose:
            print(message)

    async def connect(self, session_id: Optional[str] = None, auth_token: Optional[str] = None):
        ws = await websockets.connect(self.ws_url)
        connect_msg = maybe_strip_none(
            {
                "type": "connect",
                "session_id": session_id,
                "auth": auth_token,
            }
        )
        await ws.send(json.dumps(connect_msg))
        event = await wait_for_event(
            ws,
            lambda e: type_is(e, "connected") or type_is(e, "error"),
            self.timeout,
        )
        if type_is(event, "error"):
            code = event.get("code")
            msg = event.get("message")
            await ws.close()
            raise TestFailure(f"Connect failed: {code} {msg}")
        return ws, event.get("session_id")

    async def test_basic_connection(self) -> str:
        async with websockets.connect(self.ws_url) as ws:
            if not ws.open:
                raise TestFailure("WebSocket not open after connect")
        return "connected"

    async def test_connect_and_reconnect(self) -> str:
        ws, session_id = await self.connect(auth_token=self.auth_token)
        await ws.close()
        if not session_id:
            raise TestFailure("Gateway did not return session_id")
        ws2, session_id2 = await self.connect(session_id=session_id, auth_token=self.auth_token)
        await ws2.close()
        if session_id2 != session_id:
            raise TestFailure(f"Session restore mismatch: {session_id} vs {session_id2}")
        return f"session_id={session_id}"

    async def test_ping_message(self) -> str:
        ws, _session_id = await self.connect(auth_token=self.auth_token)
        await ws.send(json.dumps({"type": "ping", "timestamp": int(time.time() * 1000)}))
        event = await wait_for_event(ws, lambda e: type_is(e, "pong"), self.timeout)
        await ws.close()
        return f"pong_ts={event.get('timestamp')}"

    async def test_command_ping(self) -> str:
        ws, _session_id = await self.connect(auth_token=self.auth_token)
        await ws.send(json.dumps({"type": "command", "name": "ping", "args": {}}))
        event = await wait_for_event(ws, lambda e: type_is(e, "pong"), self.timeout)
        await ws.close()
        return f"pong_ts={event.get('timestamp')}"

    async def test_command_status(self) -> str:
        ws, session_id = await self.connect(auth_token=self.auth_token)
        await ws.send(json.dumps({"type": "command", "name": "status", "args": {}}))
        event = await wait_for_event(ws, lambda e: type_is(e, "agent_token"), self.timeout)
        await ws.close()
        token = event.get("token", "")
        if session_id and session_id not in token:
            raise TestFailure(f"Status token does not include session id: {token}")
        return token

    async def test_command_unknown(self) -> str:
        ws, _session_id = await self.connect(auth_token=self.auth_token)
        name = f"tool_{uuid.uuid4().hex[:8]}"
        await ws.send(json.dumps({"type": "command", "name": name, "args": {"sample": True}}))
        event = await wait_for_event(ws, lambda e: type_is(e, "agent_tool_start"), self.timeout)
        await ws.close()
        if event.get("tool") != name:
            raise TestFailure(f"Expected tool {name}, got {event.get('tool')}")
        return f"tool={name}"

    async def test_chat_llm(self) -> str:
        ws, session_id = await self.connect(auth_token=self.auth_token)
        await ws.send(
            json.dumps(
                {
                    "type": "chat",
                    "session_id": session_id,
                    "content": self.llm_message,
                }
            )
        )

        events: List[Dict[str, Any]] = []
        token_parts: List[str] = []

        def is_token_or_error(payload: Dict[str, Any]) -> bool:
            return (
                type_is(payload, "agent_token")
                or type_is(payload, "error")
                or type_is(payload, "agent_complete")
            )

        deadline = time.monotonic() + self.llm_timeout
        while time.monotonic() < deadline:
            remaining = deadline - time.monotonic()
            try:
                event = await wait_for_event(ws, is_token_or_error, remaining, collected=events)
            except TestFailure:
                break
            if type_is(event, "error"):
                await ws.close()
                raise TestFailure(
                    f"Gateway returned error: {event.get('code')} {event.get('message')}"
                )
            if type_is(event, "agent_token"):
                token = event.get("token", "")
                if token:
                    token_parts.append(token)
            if type_is(event, "agent_complete"):
                break

        await ws.close()

        if not token_parts:
            sample = json.dumps(events[:3], ensure_ascii=False)
            raise TestFailure(f"No AgentToken received (events={sample})")
        return f"tokens={len(token_parts)}"

    async def test_chat_without_connect(self) -> str:
        async with websockets.connect(self.ws_url) as ws:
            await ws.send(json.dumps({"type": "chat", "session_id": "nope", "content": "hi"}))
            event = await wait_for_event(ws, lambda e: type_is(e, "error"), self.timeout)
            if event.get("code") != "NOT_CONNECTED":
                raise TestFailure(f"Expected NOT_CONNECTED, got {event.get('code')}")
        return "NOT_CONNECTED"

    async def test_invalid_json(self) -> str:
        async with websockets.connect(self.ws_url) as ws:
            await ws.send("{invalid-json")
            event = await wait_for_event(ws, lambda e: type_is(e, "error"), self.timeout)
            if event.get("code") != "INVALID_MESSAGE":
                raise TestFailure(f"Expected INVALID_MESSAGE, got {event.get('code')}")
        return "INVALID_MESSAGE"

    async def test_auth_required(self) -> str:
        if not self.auth_token:
            raise TestFailure("auth_token not provided")
        # Expect UNAUTHORIZED when using wrong token
        async with websockets.connect(self.ws_url) as ws:
            await ws.send(json.dumps({"type": "connect", "auth": "wrong-token"}))
            event = await wait_for_event(
                ws,
                lambda e: type_is(e, "error") or type_is(e, "connected"),
                self.timeout,
            )
            if type_is(event, "connected"):
                if self.expect_auth:
                    raise TestFailure("Gateway accepted invalid auth token")
                return "auth not enforced"
            if event.get("code") != "UNAUTHORIZED":
                raise TestFailure(f"Expected UNAUTHORIZED, got {event.get('code')}")
        return "UNAUTHORIZED"


async def run_test(name: str, coro: Callable[[], Any]) -> TestResult:
    start = time.monotonic()
    try:
        detail = await coro()
        return TestResult(name=name, passed=True, duration=time.monotonic() - start, detail=str(detail))
    except Exception as exc:
        return TestResult(name=name, passed=False, duration=time.monotonic() - start, detail=str(exc))


def print_results(results: List[TestResult]) -> int:
    total = len(results)
    passed = sum(1 for r in results if r.passed)
    failed = total - passed

    print("\nGateway WebSocket Test Results")
    print("=" * 32)
    for result in results:
        status = "PASS" if result.passed else "FAIL"
        print(f"{status:4} {result.name} ({result.duration:.2f}s) {result.detail}")
    print("-" * 32)
    print(f"Total: {total}, Passed: {passed}, Failed: {failed}")

    return 0 if failed == 0 else 1


def parse_args(argv: List[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Gateway WebSocket test script")
    parser.add_argument("--ws-url", default=DEFAULT_WS_URL, help="Gateway WebSocket URL")
    parser.add_argument("--auth-token", default=None, help="Auth token for Connect")
    parser.add_argument(
        "--expect-auth", action="store_true", help="Fail if gateway accepts invalid auth"
    )
    parser.add_argument(
        "--llm-message",
        default="Hello from gateway test. Reply briefly.",
        help="Message for LLM test",
    )
    parser.add_argument(
        "--timeout", type=float, default=10.0, help="Timeout (seconds) for non-LLM events"
    )
    parser.add_argument(
        "--llm-timeout", type=float, default=60.0, help="Timeout (seconds) for LLM response"
    )
    parser.add_argument("--skip-llm", action="store_true", help="Skip real LLM chat test")
    parser.add_argument("--verbose", action="store_true", help="Verbose output")
    return parser.parse_args(argv)


async def main_async(args: argparse.Namespace) -> int:
    tester = GatewayTester(
        ws_url=args.ws_url,
        auth_token=args.auth_token,
        expect_auth=args.expect_auth,
        llm_message=args.llm_message,
        timeout=args.timeout,
        llm_timeout=args.llm_timeout,
        verbose=args.verbose,
    )

    tests: List[Tuple[str, Callable[[], Any]]] = [
        ("basic_connection", tester.test_basic_connection),
        ("connect_and_reconnect", tester.test_connect_and_reconnect),
        ("ping_message", tester.test_ping_message),
        ("command_ping", tester.test_command_ping),
        ("command_status", tester.test_command_status),
        ("command_unknown", tester.test_command_unknown),
        ("chat_without_connect", tester.test_chat_without_connect),
        ("invalid_json", tester.test_invalid_json),
    ]

    if args.auth_token:
        tests.append(("auth_required", tester.test_auth_required))

    if not args.skip_llm:
        tests.append(("chat_llm", tester.test_chat_llm))

    results: List[TestResult] = []
    for name, fn in tests:
        results.append(await run_test(name, fn))

    return print_results(results)


def main(argv: List[str]) -> int:
    args = parse_args(argv)
    try:
        return asyncio.run(main_async(args))
    except KeyboardInterrupt:
        print("Interrupted")
        return 130


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
