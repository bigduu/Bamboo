"""
Bamboo API 性能测试
使用 pytest + locust 进行并发、延迟、吞吐量测试
"""

import pytest
import asyncio
import aiohttp
import websockets
import json
import time
import uuid
import statistics
from typing import Dict, Any, List, Optional
from dataclasses import dataclass, field
from datetime import datetime
import psutil
import os
from concurrent.futures import ThreadPoolExecutor

from conftest import (
    TestConfig, create_session_via_http, connect_ws, send_chat_via_ws,
    read_sse_stream, PerformanceMonitor, TEST_MESSAGES
)


# =============================================================================
# 性能测试数据收集器
# =============================================================================

@dataclass
class PerformanceResult:
    """性能测试结果"""
    test_name: str
    concurrency: int
    total_requests: int
    successful_requests: int
    failed_requests: int
    latencies_ms: List[float] = field(default_factory=list)
    first_token_latencies_ms: List[float] = field(default_factory=list)
    total_durations_ms: List[float] = field(default_factory=list)
    start_time: Optional[float] = None
    end_time: Optional[float] = None
    memory_start_mb: float = 0.0
    memory_end_mb: float = 0.0
    
    @property
    def duration_seconds(self) -> float:
        if self.start_time and self.end_time:
            return self.end_time - self.start_time
        return 0.0
    
    @property
    def throughput_rps(self) -> float:
        if self.duration_seconds > 0:
            return self.successful_requests / self.duration_seconds
        return 0.0
    
    @property
    def latency_stats(self) -> Dict[str, float]:
        if not self.latencies_ms:
            return {}
        sorted_latencies = sorted(self.latencies_ms)
        return {
            'min_ms': min(self.latencies_ms),
            'max_ms': max(self.latencies_ms),
            'avg_ms': statistics.mean(self.latencies_ms),
            'median_ms': statistics.median(self.latencies_ms),
            'p50_ms': sorted_latencies[int(len(sorted_latencies) * 0.50)],
            'p95_ms': sorted_latencies[int(len(sorted_latencies) * 0.95)],
            'p99_ms': sorted_latencies[int(len(sorted_latencies) * 0.99)],
            'std_dev': statistics.stdev(self.latencies_ms) if len(self.latencies_ms) > 1 else 0
        }
    
    @property
    def first_token_stats(self) -> Dict[str, float]:
        if not self.first_token_latencies_ms:
            return {}
        return {
            'min_ms': min(self.first_token_latencies_ms),
            'max_ms': max(self.first_token_latencies_ms),
            'avg_ms': statistics.mean(self.first_token_latencies_ms),
            'p95_ms': sorted(self.first_token_latencies_ms)[int(len(self.first_token_latencies_ms) * 0.95)]
        }
    
    def print_summary(self):
        """打印性能摘要"""
        print(f"\n{'='*60}")
        print(f"性能测试结果: {self.test_name}")
        print(f"{'='*60}")
        print(f"并发数: {self.concurrency}")
        print(f"总请求数: {self.total_requests}")
        print(f"成功请求: {self.successful_requests}")
        print(f"失败请求: {self.failed_requests}")
        print(f"成功率: {self.successful_requests/self.total_requests*100:.2f}%")
        print(f"总耗时: {self.duration_seconds:.2f}s")
        print(f"吞吐量: {self.throughput_rps:.2f} req/s")
        
        if self.latency_stats:
            print(f"\n延迟统计 (ms):")
            for key, value in self.latency_stats.items():
                print(f"  {key}: {value:.2f}")
        
        if self.first_token_stats:
            print(f"\n首 Token 延迟统计 (ms):")
            for key, value in self.first_token_stats.items():
                print(f"  {key}: {value:.2f}")
        
        print(f"\n内存使用:")
        print(f"  开始: {self.memory_start_mb:.2f} MB")
        print(f"  结束: {self.memory_end_mb:.2f} MB")
        print(f"  增量: {self.memory_end_mb - self.memory_start_mb:.2f} MB")
        print(f"{'='*60}\n")


# =============================================================================
# 并发聊天请求测试
# =============================================================================

@pytest.mark.asyncio
class TestConcurrentChat:
    """并发聊天请求性能测试"""
    
    async def _run_concurrent_chat_test(
        self,
        config: TestConfig,
        concurrency: int,
        total_requests: int,
        test_name: str
    ) -> PerformanceResult:
        """运行并发聊天测试"""
        result = PerformanceResult(
            test_name=test_name,
            concurrency=concurrency,
            total_requests=total_requests,
            successful_requests=0,
            failed_requests=0
        )
        
        # 记录内存使用
        process = psutil.Process(os.getpid())
        result.memory_start_mb = process.memory_info().rss / 1024 / 1024
        
        # 信号量控制并发
        semaphore = asyncio.Semaphore(concurrency)
        completed_count = 0
        
        async def single_chat_request(request_id: int):
            """单个聊天请求"""
            nonlocal completed_count
            
            async with semaphore:
                start_time = time.time()
                first_token_time = None
                
                try:
                    async with aiohttp.ClientSession() as session:
                        # 创建会话
                        chat_result = await create_session_via_http(
                            session, config, TEST_MESSAGES[request_id % len(TEST_MESSAGES)]
                        )
                        
                        # 读取流
                        stream_url = f"{config.base_url}{chat_result['stream_url']}"
                        
                        async with session.get(stream_url) as response:
                            if response.status != 200:
                                return {'success': False, 'error': f'HTTP {response.status}'}
                            
                            async for line in response.content:
                                line = line.decode('utf-8').strip()
                                if line.startswith('data: '):
                                    if first_token_time is None:
                                        first_token_time = time.time()
                                    
                                    data = line[6:]
                                    try:
                                        event = json.loads(data)
                                        if event.get('type') in ['Complete', 'Error']:
                                            break
                                    except json.JSONDecodeError:
                                        pass
                        
                        end_time = time.time()
                        completed_count += 1
                        
                        return {
                            'success': True,
                            'total_latency_ms': (end_time - start_time) * 1000,
                            'first_token_latency_ms': (first_token_time - start_time) * 1000 if first_token_time else None
                        }
                        
                except Exception as e:
                    return {'success': False, 'error': str(e)}
        
        # 记录开始时间
        result.start_time = time.time()
        
        # 创建所有任务
        tasks = [single_chat_request(i) for i in range(total_requests)]
        
        # 等待所有任务完成
        task_results = await asyncio.gather(*tasks, return_exceptions=True)
        
        # 记录结束时间
        result.end_time = time.time()
        result.memory_end_mb = process.memory_info().rss / 1024 / 1024
        
        # 统计结果
        for r in task_results:
            if isinstance(r, Exception):
                result.failed_requests += 1
            elif r.get('success'):
                result.successful_requests += 1
                result.latencies_ms.append(r['total_latency_ms'])
                if r.get('first_token_latency_ms'):
                    result.first_token_latencies_ms.append(r['first_token_latency_ms'])
            else:
                result.failed_requests += 1
        
        return result
    
    async def test_concurrent_10(self, config: TestConfig):
        """测试 10 并发"""
        result = await self._run_concurrent_chat_test(
            config, concurrency=10, total_requests=30, test_name="10 Concurrent Chat"
        )
        result.print_summary()
        
        # 断言：成功率 > 90%
        assert result.successful_requests / result.total_requests >= 0.9
        # 断言：平均延迟 < 30s
        if result.latency_stats:
            assert result.latency_stats['avg_ms'] < 30000
    
    async def test_concurrent_50(self, config: TestConfig):
        """测试 50 并发"""
        result = await self._run_concurrent_chat_test(
            config, concurrency=50, total_requests=100, test_name="50 Concurrent Chat"
        )
        result.print_summary()
        
        # 断言：成功率 > 80%
        assert result.successful_requests / result.total_requests >= 0.8
    
    async def test_concurrent_100(self, config: TestConfig):
        """测试 100 并发"""
        result = await self._run_concurrent_chat_test(
            config, concurrency=100, total_requests=200, test_name="100 Concurrent Chat"
        )
        result.print_summary()
        
        # 断言：成功率 > 70%
        assert result.successful_requests / result.total_requests >= 0.7


# =============================================================================
# 延迟测量测试
# =============================================================================

@pytest.mark.asyncio
class TestLatencyMeasurement:
    """延迟测量测试"""
    
    async def test_first_token_latency(self, config: TestConfig):
        """测试首 token 延迟"""
        latencies = []
        
        for i in range(10):
            async with aiohttp.ClientSession() as session:
                start_time = time.time()
                first_token_time = None
                
                # 创建会话
                result = await create_session_via_http(
                    session, config, "Measure first token latency"
                )
                
                # 读取流，记录首 token 时间
                stream_url = f"{config.base_url}{result['stream_url']}"
                
                async with session.get(stream_url) as response:
                    async for line in response.content:
                        line = line.decode('utf-8').strip()
                        if line.startswith('data: '):
                            first_token_time = time.time()
                            break
                
                if first_token_time:
                    latency_ms = (first_token_time - start_time) * 1000
                    latencies.append(latency_ms)
                
                await asyncio.sleep(0.5)  # 间隔，避免请求过快
        
        # 统计
        if latencies:
            print(f"\n首 Token 延迟统计:")
            print(f"  样本数: {len(latencies)}")
            print(f"  平均: {statistics.mean(latencies):.2f} ms")
            print(f"  最小: {min(latencies):.2f} ms")
            print(f"  最大: {max(latencies):.2f} ms")
            print(f"  P95: {sorted(latencies)[int(len(latencies)*0.95)]:.2f} ms")
            
            # 断言：平均首 token 延迟 < 5s
            assert statistics.mean(latencies) < 5000
    
    async def test_total_response_time(self, config: TestConfig):
        """测试总响应时间"""
        durations = []
        
        for i in range(10):
            async with aiohttp.ClientSession() as session:
                start_time = time.time()
                
                # 创建会话
                result = await create_session_via_http(
                    session, config, "Measure total response time"
                )
                
                # 读取完整流
                stream_url = f"{config.base_url}{result['stream_url']}"
                
                async with session.get(stream_url) as response:
                    async for line in response.content:
                        line = line.decode('utf-8').strip()
                        if line.startswith('data: '):
                            data = line[6:]
                            try:
                                event = json.loads(data)
                                if event.get('type') in ['Complete', 'Error']:
                                    break
                            except json.JSONDecodeError:
                                pass
                
                duration_ms = (time.time() - start_time) * 1000
                durations.append(duration_ms)
                
                await asyncio.sleep(0.5)
        
        # 统计
        if durations:
            print(f"\n总响应时间统计:")
            print(f"  样本数: {len(durations)}")
            print(f"  平均: {statistics.mean(durations):.2f} ms")
            print(f"  最小: {min(durations):.2f} ms")
            print(f"  最大: {max(durations):.2f} ms")
            print(f"  P95: {sorted(durations)[int(len(durations)*0.95)]:.2f} ms")
            
            # 断言：平均响应时间 < 30s
            assert statistics.mean(durations) < 30000
    
    async def test_ws_latency(self, config: TestConfig):
        """测试 WebSocket 延迟"""
        latencies = []
        
        for i in range(10):
            session_id = f"latency-test-{uuid.uuid4().hex[:8]}"
            
            async with websockets.connect(config.ws_url) as ws:
                # 连接
                await connect_ws(ws, session_id)
                
                # 发送消息并计时
                start_time = time.time()
                await send_chat_via_ws(ws, session_id, "Latency test")
                
                # 等待第一个响应
                response = await asyncio.wait_for(ws.recv(), timeout=30)
                end_time = time.time()
                
                latency_ms = (end_time - start_time) * 1000
                latencies.append(latency_ms)
                
                await asyncio.sleep(0.5)
        
        # 统计
        if latencies:
            print(f"\nWebSocket 延迟统计:")
            print(f"  样本数: {len(latencies)}")
            print(f"  平均: {statistics.mean(latencies):.2f} ms")
            print(f"  最小: {min(latencies):.2f} ms")
            print(f"  最大: {max(latencies):.2f} ms")
            
            # 断言：平均延迟 < 5s
            assert statistics.mean(latencies) < 5000


# =============================================================================
# 吞吐量测试
# =============================================================================

@pytest.mark.asyncio
class TestThroughput:
    """吞吐量测试"""
    
    async def test_http_throughput(self, config: TestConfig):
        """测试 HTTP 吞吐量"""
        duration_seconds = 30
        concurrency = 20
        
        semaphore = asyncio.Semaphore(concurrency)
        success_count = 0
        error_count = 0
        start_time = time.time()
        
        async def request_task():
            nonlocal success_count, error_count
            
            async with semaphore:
                try:
                    async with aiohttp.ClientSession() as session:
                        result = await create_session_via_http(
                            session, config, "Throughput test"
                        )
                        
                        stream_url = f"{config.base_url}{result['stream_url']}"
                        
                        async with session.get(stream_url) as response:
                            async for line in response.content:
                                line = line.decode('utf-8').strip()
                                if line.startswith('data: '):
                                    data = line[6:]
                                    try:
                                        event = json.loads(data)
                                        if event.get('type') in ['Complete', 'Error']:
                                            break
                                    except json.JSONDecodeError:
                                        pass
                        
                        success_count += 1
                except Exception as e:
                    error_count += 1
        
        # 持续运行指定时间
        tasks = []
        while time.time() - start_time < duration_seconds:
            tasks.append(asyncio.create_task(request_task()))
            
            # 控制创建速度
            if len(tasks) >= concurrency * 2:
                done, pending = await asyncio.wait(
                    tasks, return_when=asyncio.FIRST_COMPLETED
                )
                tasks = list(pending)
        
        # 等待剩余任务
        if tasks:
            await asyncio.gather(*tasks, return_exceptions=True)
        
        actual_duration = time.time() - start_time
        throughput = success_count / actual_duration
        
        print(f"\nHTTP 吞吐量测试结果:")
        print(f"  持续时间: {actual_duration:.2f}s")
        print(f"  成功请求: {success_count}")
        print(f"  失败请求: {error_count}")
        print(f"  吞吐量: {throughput:.2f} req/s")
        
        # 断言：吞吐量 > 0.5 req/s
        assert throughput > 0.5
    
    async def test_ws_throughput(self, config: TestConfig):
        """测试 WebSocket 吞吐量"""
        duration_seconds = 30
        num_clients = 10
        messages_per_client = 20
        
        async def ws_client(client_id: int):
            """WebSocket 客户端"""
            session_id = f"throughput-ws-{client_id}-{uuid.uuid4().hex[:8]}"
            success_count = 0
            
            try:
                async with websockets.connect(config.ws_url) as ws:
                    await connect_ws(ws, session_id)
                    
                    start_time = time.time()
                    
                    while time.time() - start_time < duration_seconds and success_count < messages_per_client:
                        try:
                            await send_chat_via_ws(ws, session_id, f"Message {success_count}")
                            
                            # 等待响应
                            response = await asyncio.wait_for(ws.recv(), timeout=10)
                            data = json.loads(response)
                            
                            if data.get('type') not in ['Error']:
                                success_count += 1
                            
                            await asyncio.sleep(0.1)
                        except asyncio.TimeoutError:
                            break
                    
                    return success_count
            except Exception as e:
                return success_count
        
        start_time = time.time()
        
        # 启动多个客户端
        tasks = [ws_client(i) for i in range(num_clients)]
        results = await asyncio.gather(*tasks)
        
        actual_duration = time.time() - start_time
        total_messages = sum(results)
        throughput = total_messages / actual_duration
        
        print(f"\nWebSocket 吞吐量测试结果:")
        print(f"  客户端数: {num_clients}")
        print(f"  持续时间: {actual_duration:.2f}s")
        print(f"  总消息数: {total_messages}")
        print(f"  吞吐量: {throughput:.2f} msg/s")
        
        # 断言：吞吐量 > 1 msg/s
        assert throughput > 1


# =============================================================================
# 内存使用监控测试
# =============================================================================

@pytest.mark.asyncio
class TestMemoryUsage:
    """内存使用监控测试"""
    
    async def test_memory_under_load(self, config: TestConfig):
        """测试负载下的内存使用"""
        process = psutil.Process(os.getpid())
        
        # 记录初始内存
        initial_memory = process.memory_info().rss / 1024 / 1024
        print(f"\n初始内存使用: {initial_memory:.2f} MB")
        
        # 创建大量会话
        num_sessions = 50
        session_ids = []
        
        async with aiohttp.ClientSession() as session:
            for i in range(num_sessions):
                result = await create_session_via_http(
                    session, config, f"Memory test message {i}"
                )
                session_ids.append(result['session_id'])
                
                if i % 10 == 0:
                    current_memory = process.memory_info().rss / 1024 / 1024
                    print(f"  创建 {i} 个会话后: {current_memory:.2f} MB")
        
        # 记录最终内存
        final_memory = process.memory_info().rss / 1024 / 1024
        memory_increase = final_memory - initial_memory
        
        print(f"最终内存使用: {final_memory:.2f} MB")
        print(f"内存增量: {memory_increase:.2f} MB")
        print(f"每个会话平均内存: {memory_increase/num_sessions:.2f} MB")
        
        # 断言：内存增量 < 500MB
        assert memory_increase < 500
    
    async def test_memory_stability(self, config: TestConfig):
        """测试内存稳定性（长时间运行）"""
        process = psutil.Process(os.getpid())
        
        # 记录多个时间点的内存
        memory_samples = []
        duration_seconds = 60
        sample_interval = 10
        
        start_time = time.time()
        
        async def continuous_requests():
            """持续发送请求"""
            while time.time() - start_time < duration_seconds:
                try:
                    async with aiohttp.ClientSession() as session:
                        result = await create_session_via_http(
                            session, config, "Stability test"
                        )
                        
                        stream_url = f"{config.base_url}{result['stream_url']}"
                        
                        async with session.get(stream_url) as response:
                            async for line in response.content:
                                line = line.decode('utf-8').strip()
                                if line.startswith('data: '):
                                    data = line[6:]
                                    try:
                                        event = json.loads(data)
                                        if event.get('type') in ['Complete', 'Error']:
                                            break
                                    except json.JSONDecodeError:
                                        pass
                except Exception as e:
                    pass
                
                await asyncio.sleep(0.5)
        
        async def sample_memory():
            """采样内存"""
            while time.time() - start_time < duration_seconds:
                memory = process.memory_info().rss / 1024 / 1024
                memory_samples.append(memory)
                print(f"  内存采样: {memory:.2f} MB")
                await asyncio.sleep(sample_interval)
        
        # 同时运行请求和采样
        await asyncio.gather(continuous_requests(), sample_memory())
        
        # 分析内存趋势
        if len(memory_samples) >= 2:
            initial = memory_samples[0]
            final = memory_samples[-1]
            max_memory = max(memory_samples)
            
            print(f"\n内存稳定性分析:")
            print(f"  初始: {initial:.2f} MB")
            print(f"  最终: {final:.2f} MB")
            print(f"  最大: {max_memory:.2f} MB")
            print(f"  增长: {final - initial:.2f} MB")
            
            # 断言：内存增长 < 100MB
            assert final - initial < 100


# =============================================================================
# 综合性能基准测试
# =============================================================================

@pytest.mark.asyncio
class TestPerformanceBenchmark:
    """综合性能基准测试"""
    
    async def test_full_benchmark(self, config: TestConfig):
        """完整性能基准测试"""
        print("\n" + "="*60)
        print("Bamboo API 性能基准测试")
        print("="*60)
        
        results = []
        
        # 1. 低并发测试
        print("\n[1/4] 低并发测试 (10 concurrent)...")
        concurrent_test = TestConcurrentChat()
        result = await concurrent_test._run_concurrent_chat_test(
            config, concurrency=10, total_requests=30, test_name="Low Concurrency"
        )
        result.print_summary()
        results.append(result)
        
        await asyncio.sleep(2)
        
        # 2. 中并发测试
        print("\n[2/4] 中并发测试 (50 concurrent)...")
        result = await concurrent_test._run_concurrent_chat_test(
            config, concurrency=50, total_requests=100, test_name="Medium Concurrency"
        )
        result.print_summary()
        results.append(result)
        
        await asyncio.sleep(2)
        
        # 3. 高并发测试
        print("\n[3/4] 高并发测试 (100 concurrent)...")
        result = await concurrent_test._run_concurrent_chat_test(
            config, concurrency=100, total_requests=200, test_name="High Concurrency"
        )
        result.print_summary()
        results.append(result)
        
        await asyncio.sleep(2)
        
        # 4. 延迟测试
        print("\n[4/4] 延迟测试...")
        latency_test = TestLatencyMeasurement()
        await latency_test.test_first_token_latency(config)
        await latency_test.test_total_response_time(config)
        
        # 汇总报告
        print("\n" + "="*60)
        print("性能基准测试汇总")
        print("="*60)
        
        for r in results:
            print(f"\n{r.test_name}:")
            print(f"  成功率: {r.successful_requests/r.total_requests*100:.1f}%")
            print(f"  吞吐量: {r.throughput_rps:.2f} req/s")
            if r.latency_stats:
                print(f"  平均延迟: {r.latency_stats['avg_ms']:.0f} ms")
                print(f"  P95延迟: {r.latency_stats['p95_ms']:.0f} ms")
        
        print("\n" + "="*60)


# =============================================================================
# Locust 性能测试配置（用于更复杂的负载测试）
# =============================================================================

LOCUST_FILE_CONTENT = '''
"""
Locust 性能测试文件
使用方法: locust -f test_performance.py --host=http://127.0.0.1:8080
"""

from locust import HttpUser, task, between
import json
import uuid


class BambooUser(HttpUser):
    """Bamboo API 用户模拟"""
    wait_time = between(1, 5)
    
    def on_start(self):
        """用户启动时创建会话"""
        self.session_id = None
    
    @task(3)
    def chat_request(self):
        """模拟聊天请求"""
        payload = {
            "message": "Hello, this is a test message from Locust",
            "model": "gpt-4"
        }
        
        with self.client.post(
            "/api/v1/chat",
            json=payload,
            catch_response=True
        ) as response:
            if response.status_code == 201:
                data = response.json()
                self.session_id = data.get('session_id')
                response.success()
            else:
                response.failure(f"Failed to create session: {response.status_code}")
    
    @task(1)
    def health_check(self):
        """模拟健康检查"""
        with self.client.get("/api/v1/health", catch_response=True) as response:
            if response.status_code == 200:
                response.success()
            else:
                response.failure(f"Health check failed: {response.status_code}")
    
    @task(2)
    def get_history(self):
        """模拟获取历史记录"""
        if self.session_id:
            with self.client.get(
                f"/api/v1/history/{self.session_id}",
                catch_response=True
            ) as response:
                if response.status_code == 200:
                    response.success()
                else:
                    response.failure(f"Failed to get history: {response.status_code}")


class WebSocketUser:
    """WebSocket 用户（需要额外配置）"""
    # 注意：Locust 默认不支持 WebSocket，需要使用 locust-plugins
    pass
'''


def create_locust_file():
    """创建 Locust 测试文件"""
    locust_path = os.path.join(os.path.dirname(__file__), 'locustfile.py')
    with open(locust_path, 'w') as f:
        f.write(LOCUST_FILE_CONTENT)
    print(f"Locust 测试文件已创建: {locust_path}")


# =============================================================================
# 主函数
# =============================================================================

if __name__ == "__main__":
    # 创建 Locust 文件
    create_locust_file()
    
    # 运行 pytest
    pytest.main([__file__, "-v", "--tb=short", "-s"])
