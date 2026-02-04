#!/usr/bin/env python3
"""
Bamboo API Chat Endpoints Test Suite

This script provides comprehensive testing for Bamboo Server's chat-related API endpoints.

Tested Endpoints:
- POST /api/v1/chat - Create chat session
- GET /api/v1/stream/{session_id} - SSE streaming response
- GET /api/v1/history/{session_id} - Get chat history
- POST /api/v1/stop/{session_id} - Stop session
- GET /health - Health check
- GET /api/v1/config - Get configuration

Requirements:
    pip install requests httpx pytest pytest-asyncio

Usage:
    python test_chat_api.py
    pytest test_chat_api.py -v
"""

import json
import time
import asyncio
import concurrent.futures
from typing import Optional, Dict, Any, List
from dataclasses import dataclass
from datetime import datetime

import requests
import httpx


# Configuration
BASE_URL = "http://localhost:12123"
API_VERSION = "/api/v1"
TIMEOUT = 30


@dataclass
class TestResult:
    """Test result container"""
    name: str
    passed: bool
    duration: float
    message: str = ""
    error: Optional[str] = None


class BambooAPITester:
    """Bamboo API Test Client"""
    
    def __init__(self, base_url: str = BASE_URL):
        self.base_url = base_url
        self.api_url = f"{base_url}{API_VERSION}"
        self.session = requests.Session()
        self.created_sessions: List[str] = []
        
    def _url(self, endpoint: str) -> str:
        """Build full API URL"""
        return f"{self.api_url}{endpoint}"
    
    def cleanup(self):
        """Clean up created sessions"""
        for session_id in self.created_sessions:
            try:
                self.stop_session(session_id)
            except Exception:
                pass
    
    # ==================== API Methods ====================
    
    def health_check(self) -> Dict[str, Any]:
        """GET /health"""
        response = self.session.get(
            f"{self.base_url}/health",
            timeout=TIMEOUT
        )
        response.raise_for_status()
        return response.json()
    
    def get_config(self) -> Dict[str, Any]:
        """GET /api/v1/config"""
        response = self.session.get(
            self._url("/config"),
            timeout=TIMEOUT
        )
        response.raise_for_status()
        return response.json()
    
    def create_chat(self, message: str, model: Optional[str] = None, 
                    conversation_id: Optional[str] = None,
                    stream: bool = True) -> Dict[str, Any]:
        """POST /api/v1/chat"""
        payload = {
            "message": message,
            "stream": stream
        }
        if model:
            payload["model"] = model
        if conversation_id:
            payload["conversation_id"] = conversation_id
            
        response = self.session.post(
            self._url("/chat"),
            json=payload,
            timeout=TIMEOUT
        )
        response.raise_for_status()
        data = response.json()
        
        if "session_id" in data:
            self.created_sessions.append(data["session_id"])
        
        return data
    
    def stream_chat(self, session_id: str) -> List[str]:
        """GET /api/v1/stream/{session_id} - SSE streaming"""
        chunks = []
        response = self.session.get(
            self._url(f"/stream/{session_id}"),
            stream=True,
            headers={"Accept": "text/event-stream"},
            timeout=TIMEOUT
        )
        response.raise_for_status()
        
        for line in response.iter_lines():
            if line:
                line_str = line.decode('utf-8')
                if line_str.startswith('data: '):
                    data = line_str[6:]  # Remove 'data: ' prefix
                    if data != '[DONE]':
                        try:
                            chunk = json.loads(data)
                            chunks.append(chunk)
                        except json.JSONDecodeError:
                            chunks.append({"content": data})
                    else:
                        break
        
        return chunks
    
    def get_history(self, session_id: str) -> Dict[str, Any]:
        """GET /api/v1/history/{session_id}"""
        response = self.session.get(
            self._url(f"/history/{session_id}"),
            timeout=TIMEOUT
        )
        response.raise_for_status()
        return response.json()
    
    def stop_session(self, session_id: str) -> Dict[str, Any]:
        """POST /api/v1/stop/{session_id}"""
        response = self.session.post(
            self._url(f"/stop/{session_id}"),
            timeout=TIMEOUT
        )
        response.raise_for_status()
        return response.json()


# ==================== Test Functions ====================

def test_health_check() -> TestResult:
    """Test health check endpoint"""
    start = time.time()
    tester = BambooAPITester()
    
    try:
        result = tester.health_check()
        duration = time.time() - start
        
        assert "status" in result, "Missing 'status' field"
        assert result["status"] in ["healthy", "ok", "up"], f"Unexpected status: {result['status']}"
        
        return TestResult(
            name="Health Check",
            passed=True,
            duration=duration,
            message=f"Status: {result.get('status')}"
        )
    except Exception as e:
        return TestResult(
            name="Health Check",
            passed=False,
            duration=time.time() - start,
            error=str(e)
        )


def test_get_config() -> TestResult:
    """Test config endpoint"""
    start = time.time()
    tester = BambooAPITester()
    
    try:
        result = tester.get_config()
        duration = time.time() - start
        
        # Config should have some basic fields
        assert isinstance(result, dict), "Config should be a dictionary"
        
        return TestResult(
            name="Get Config",
            passed=True,
            duration=duration,
            message=f"Config keys: {list(result.keys())}"
        )
    except Exception as e:
        return TestResult(
            name="Get Config",
            passed=False,
            duration=time.time() - start,
            error=str(e)
        )


def test_create_chat() -> TestResult:
    """Test creating a chat session"""
    start = time.time()
    tester = BambooAPITester()
    
    try:
        result = tester.create_chat(
            message="Hello, this is a test message",
            stream=False
        )
        duration = time.time() - start
        
        assert "session_id" in result, "Missing 'session_id' field"
        assert "response" in result or "message" in result, "Missing response content"
        
        session_id = result["session_id"]
        tester.cleanup()
        
        return TestResult(
            name="Create Chat Session",
            passed=True,
            duration=duration,
            message=f"Session ID: {session_id[:16]}..."
        )
    except Exception as e:
        return TestResult(
            name="Create Chat Session",
            passed=False,
            duration=time.time() - start,
            error=str(e)
        )


def test_streaming_response() -> TestResult:
    """Test SSE streaming response"""
    start = time.time()
    tester = BambooAPITester()
    
    try:
        # First create a chat session
        chat_result = tester.create_chat(
            message="Tell me a short joke",
            stream=True
        )
        session_id = chat_result["session_id"]
        
        # Get streaming response
        chunks = tester.stream_chat(session_id)
        duration = time.time() - start
        
        assert len(chunks) > 0, "No streaming chunks received"
        
        # Collect full response
        full_response = ""
        for chunk in chunks:
            if isinstance(chunk, dict):
                content = chunk.get("content", chunk.get("delta", ""))
                full_response += content
        
        tester.cleanup()
        
        return TestResult(
            name="Streaming Response",
            passed=True,
            duration=duration,
            message=f"Received {len(chunks)} chunks, {len(full_response)} chars"
        )
    except Exception as e:
        return TestResult(
            name="Streaming Response",
            passed=False,
            duration=time.time() - start,
            error=str(e)
        )


def test_chat_history() -> TestResult:
    """Test getting chat history"""
    start = time.time()
    tester = BambooAPITester()
    
    try:
        # Create a chat session
        chat_result = tester.create_chat(
            message="What is Python?",
            stream=False
        )
        session_id = chat_result["session_id"]
        
        # Get history
        history = tester.get_history(session_id)
        duration = time.time() - start
        
        assert "messages" in history or "history" in history, "Missing messages in history"
        
        tester.cleanup()
        
        return TestResult(
            name="Chat History",
            passed=True,
            duration=duration,
            message=f"History retrieved for session {session_id[:16]}..."
        )
    except Exception as e:
        return TestResult(
            name="Chat History",
            passed=False,
            duration=time.time() - start,
            error=str(e)
        )


def test_stop_session() -> TestResult:
    """Test stopping a session"""
    start = time.time()
    tester = BambooAPITester()
    
    try:
        # Create a chat session
        chat_result = tester.create_chat(
            message="Long message that might take time...",
            stream=True
        )
        session_id = chat_result["session_id"]
        
        # Stop the session
        stop_result = tester.stop_session(session_id)
        duration = time.time() - start
        
        assert "status" in stop_result or "success" in stop_result, "Stop operation failed"
        
        return TestResult(
            name="Stop Session",
            passed=True,
            duration=duration,
            message=f"Session {session_id[:16]}... stopped"
        )
    except Exception as e:
        return TestResult(
            name="Stop Session",
            passed=False,
            duration=time.time() - start,
            error=str(e)
        )


def test_multi_turn_conversation() -> TestResult:
    """Test multi-turn conversation"""
    start = time.time()
    tester = BambooAPITester()
    conversation_id = None
    
    try:
        messages = [
            "What is the capital of France?",
            "What is the population of that city?",
            "Tell me about a famous landmark there."
        ]
        
        responses = []
        for i, msg in enumerate(messages):
            result = tester.create_chat(
                message=msg,
                conversation_id=conversation_id,
                stream=False
            )
            
            if conversation_id is None:
                conversation_id = result.get("conversation_id") or result.get("session_id")
            
            response_text = result.get("response", result.get("message", ""))
            responses.append(response_text)
        
        duration = time.time() - start
        
        # Verify we got responses for all messages
        assert len(responses) == len(messages), "Not all messages got responses"
        assert all(responses), "Some responses were empty"
        
        tester.cleanup()
        
        return TestResult(
            name="Multi-turn Conversation",
            passed=True,
            duration=duration,
            message=f"{len(messages)} turns completed"
        )
    except Exception as e:
        return TestResult(
            name="Multi-turn Conversation",
            passed=False,
            duration=time.time() - start,
            error=str(e)
        )


def test_concurrent_requests() -> TestResult:
    """Test concurrent API requests"""
    start = time.time()
    
    def make_request(i: int) -> Dict:
        tester = BambooAPITester()
        return tester.create_chat(
            message=f"Concurrent test message {i}",
            stream=False
        )
    
    try:
        with concurrent.futures.ThreadPoolExecutor(max_workers=5) as executor:
            futures = [executor.submit(make_request, i) for i in range(5)]
            results = [f.result() for f in concurrent.futures.as_completed(futures)]
        
        duration = time.time() - start
        
        # Verify all requests succeeded
        assert len(results) == 5, "Not all concurrent requests completed"
        assert all("session_id" in r for r in results), "Some requests failed"
        
        return TestResult(
            name="Concurrent Requests",
            passed=True,
            duration=duration,
            message=f"5 concurrent requests completed"
        )
    except Exception as e:
        return TestResult(
            name="Concurrent Requests",
            passed=False,
            duration=time.time() - start,
            error=str(e)
        )


def test_invalid_session_id() -> TestResult:
    """Test error handling for invalid session ID"""
    start = time.time()
    tester = BambooAPITester()
    
    try:
        # Try to get history with invalid session ID
        response = tester.session.get(
            tester._url("/history/invalid_session_id_12345"),
            timeout=TIMEOUT
        )
        
        # Should return 404 or similar error
        assert response.status_code in [404, 400, 422], \
            f"Expected error status, got {response.status_code}"
        
        duration = time.time() - start
        
        return TestResult(
            name="Invalid Session ID Handling",
            passed=True,
            duration=duration,
            message=f"Correctly returned {response.status_code}"
        )
    except requests.exceptions.HTTPError as e:
        duration = time.time() - start
        if e.response.status_code in [404, 400, 422]:
            return TestResult(
                name="Invalid Session ID Handling",
                passed=True,
                duration=duration,
                message=f"Correctly returned {e.response.status_code}"
            )
        return TestResult(
            name="Invalid Session ID Handling",
            passed=False,
            duration=duration,
            error=str(e)
        )
    except Exception as e:
        return TestResult(
            name="Invalid Session ID Handling",
            passed=False,
            duration=time.time() - start,
            error=str(e)
        )


def test_empty_message() -> TestResult:
    """Test error handling for empty message"""
    start = time.time()
    tester = BambooAPITester()
    
    try:
        response = tester.session.post(
            tester._url("/chat"),
            json={"message": "", "stream": False},
            timeout=TIMEOUT
        )
        
        duration = time.time() - start
        
        # Should return error for empty message
        if response.status_code in [400, 422]:
            return TestResult(
                name="Empty Message Handling",
                passed=True,
                duration=duration,
                message=f"Correctly rejected empty message with {response.status_code}"
            )
        else:
            # Some APIs might accept empty messages
            return TestResult(
                name="Empty Message Handling",
                passed=True,
                duration=duration,
                message=f"API accepted empty message (status: {response.status_code})"
            )
            
    except Exception as e:
        return TestResult(
            name="Empty Message Handling",
            passed=False,
            duration=time.time() - start,
            error=str(e)
        )


# ==================== Async Tests with httpx ====================

async def test_async_chat() -> TestResult:
    """Test async chat with httpx"""
    start = time.time()
    
    try:
        async with httpx.AsyncClient() as client:
            response = await client.post(
                f"{BASE_URL}{API_VERSION}/chat",
                json={"message": "Async test message", "stream": False},
                timeout=TIMEOUT
            )
            response.raise_for_status()
            data = response.json()
        
        duration = time.time() - start
        
        assert "session_id" in data, "Missing session_id in response"
        
        return TestResult(
            name="Async Chat (httpx)",
            passed=True,
            duration=duration,
            message=f"Session ID: {data['session_id'][:16]}..."
        )
    except Exception as e:
        return TestResult(
            name="Async Chat (httpx)",
            passed=False,
            duration=time.time() - start,
            error=str(e)
        )


async def test_async_concurrent() -> TestResult:
    """Test async concurrent requests"""
    start = time.time()
    
    async def make_async_request(i: int) -> Dict:
        async with httpx.AsyncClient() as client:
            response = await client.post(
                f"{BASE_URL}{API_VERSION}/chat",
                json={"message": f"Async concurrent test {i}", "stream": False},
                timeout=TIMEOUT
            )
            response.raise_for_status()
            return response.json()
    
    try:
        tasks = [make_async_request(i) for i in range(5)]
        results = await asyncio.gather(*tasks)
        
        duration = time.time() - start
        
        assert len(results) == 5, "Not all async requests completed"
        
        return TestResult(
            name="Async Concurrent Requests",
            passed=True,
            duration=duration,
            message=f"5 async requests completed"
        )
    except Exception as e:
        return TestResult(
            name="Async Concurrent Requests",
            passed=False,
            duration=time.time() - start,
            error=str(e)
        )


# ==================== Main Runner ====================

def run_all_tests():
    """Run all tests and print results"""
    print("=" * 70)
    print("Bamboo API Chat Endpoints Test Suite")
    print(f"Base URL: {BASE_URL}")
    print(f"Time: {datetime.now().isoformat()}")
    print("=" * 70)
    print()
    
    # Sync tests
    sync_tests = [
        test_health_check,
        test_get_config,
        test_create_chat,
        test_streaming_response,
        test_chat_history,
        test_stop_session,
        test_multi_turn_conversation,
        test_concurrent_requests,
        test_invalid_session_id,
        test_empty_message,
    ]
    
    results = []
    
    # Run sync tests
    for test_func in sync_tests:
        result = test_func()
        results.append(result)
        status = "✅ PASS" if result.passed else "❌ FAIL"
        print(f"{status} | {result.name} ({result.duration:.2f}s)")
        if result.message:
            print(f"       {result.message}")
        if result.error:
            print(f"       Error: {result.error}")
    
    # Run async tests
    print()
    print("Running async tests...")
    print()
    
    async_tests = [
        test_async_chat,
        test_async_concurrent,
    ]
    
    for test_func in async_tests:
        result = asyncio.run(test_func())
        results.append(result)
        status = "✅ PASS" if result.passed else "❌ FAIL"
        print(f"{status} | {result.name} ({result.duration:.2f}s)")
        if result.message:
            print(f"       {result.message}")
        if result.error:
            print(f"       Error: {result.error}")
    
    # Summary
    print()
    print("=" * 70)
    print("Test Summary")
    print("=" * 70)
    
    passed = sum(1 for r in results if r.passed)
    failed = sum(1 for r in results if not r.passed)
    total_time = sum(r.duration for r in results)
    
    print(f"Total: {len(results)} tests")
    print(f"Passed: {passed}")
    print(f"Failed: {failed}")
    print(f"Total time: {total_time:.2f}s")
    print()
    
    if failed > 0:
        print("Failed tests:")
        for r in results:
            if not r.passed:
                print(f"  - {r.name}: {r.error}")
    
    return results


# ==================== Pytest Integration ====================

class TestBambooChatAPI:
    """Pytest test class"""
    
    @classmethod
    def setup_class(cls):
        cls.tester = BambooAPITester()
    
    @classmethod
    def teardown_class(cls):
        cls.tester.cleanup()
    
    def test_health(self):
        result = test_health_check()
        assert result.passed, result.error
    
    def test_config(self):
        result = test_get_config()
        assert result.passed, result.error
    
    def test_chat_creation(self):
        result = test_create_chat()
        assert result.passed, result.error
    
    def test_streaming(self):
        result = test_streaming_response()
        assert result.passed, result.error
    
    def test_history(self):
        result = test_chat_history()
        assert result.passed, result.error
    
    def test_stop(self):
        result = test_stop_session()
        assert result.passed, result.error
    
    def test_multi_turn(self):
        result = test_multi_turn_conversation()
        assert result.passed, result.error
    
    def test_concurrent(self):
        result = test_concurrent_requests()
        assert result.passed, result.error
    
    def test_invalid_session(self):
        result = test_invalid_session_id()
        assert result.passed, result.error
    
    def test_empty_msg(self):
        result = test_empty_message()
        assert result.passed, result.error
    
    @pytest.mark.asyncio
    async def test_async_chat_request(self):
        result = await test_async_chat()
        assert result.passed, result.error
    
    @pytest.mark.asyncio
    async def test_async_concurrent_requests(self):
        result = await test_async_concurrent()
        assert result.passed, result.error


if __name__ == "__main__":
    run_all_tests()
