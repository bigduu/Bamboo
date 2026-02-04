"""
Bamboo API 集成测试
测试 HTTP API 和 WebSocket 的协同工作、会话持久化、多客户端场景
"""

import pytest
import asyncio
import aiohttp
import websockets
import json
import time
import uuid
from typing import Dict, Any, List

from conftest import (
    TestConfig, create_session_via_http, connect_ws, send_chat_via_ws,
    read_sse_stream, get_session_history, stop_session, PerformanceMonitor
)


# =============================================================================
# 基础 HTTP API 测试
# =============================================================================

@pytest.mark.asyncio
class TestHttpApi:
    """HTTP API 基础测试"""
    
    async def test_health_endpoint(self, config: TestConfig, http_session):
        """测试健康检查端点"""
        async with http_session.get(config.health_url) as response:
            assert response.status == 200
            data = await response.json()
            assert 'status' in data or 'message' in data
    
    async def test_create_session(self, config: TestConfig, http_session):
        """测试创建会话"""
        result = await create_session_via_http(
            http_session, config, "Test message"
        )
        
        assert 'session_id' in result
        assert 'stream_url' in result
        assert 'status' in result
        assert result['status'] == 'streaming'
        assert result['stream_url'].startswith('/api/v1/stream/')
    
    async def test_create_session_with_custom_id(self, config: TestConfig, http_session):
        """测试使用自定义 session_id 创建会话"""
        custom_id = f"test-session-{uuid.uuid4().hex[:8]}"
        
        payload = {
            "message": "Test with custom ID",
            "session_id": custom_id,
            "model": "gpt-4"
        }
        
        async with http_session.post(config.chat_url, json=payload) as response:
            assert response.status == 201
            data = await response.json()
            assert data['session_id'] == custom_id
    
    async def test_stream_endpoint(self, config: TestConfig, http_session):
        """测试 SSE 流端点"""
        # 先创建会话
        result = await create_session_via_http(http_session, config, "Hello")
        session_id = result['session_id']
        
        # 读取流
        stream_url = f"{config.base_url}{result['stream_url']}"
        events = await read_sse_stream(http_session, stream_url, timeout=30.0)
        
        # 验证收到事件
        assert len(events) > 0
        
        # 验证事件类型
        event_types = [e.get('type') for e in events]
        assert 'Token' in event_types or 'Complete' in event_types
    
    async def test_history_endpoint(self, config: TestConfig, http_session):
        """测试历史记录端点"""
        # 创建会话
        result = await create_session_via_http(http_session, config, "Test history")
        session_id = result['session_id']
        
        # 等待流完成
        stream_url = f"{config.base_url}{result['stream_url']}"
        await read_sse_stream(http_session, stream_url, timeout=30.0)
        
        # 获取历史
        history = await get_session_history(http_session, config, session_id)
        
        # 验证历史记录
        assert isinstance(history, list)
        assert len(history) >= 1  # 至少包含用户消息
    
    async def test_stop_endpoint(self, config: TestConfig, http_session):
        """测试停止会话端点"""
        # 创建会话
        result = await create_session_via_http(http_session, config, "Long message...")
        session_id = result['session_id']
        
        # 停止会话
        success = await stop_session(http_session, config, session_id)
        assert success


# =============================================================================
# WebSocket API 测试
# =============================================================================

@pytest.mark.asyncio
class TestWebSocketApi:
    """WebSocket API 测试"""
    
    async def test_ws_connection(self, config: TestConfig):
        """测试 WebSocket 连接"""
        async with websockets.connect(config.ws_url) as ws:
            assert ws.open
    
    async def test_ws_connect_message(self, config: TestConfig):
        """测试 WebSocket Connect 消息"""
        session_id = f"ws-test-{uuid.uuid4().hex[:8]}"
        
        async with websockets.connect(config.ws_url) as ws:
            # 发送 Connect
            connect_msg = {
                "type": "Connect",
                "session_id": session_id
            }
            await ws.send(json.dumps(connect_msg))
            
            # 等待响应
            response = await asyncio.wait_for(ws.recv(), timeout=10)
            data = json.loads(response)
            
            assert data.get('type') == 'Connected'
            assert data.get('session_id') == session_id
    
    async def test_ws_chat_message(self, config: TestConfig):
        """测试 WebSocket Chat 消息"""
        session_id = f"ws-chat-{uuid.uuid4().hex[:8]}"
        
        async with websockets.connect(config.ws_url) as ws:
            # 连接
            await connect_ws(ws, session_id)
            
            # 发送聊天消息
            await send_chat_via_ws(ws, session_id, "Hello from WebSocket")
            
            # 等待响应（可能会有多个 Token 事件）
            events = []
            start_time = time.time()
            
            while time.time() - start_time < 30:
                try:
                    response = await asyncio.wait_for(ws.recv(), timeout=5)
                    data = json.loads(response)
                    events.append(data)
                    
                    # 检查是否完成
                    if data.get('type') in ['Complete', 'Error']:
                        break
                except asyncio.TimeoutError:
                    break
            
            # 验证收到响应
            assert len(events) > 0
            event_types = [e.get('type') for e in events]
            assert any(t in event_types for t in ['AgentToken', 'Complete', 'Error'])
    
    async def test_ws_without_connect(self, config: TestConfig):
        """测试未连接直接发送 Chat 的错误处理"""
        async with websockets.connect(config.ws_url) as ws:
            # 直接发送 Chat，不先 Connect
            chat_msg = {
                "type": "Chat",
                "session_id": "test",
                "content": "Hello"
            }
            await ws.send(json.dumps(chat_msg))
            
            # 等待错误响应
            response = await asyncio.wait_for(ws.recv(), timeout=5)
            data = json.loads(response)
            
            assert data.get('type') == 'Error'
            assert 'NOT_CONNECTED' in data.get('code', '')


# =============================================================================
# HTTP + WebSocket 协同测试
# =============================================================================

@pytest.mark.asyncio
class TestHttpWebSocketIntegration:
    """HTTP 和 WebSocket 协同工作测试"""
    
    async def test_create_via_http_chat_via_ws(self, config: TestConfig, http_session):
        """测试通过 HTTP 创建会话，通过 WebSocket 聊天"""
        # 通过 HTTP 创建会话
        result = await create_session_via_http(http_session, config, "Initial message")
        session_id = result['session_id']
        
        # 通过 WebSocket 连接并聊天
        async with websockets.connect(config.ws_url) as ws:
            # 连接会话
            connected = await connect_ws(ws, session_id)
            assert connected.get('type') == 'Connected'
            
            # 发送消息
            await send_chat_via_ws(ws, session_id, "Follow up via WebSocket")
            
            # 接收响应
            events = []
            start_time = time.time()
            
            while time.time() - start_time < 30:
                try:
                    response = await asyncio.wait_for(ws.recv(), timeout=5)
                    data = json.loads(response)
                    events.append(data)
                    
                    if data.get('type') in ['Complete', 'Error']:
                        break
                except asyncio.TimeoutError:
                    break
            
            assert len(events) > 0
    
    async def test_parallel_http_ws_usage(self, config: TestConfig, http_session):
        """测试同时使用 HTTP 和 WebSocket"""
        results = {'http': None, 'ws': None}
        
        async def http_task():
            """HTTP 任务"""
            result = await create_session_via_http(
                http_session, config, "HTTP parallel test"
            )
            session_id = result['session_id']
            
            # 读取流
            stream_url = f"{config.base_url}{result['stream_url']}"
            events = await read_sse_stream(http_session, stream_url, timeout=30.0)
            
            results['http'] = {
                'session_id': session_id,
                'event_count': len(events)
            }
        
        async def ws_task():
            """WebSocket 任务"""
            session_id = f"parallel-ws-{uuid.uuid4().hex[:8]}"
            
            async with websockets.connect(config.ws_url) as ws:
                await connect_ws(ws, session_id)
                await send_chat_via_ws(ws, session_id, "WS parallel test")
                
                events = []
                start_time = time.time()
                
                while time.time() - start_time < 30:
                    try:
                        response = await asyncio.wait_for(ws.recv(), timeout=5)
                        data = json.loads(response)
                        events.append(data)
                        
                        if data.get('type') in ['Complete', 'Error']:
                            break
                    except asyncio.TimeoutError:
                        break
                
                results['ws'] = {
                    'session_id': session_id,
                    'event_count': len(events)
                }
        
        # 并行执行
        await asyncio.gather(http_task(), ws_task())
        
        # 验证结果
        assert results['http'] is not None
        assert results['ws'] is not None
        assert results['http']['event_count'] > 0
        assert results['ws']['event_count'] > 0


# =============================================================================
# 会话持久化测试
# =============================================================================

@pytest.mark.asyncio
class TestSessionPersistence:
    """会话持久化测试"""
    
    async def test_session_persists_after_disconnect(self, config: TestConfig):
        """测试断开后会话仍然保持"""
        session_id = f"persist-{uuid.uuid4().hex[:8]}"
        
        # 第一次连接
        async with websockets.connect(config.ws_url) as ws:
            await connect_ws(ws, session_id)
            await send_chat_via_ws(ws, session_id, "First message")
            
            # 接收一些响应
            start_time = time.time()
            while time.time() - start_time < 10:
                try:
                    response = await asyncio.wait_for(ws.recv(), timeout=2)
                    data = json.loads(response)
                    if data.get('type') in ['Complete', 'Error']:
                        break
                except asyncio.TimeoutError:
                    break
        
        # 断开连接后重新连接
        async with websockets.connect(config.ws_url) as ws:
            await connect_ws(ws, session_id)
            
            # 验证可以发送新消息
            await send_chat_via_ws(ws, session_id, "Second message after reconnect")
            
            # 接收响应
            response = await asyncio.wait_for(ws.recv(), timeout=10)
            data = json.loads(response)
            
            # 应该能收到响应
            assert data.get('type') is not None
    
    async def test_http_session_persistence(self, config: TestConfig, http_session):
        """测试 HTTP 会话持久化"""
        # 创建会话
        result = await create_session_via_http(
            http_session, config, "Persistence test"
        )
        session_id = result['session_id']
        
        # 等待流完成
        stream_url = f"{config.base_url}{result['stream_url']}"
        await read_sse_stream(http_session, stream_url, timeout=30.0)
        
        # 获取历史，验证会话存在
        history = await get_session_history(http_session, config, session_id)
        assert len(history) >= 1
        
        # 再次获取历史，验证会话仍然保持
        history2 = await get_session_history(http_session, config, session_id)
        assert len(history2) == len(history)


# =============================================================================
# 多客户端场景测试
# =============================================================================

@pytest.mark.asyncio
class TestMultiClient:
    """多客户端场景测试"""
    
    async def test_multiple_ws_clients_same_session(self, config: TestConfig):
        """测试多个 WebSocket 客户端连接到同一会话"""
        session_id = f"multi-ws-{uuid.uuid4().hex[:8]}"
        
        async def client_task(client_id: int):
            """客户端任务"""
            async with websockets.connect(config.ws_url) as ws:
                # 连接同一会话
                await connect_ws(ws, session_id)
                
                # 发送消息
                await send_chat_via_ws(ws, session_id, f"Message from client {client_id}")
                
                # 接收响应
                events = []
                start_time = time.time()
                
                while time.time() - start_time < 15:
                    try:
                        response = await asyncio.wait_for(ws.recv(), timeout=3)
                        data = json.loads(response)
                        events.append(data)
                        
                        if data.get('type') in ['Complete', 'Error']:
                            break
                    except asyncio.TimeoutError:
                        break
                
                return {'client_id': client_id, 'events': events}
        
        # 启动多个客户端
        clients = [client_task(i) for i in range(3)]
        results = await asyncio.gather(*clients)
        
        # 验证所有客户端都收到了响应
        for result in results:
            assert len(result['events']) > 0
    
    async def test_concurrent_http_requests(self, config: TestConfig):
        """测试并发 HTTP 请求"""
        async def request_task(task_id: int):
            """请求任务"""
            async with aiohttp.ClientSession() as session:
                result = await create_session_via_http(
                    session, config, f"Concurrent request {task_id}"
                )
                
                # 读取流
                stream_url = f"{config.base_url}{result['stream_url']}"
                events = await read_sse_stream(session, stream_url, timeout=30.0)
                
                return {
                    'task_id': task_id,
                    'session_id': result['session_id'],
                    'event_count': len(events)
                }
        
        # 并发发送 5 个请求
        tasks = [request_task(i) for i in range(5)]
        results = await asyncio.gather(*tasks)
        
        # 验证所有请求都成功
        for result in results:
            assert result['event_count'] > 0
            assert result['session_id'] is not None
        
        # 验证会话 ID 不同
        session_ids = [r['session_id'] for r in results]
        assert len(set(session_ids)) == 5  # 所有会话 ID 应该不同


# =============================================================================
# 端到端测试
# =============================================================================

@pytest.mark.asyncio
class TestEndToEnd:
    """端到端测试 - 完整对话流程"""
    
    async def test_complete_conversation_flow_http(self, config: TestConfig, http_session):
        """测试完整的 HTTP 对话流程：创建 → 聊天 → 历史 → 停止"""
        monitor = PerformanceMonitor()
        monitor.start()
        
        # 1. 创建会话
        create_start = time.time()
        result = await create_session_via_http(
            http_session, config, "Start conversation"
        )
        create_latency = (time.time() - create_start) * 1000
        monitor.record_latency('create_session', create_latency)
        
        session_id = result['session_id']
        
        # 2. 读取流（聊天）
        stream_start = time.time()
        stream_url = f"{config.base_url}{result['stream_url']}"
        events = await read_sse_stream(http_session, stream_url, timeout=30.0)
        stream_latency = (time.time() - stream_start) * 1000
        monitor.record_latency('first_chat_stream', stream_latency)
        
        assert len(events) > 0
        
        # 3. 获取历史
        history_start = time.time()
        history = await get_session_history(http_session, config, session_id)
        history_latency = (time.time() - history_start) * 1000
        monitor.record_latency('get_history', history_latency)
        
        assert len(history) >= 1
        
        # 4. 停止会话
        stop_start = time.time()
        success = await stop_session(http_session, config, session_id)
        stop_latency = (time.time() - stop_start) * 1000
        monitor.record_latency('stop_session', stop_latency)
        
        assert success
        
        monitor.stop()
        
        # 打印性能摘要
        summary = monitor.get_summary()
        print(f"\nEnd-to-End Performance Summary:")
        print(f"  Total Duration: {summary['duration_seconds']:.2f}s")
        print(f"  Memory Delta: {summary['memory_delta_mb']:.2f} MB")
        for key, value in summary.items():
            if 'latency' in key:
                print(f"  {key}: {value}")
    
    async def test_complete_conversation_flow_ws(self, config: TestConfig):
        """测试完整的 WebSocket 对话流程"""
        session_id = f"e2e-ws-{uuid.uuid4().hex[:8]}"
        
        async with websockets.connect(config.ws_url) as ws:
            # 1. 连接
            connected = await connect_ws(ws, session_id)
            assert connected.get('type') == 'Connected'
            
            # 2. 多轮对话
            messages = [
                "Hello, how are you?",
                "What's the weather like?",
                "Tell me a joke"
            ]
            
            for msg in messages:
                # 发送消息
                await send_chat_via_ws(ws, session_id, msg)
                
                # 接收响应
                events = []
                start_time = time.time()
                
                while time.time() - start_time < 30:
                    try:
                        response = await asyncio.wait_for(ws.recv(), timeout=5)
                        data = json.loads(response)
                        events.append(data)
                        
                        if data.get('type') in ['Complete', 'Error']:
                            break
                    except asyncio.TimeoutError:
                        break
                
                # 验证收到响应
                assert len(events) > 0
                
                # 短暂等待，避免请求过快
                await asyncio.sleep(0.5)


# =============================================================================
# 错误恢复测试
# =============================================================================

@pytest.mark.asyncio
class TestErrorRecovery:
    """错误恢复场景测试"""
    
    async def test_stream_not_found(self, config: TestConfig, http_session):
        """测试访问不存在的流"""
        fake_session_id = "non-existent-session-12345"
        stream_url = f"{config.base_url}{config.api_prefix}/stream/{fake_session_id}"
        
        async with http_session.get(stream_url) as response:
            # 应该返回 404
            assert response.status == 404
    
    async def test_invalid_json_payload(self, config: TestConfig, http_session):
        """测试发送无效的 JSON"""
        async with http_session.post(
            config.chat_url,
            data="invalid json {",
            headers={"Content-Type": "application/json"}
        ) as response:
            # 应该返回 400
            assert response.status == 400
    
    async def test_ws_reconnect_after_disconnect(self, config: TestConfig):
        """测试断线重连"""
        session_id = f"reconnect-{uuid.uuid4().hex[:8]}"
        
        # 第一次连接并发送消息
        async with websockets.connect(config.ws_url) as ws:
            await connect_ws(ws, session_id)
            await send_chat_via_ws(ws, session_id, "First message")
            
            # 接收一些响应
            start_time = time.time()
            while time.time() - start_time < 5:
                try:
                    response = await asyncio.wait_for(ws.recv(), timeout=2)
                    data = json.loads(response)
                    if data.get('type') in ['Complete', 'Error']:
                        break
                except asyncio.TimeoutError:
                    break
        
        # 模拟断线后重连
        async with websockets.connect(config.ws_url) as ws:
            await connect_ws(ws, session_id)
            
            # 发送新消息，验证会话恢复
            await send_chat_via_ws(ws, session_id, "After reconnect")
            
            response = await asyncio.wait_for(ws.recv(), timeout=10)
            data = json.loads(response)
            
            # 应该能正常收到响应
            assert data.get('type') is not None
    
    async def test_graceful_degradation_under_load(self, config: TestConfig):
        """测试负载下的优雅降级"""
        async def stress_client(client_id: int):
            """压力测试客户端"""
            try:
                async with websockets.connect(
                    config.ws_url,
                    timeout=5
                ) as ws:
                    session_id = f"stress-{client_id}-{uuid.uuid4().hex[:8]}"
                    await connect_ws(ws, session_id)
                    
                    # 快速发送多条消息
                    for i in range(5):
                        await send_chat_via_ws(ws, session_id, f"Stress message {i}")
                        await asyncio.sleep(0.1)
                    
                    # 尝试接收响应
                    events = []
                    start_time = time.time()
                    
                    while time.time() - start_time < 10:
                        try:
                            response = await asyncio.wait_for(ws.recv(), timeout=2)
                            data = json.loads(response)
                            events.append(data)
                            
                            if data.get('type') in ['Complete', 'Error']:
                                break
                        except asyncio.TimeoutError:
                            break
                    
                    return {'client_id': client_id, 'success': True, 'events': len(events)}
            except Exception as e:
                return {'client_id': client_id, 'success': False, 'error': str(e)}
        
        # 启动多个并发客户端
        clients = [stress_client(i) for i in range(10)]
        results = await asyncio.gather(*clients)
        
        # 统计结果
        successful = sum(1 for r in results if r['success'])
        failed = len(results) - successful
        
        print(f"\nStress Test Results:")
        print(f"  Total: {len(results)}")
        print(f"  Successful: {successful}")
        print(f"  Failed: {failed}")
        
        # 至少 50% 应该成功（允许部分失败）
        assert successful >= len(results) * 0.5


# =============================================================================
# 主函数（用于直接运行）
# =============================================================================

if __name__ == "__main__":
    pytest.main([__file__, "-v", "--tb=short"])
