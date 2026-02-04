"""
Bamboo API 测试共享 Fixtures
==============================
此文件包含 pytest 共享 fixtures，自动被所有测试文件使用。
"""

import os
import pytest
import pytest_asyncio
import asyncio
import aiohttp
import websockets
import json
import time
import uuid
import psutil
from pathlib import Path
from typing import Generator, Dict, Any, List, Optional
from dataclasses import dataclass


# 项目根目录
PROJECT_ROOT = Path(__file__).parent.parent.parent
TESTS_DIR = Path(__file__).parent
FIXTURES_DIR = TESTS_DIR / "fixtures"


# =============================================================================
# 配置类
# =============================================================================

@dataclass
class TestConfig:
    """测试配置"""
    base_url: str = "http://127.0.0.1:8080"
    ws_url: str = "ws://127.0.0.1:18790"
    api_prefix: str = "/api/v1"
    timeout: int = 30
    
    @property
    def chat_url(self) -> str:
        return f"{self.base_url}{self.api_prefix}/chat"
    
    @property
    def health_url(self) -> str:
        return f"{self.base_url}{self.api_prefix}/health"


def load_fixture(filename: str) -> Dict[str, Any]:
    """加载 JSON fixture 文件"""
    import json
    fixture_path = FIXTURES_DIR / filename
    if not fixture_path.exists():
        raise FileNotFoundError(f"Fixture not found: {fixture_path}")
    with open(fixture_path, 'r', encoding='utf-8') as f:
        return json.load(f)


# =============================================================================
# 辅助函数
# =============================================================================

async def create_session_via_http(
    http_session: aiohttp.ClientSession,
    config: TestConfig,
    message: str = "Hello, this is a test message"
) -> Dict[str, Any]:
    """通过 HTTP 创建会话"""
    payload = {
        "message": message,
        "model": "gpt-4"
    }
    
    async with http_session.post(
        config.chat_url,
        json=payload,
        timeout=aiohttp.ClientTimeout(total=config.timeout)
    ) as response:
        assert response.status == 201
        return await response.json()


async def connect_ws(
    ws: websockets.WebSocketClientProtocol,
    session_id: str
) -> Dict[str, Any]:
    """通过 WebSocket 连接会话"""
    connect_msg = {
        "type": "Connect",
        "session_id": session_id
    }
    await ws.send(json.dumps(connect_msg))
    
    # 等待 Connected 响应
    response = await asyncio.wait_for(ws.recv(), timeout=10)
    return json.loads(response)


async def send_chat_via_ws(
    ws: websockets.WebSocketClientProtocol,
    session_id: str,
    content: str
) -> None:
    """通过 WebSocket 发送聊天消息"""
    chat_msg = {
        "type": "Chat",
        "session_id": session_id,
        "content": content
    }
    await ws.send(json.dumps(chat_msg))


async def read_sse_stream(
    http_session: aiohttp.ClientSession,
    stream_url: str,
    timeout: float = 30.0
) -> List[Dict[str, Any]]:
    """读取 SSE 流并返回所有事件"""
    events = []
    start_time = time.time()
    
    async with http_session.get(stream_url) as response:
        assert response.status == 200
        
        async for line in response.content:
            if time.time() - start_time > timeout:
                break
                
            line = line.decode('utf-8').strip()
            if line.startswith('data: '):
                data = line[6:]  # 去掉 'data: ' 前缀
                try:
                    event = json.loads(data)
                    events.append(event)
                    
                    # 检查是否完成
                    if event.get('type') in ['Complete', 'Error']:
                        break
                except json.JSONDecodeError:
                    pass
    
    return events


async def get_session_history(
    http_session: aiohttp.ClientSession,
    config: TestConfig,
    session_id: str
) -> List[Dict[str, Any]]:
    """获取会话历史"""
    url = f"{config.base_url}{config.api_prefix}/history/{session_id}"
    
    async with http_session.get(url) as response:
        if response.status == 200:
            return await response.json()
        return []


async def stop_session(
    http_session: aiohttp.ClientSession,
    config: TestConfig,
    session_id: str
) -> bool:
    """停止会话"""
    url = f"{config.base_url}{config.api_prefix}/stop/{session_id}"
    
    async with http_session.post(url) as response:
        return response.status == 200


# =============================================================================
# 性能监控
# =============================================================================

class PerformanceMonitor:
    """性能监控器"""
    
    def __init__(self):
        self.process = psutil.Process()
        self.start_time: Optional[float] = None
        self.metrics: Dict[str, Any] = {}
    
    def start(self):
        """开始监控"""
        self.start_time = time.time()
        self.metrics['start_memory'] = self.process.memory_info().rss / 1024 / 1024  # MB
        self.metrics['start_cpu'] = self.process.cpu_percent()
    
    def stop(self):
        """停止监控并记录指标"""
        if self.start_time:
            self.metrics['duration'] = time.time() - self.start_time
        self.metrics['end_memory'] = self.process.memory_info().rss / 1024 / 1024  # MB
        self.metrics['memory_delta'] = self.metrics['end_memory'] - self.metrics['start_memory']
        self.metrics['end_cpu'] = self.process.cpu_percent()
    
    def record_latency(self, name: str, latency_ms: float):
        """记录延迟指标"""
        if 'latencies' not in self.metrics:
            self.metrics['latencies'] = {}
        if name not in self.metrics['latencies']:
            self.metrics['latencies'][name] = []
        self.metrics['latencies'][name].append(latency_ms)
    
    def get_summary(self) -> Dict[str, Any]:
        """获取性能摘要"""
        summary = {
            'duration_seconds': self.metrics.get('duration', 0),
            'memory_start_mb': self.metrics.get('start_memory', 0),
            'memory_end_mb': self.metrics.get('end_memory', 0),
            'memory_delta_mb': self.metrics.get('memory_delta', 0),
        }
        
        # 计算延迟统计
        if 'latencies' in self.metrics:
            for name, values in self.metrics['latencies'].items():
                if values:
                    summary[f'{name}_latency_ms'] = {
                        'min': min(values),
                        'max': max(values),
                        'avg': sum(values) / len(values),
                        'p95': sorted(values)[int(len(values) * 0.95)] if len(values) > 1 else values[0],
                        'count': len(values)
                    }
        
        return summary


# =============================================================================
# 测试数据
# =============================================================================

TEST_MESSAGES = [
    "Hello, how are you?",
    "What is the weather like today?",
    "Tell me a joke",
    "Explain quantum computing in simple terms",
    "Write a Python function to calculate fibonacci numbers",
    "What are the benefits of exercise?",
    "How do I make a good cup of coffee?",
    "Explain the theory of relativity",
    "What is machine learning?",
    "How does blockchain work?"
]


# ============================================
# 异步 Fixtures
# ============================================

@pytest_asyncio.fixture
async def config() -> TestConfig:
    """返回测试配置"""
    return TestConfig(
        base_url=os.getenv("BAMBOO_API_URL", "http://127.0.0.1:8080"),
        ws_url=os.getenv("BAMBOO_WS_URL", "ws://127.0.0.1:18790"),
        timeout=int(os.getenv("BAMBOO_API_TIMEOUT", "30"))
    )


@pytest_asyncio.fixture
async def http_session():
    """创建 HTTP session"""
    async with aiohttp.ClientSession() as session:
        yield session


# ============================================
# 基础 Fixtures
# ============================================

@pytest.fixture(scope="session")
def project_root() -> Path:
    """返回项目根目录路径"""
    return PROJECT_ROOT


@pytest.fixture(scope="session")
def tests_dir() -> Path:
    """返回测试目录路径"""
    return TESTS_DIR


@pytest.fixture(scope="session")
def fixtures_dir() -> Path:
    """返回 fixtures 目录路径"""
    return FIXTURES_DIR


# ============================================
# 环境配置 Fixtures
# ============================================

@pytest.fixture(scope="session")
def test_env() -> str:
    """返回当前测试环境"""
    return os.getenv("TEST_ENV", "development")


@pytest.fixture(scope="session")
def api_base_url() -> str:
    """返回 API 基础 URL"""
    return os.getenv("BAMBOO_API_URL", "http://localhost:8080")


@pytest.fixture(scope="session")
def api_timeout() -> int:
    """返回 API 超时时间（秒）"""
    return int(os.getenv("BAMBOO_API_TIMEOUT", "30"))


# ============================================
# HTTP 客户端 Fixtures
# ============================================

@pytest.fixture
def api_client(api_base_url: str, api_timeout: int):
    """
    创建 API 客户端
    
    这是一个示例 fixture，实际实现取决于 Bamboo API 客户端的具体实现。
    """
    import requests
    
    session = requests.Session()
    session.headers.update({
        "Content-Type": "application/json",
        "Accept": "application/json",
    })
    
    # 如果配置了 API Key，添加到请求头
    api_key = os.getenv("BAMBOO_API_KEY")
    if api_key:
        session.headers["Authorization"] = f"Bearer {api_key}"
    
    yield session
    
    # 清理
    session.close()


@pytest.fixture
def mock_api(responses):
    """
    提供 mock API 响应的 fixture
    
    需要安装 pytest-responses 或 responses 库
    """
    return responses


# ============================================
# 测试数据 Fixtures
# ============================================

@pytest.fixture
def sample_agent_config() -> Dict[str, Any]:
    """示例 Agent 配置"""
    return {
        "name": "test-agent",
        "description": "Test agent for API testing",
        "model": "gpt-4",
        "temperature": 0.7,
        "max_tokens": 2048,
    }


@pytest.fixture
def sample_skill_config() -> Dict[str, Any]:
    """示例 Skill 配置"""
    return {
        "name": "test-skill",
        "description": "Test skill for API testing",
        "parameters": {
            "type": "object",
            "properties": {
                "input": {"type": "string"}
            }
        },
    }


@pytest.fixture
def sample_conversation() -> Dict[str, Any]:
    """示例对话数据"""
    return {
        "messages": [
            {"role": "user", "content": "Hello"},
            {"role": "assistant", "content": "Hi there!"}
        ]
    }


# ============================================
# 条件跳过 Fixtures
# ============================================

@pytest.fixture(scope="session")
def integration_tests_enabled() -> bool:
    """检查是否启用集成测试"""
    return os.getenv("RUN_INTEGRATION_TESTS", "false").lower() == "true"


@pytest.fixture(scope="session")
def slow_tests_enabled() -> bool:
    """检查是否启用慢测试"""
    return os.getenv("RUN_SLOW_TESTS", "false").lower() == "true"


# ============================================
# pytest 配置钩子
# ============================================

def pytest_configure(config):
    """pytest 配置钩子"""
    # 添加自定义标记
    config.addinivalue_line(
        "markers", "requires_env(name): mark test to require specific environment variable"
    )


def pytest_runtest_setup(item):
    """测试运行前设置"""
    # 检查是否需要特定环境变量
    for marker in item.iter_markers("requires_env"):
        env_var = marker.args[0]
        if not os.getenv(env_var):
            pytest.skip(f"需要环境变量: {env_var}")
