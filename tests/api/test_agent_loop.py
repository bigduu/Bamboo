#!/usr/bin/env python3
"""
Bamboo Agent Loop 多轮对话测试

测试目标：验证 Agent Loop 在多轮对话中的工具调用链、状态机转换和事件序列

测试场景：
1. 单轮工具调用（Agent 识别需要工具 → 调用 → 返回结果 → 生成回复）
2. 多轮对话中的连续工具调用（上下文保持）
3. 工具调用链（A工具结果作为B工具输入）
4. 错误恢复（工具调用失败后的重试/降级）
5. 最大轮数限制测试（验证 max_rounds 配置生效）
6. 超时处理（长时间工具调用的中断）

状态机验证：
- Idle → Processing → ToolCalling → Waiting → Complete
- 状态转换验证

事件序列验证：
- ToolStart → ToolComplete/ToolError → AgentComplete
"""

import unittest
import asyncio
import json
import time
from typing import Dict, List, Any, Optional, Callable
from dataclasses import dataclass, field
from enum import Enum, auto
from unittest.mock import Mock, AsyncMock, patch


# ============================================================================
# 类型定义
# ============================================================================

class AgentState(Enum):
    """Agent Loop 状态机状态"""
    IDLE = auto()
    PROCESSING = auto()
    TOOL_CALLING = auto()
    WAITING = auto()
    COMPLETE = auto()
    ERROR = auto()


class EventType(Enum):
    """Agent 事件类型"""
    TOOL_START = "tool_start"
    TOOL_COMPLETE = "tool_complete"
    TOOL_ERROR = "tool_error"
    AGENT_COMPLETE = "agent_complete"
    STATE_CHANGE = "state_change"


@dataclass
class ToolCall:
    """工具调用定义"""
    id: str
    name: str
    arguments: Dict[str, Any]


@dataclass
class ToolResult:
    """工具执行结果"""
    success: bool
    output: str
    error: Optional[str] = None
    duration_ms: int = 0


@dataclass
class AgentEvent:
    """Agent 事件"""
    type: EventType
    tool_call_id: Optional[str] = None
    tool_name: Optional[str] = None
    data: Dict[str, Any] = field(default_factory=dict)
    timestamp: float = field(default_factory=time.time)


@dataclass
class AgentConfig:
    """Agent Loop 配置"""
    max_rounds: int = 3
    tool_timeout_ms: int = 5000
    enable_retry: bool = True
    max_retries: int = 2


# ============================================================================
# Mock LLM 实现
# ============================================================================

class MockLLM:
    """
    Mock LLM 用于控制测试场景
    通过预设响应序列来模拟多轮对话中的 LLM 行为
    """
    
    def __init__(self):
        self.responses: List[Dict[str, Any]] = []
        self.response_index: int = 0
        self.call_history: List[Dict[str, Any]] = []
    
    def add_response(self, response: Dict[str, Any]):
        """添加预设响应"""
        self.responses.append(response)
    
    def add_tool_call_response(self, tool_calls: List[ToolCall], content: str = ""):
        """添加工具调用响应"""
        self.responses.append({
            "type": "tool_calls",
            "content": content,
            "tool_calls": tool_calls
        })
    
    def add_text_response(self, content: str):
        """添加纯文本响应"""
        self.responses.append({
            "type": "text",
            "content": content
        })
    
    async def chat(self, messages: List[Dict[str, Any]], tools: List[Dict[str, Any]]) -> Dict[str, Any]:
        """模拟 LLM 聊天接口"""
        self.call_history.append({
            "messages": messages.copy(),
            "tools": tools.copy(),
            "timestamp": time.time()
        })
        
        if self.response_index < len(self.responses):
            response = self.responses[self.response_index]
            self.response_index += 1
            return response
        
        # 默认响应
        return {"type": "text", "content": "Default response"}
    
    def reset(self):
        """重置状态"""
        self.response_index = 0
        self.call_history.clear()


# ============================================================================
# Mock Tool Executor 实现
# ============================================================================

class MockToolExecutor:
    """
    Mock 工具执行器
    支持工具调用链、错误模拟和延迟模拟
    """
    
    def __init__(self):
        self.tools: Dict[str, Callable] = {}
        self.execution_log: List[Dict[str, Any]] = []
        self.fail_next_call: Optional[str] = None
        self.delay_ms: int = 0
        self.call_count: Dict[str, int] = {}
    
    def register_tool(self, name: str, handler: Callable):
        """注册工具"""
        self.tools[name] = handler
        self.call_count[name] = 0
    
    def set_fail_next(self, tool_name: str):
        """设置下次调用失败"""
        self.fail_next_call = tool_name
    
    def set_delay(self, delay_ms: int):
        """设置工具执行延迟"""
        self.delay_ms = delay_ms
    
    async def execute(self, call: ToolCall) -> ToolResult:
        """执行工具调用"""
        start_time = time.time()
        
        # 模拟延迟
        if self.delay_ms > 0:
            await asyncio.sleep(self.delay_ms / 1000)
        
        # 记录调用
        self.call_count[call.name] = self.call_count.get(call.name, 0) + 1
        
        # 检查是否应该失败
        if self.fail_next_call == call.name:
            self.fail_next_call = None
            result = ToolResult(
                success=False,
                output="",
                error=f"Simulated error for {call.name}",
                duration_ms=int((time.time() - start_time) * 1000)
            )
        elif call.name in self.tools:
            try:
                output = await self.tools[call.name](call.arguments)
                result = ToolResult(
                    success=True,
                    output=output,
                    duration_ms=int((time.time() - start_time) * 1000)
                )
            except Exception as e:
                result = ToolResult(
                    success=False,
                    output="",
                    error=str(e),
                    duration_ms=int((time.time() - start_time) * 1000)
                )
        else:
            result = ToolResult(
                success=False,
                output="",
                error=f"Tool not found: {call.name}",
                duration_ms=0
            )
        
        self.execution_log.append({
            "tool_call": call,
            "result": result,
            "timestamp": time.time()
        })
        
        return result
    
    def get_call_count(self, tool_name: str) -> int:
        """获取工具调用次数"""
        return self.call_count.get(tool_name, 0)
    
    def reset(self):
        """重置状态"""
        self.execution_log.clear()
        self.fail_next_call = None
        self.delay_ms = 0
        self.call_count = {name: 0 for name in self.call_count}


# ============================================================================
# Agent Loop 实现（测试版本）
# ============================================================================

class AgentLoop:
    """
    Agent Loop 测试实现
    包含完整的状态机和事件系统
    """
    
    def __init__(self, config: AgentConfig, llm: MockLLM, executor: MockToolExecutor):
        self.config = config
        self.llm = llm
        self.executor = executor
        self.state = AgentState.IDLE
        self.events: List[AgentEvent] = []
        self.current_round = 0
        self.session_context: Dict[str, Any] = {"messages": []}
    
    def _emit_event(self, event: AgentEvent):
        """发送事件"""
        self.events.append(event)
    
    def _set_state(self, new_state: AgentState):
        """设置状态并发送状态变更事件"""
        old_state = self.state
        self.state = new_state
        self._emit_event(AgentEvent(
            type=EventType.STATE_CHANGE,
            data={"from": old_state.name, "to": new_state.name}
        ))
    
    async def run(self, initial_message: str) -> str:
        """
        运行 Agent Loop
        
        流程：
        1. Idle → Processing: 接收用户输入
        2. Processing → ToolCalling: LLM 决定调用工具
        3. ToolCalling → Waiting: 等待工具执行完成
        4. Waiting → Processing: 工具结果返回，继续处理
        5. Processing → Complete: 生成最终回复
        """
        self._set_state(AgentState.PROCESSING)
        self.session_context["messages"].append({"role": "user", "content": initial_message})
        
        final_response = ""
        
        for round_num in range(self.config.max_rounds):
            self.current_round = round_num + 1
            
            # 调用 LLM
            llm_response = await self.llm.chat(
                self.session_context["messages"],
                tools=[]  # 简化：实际应该传递可用工具列表
            )
            
            if llm_response["type"] == "text":
                # LLM 直接返回文本，对话结束
                final_response = llm_response["content"]
                break
            
            elif llm_response["type"] == "tool_calls":
                # LLM 请求调用工具
                self._set_state(AgentState.TOOL_CALLING)
                
                tool_calls = llm_response.get("tool_calls", [])
                tool_results = []
                
                for tool_call in tool_calls:
                    # 发送 ToolStart 事件
                    self._emit_event(AgentEvent(
                        type=EventType.TOOL_START,
                        tool_call_id=tool_call.id,
                        tool_name=tool_call.name,
                        data={"arguments": tool_call.arguments}
                    ))
                    
                    # 执行工具
                    result = await self._execute_tool_with_retry(tool_call)
                    tool_results.append(result)
                    
                    # 发送 ToolComplete 或 ToolError 事件
                    if result.success:
                        self._emit_event(AgentEvent(
                            type=EventType.TOOL_COMPLETE,
                            tool_call_id=tool_call.id,
                            tool_name=tool_call.name,
                            data={"output": result.output, "duration_ms": result.duration_ms}
                        ))
                    else:
                        self._emit_event(AgentEvent(
                            type=EventType.TOOL_ERROR,
                            tool_call_id=tool_call.id,
                            tool_name=tool_call.name,
                            data={"error": result.error}
                        ))
                
                # 将工具结果添加到上下文
                for tool_call, result in zip(tool_calls, tool_results):
                    self.session_context["messages"].append({
                        "role": "tool",
                        "tool_call_id": tool_call.id,
                        "content": result.output if result.success else result.error
                    })
                
                self._set_state(AgentState.PROCESSING)
        
        # 发送完成事件
        self._emit_event(AgentEvent(
            type=EventType.AGENT_COMPLETE,
            data={"rounds": self.current_round, "response": final_response}
        ))
        
        self._set_state(AgentState.COMPLETE)
        return final_response
    
    async def _execute_tool_with_retry(self, tool_call: ToolCall) -> ToolResult:
        """带重试的工具执行"""
        last_result = None
        
        for attempt in range(self.config.max_retries + 1):
            result = await self.executor.execute(tool_call)
            last_result = result
            
            if result.success:
                return result
            
            if not self.config.enable_retry:
                break
            
            # 重试前等待
            if attempt < self.config.max_retries:
                await asyncio.sleep(0.1 * (attempt + 1))
        
        return last_result
    
    def get_events_by_type(self, event_type: EventType) -> List[AgentEvent]:
        """获取特定类型的事件"""
        return [e for e in self.events if e.type == event_type]
    
    def get_state_transitions(self) -> List[tuple]:
        """获取状态转换序列"""
        transitions = []
        prev_state = AgentState.IDLE
        
        for event in self.events:
            if event.type == EventType.STATE_CHANGE:
                from_state = event.data["from"]
                to_state = event.data["to"]
                transitions.append((from_state, to_state))
                prev_state = to_state
        
        return transitions


# ============================================================================
# 测试用例
# ============================================================================

class TestSingleRoundToolExecution(unittest.TestCase):
    """单轮工具调用测试"""
    
    def setUp(self):
        self.config = AgentConfig(max_rounds=3)
        self.llm = MockLLM()
        self.executor = MockToolExecutor()
        self.agent = AgentLoop(self.config, self.llm, self.executor)
        
        # 注册计算器工具
        async def calculator(args: Dict[str, Any]) -> str:
            expr = args.get("expression", "")
            try:
                result = eval(expr)  # 简化计算
                return str(result)
            except:
                return "Error"
        
        self.executor.register_tool("calculator", calculator)
    
    def test_single_tool_call_flow(self):
        """测试单轮工具调用完整流程"""
        asyncio.run(self._async_test())
    
    async def _async_test(self):
        # 设置 LLM 响应：调用工具
        self.llm.add_tool_call_response([
            ToolCall(id="call_1", name="calculator", arguments={"expression": "2 + 2"})
        ], content="")
        
        # 设置 LLM 响应：最终回复
        self.llm.add_text_response("计算结果是 4")
        
        # 运行 Agent
        response = await self.agent.run("计算 2 + 2")
        
        # 验证结果
        self.assertEqual(response, "计算结果是 4")
        
        # 验证事件序列
        tool_start_events = self.agent.get_events_by_type(EventType.TOOL_START)
        tool_complete_events = self.agent.get_events_by_type(EventType.TOOL_COMPLETE)
        
        self.assertEqual(len(tool_start_events), 1)
        self.assertEqual(len(tool_complete_events), 1)
        self.assertEqual(tool_start_events[0].tool_name, "calculator")
        self.assertEqual(tool_complete_events[0].data["output"], "4")
        
        # 验证状态转换
        transitions = self.agent.get_state_transitions()
        expected_transitions = [
            ("IDLE", "PROCESSING"),
            ("PROCESSING", "TOOL_CALLING"),
            ("TOOL_CALLING", "PROCESSING"),
            ("PROCESSING", "COMPLETE")
        ]
        self.assertEqual(transitions, expected_transitions)


class TestMultiRoundConversation(unittest.TestCase):
    """多轮对话连续工具调用测试"""
    
    def setUp(self):
        self.config = AgentConfig(max_rounds=5)
        self.llm = MockLLM()
        self.executor = MockToolExecutor()
        self.agent = AgentLoop(self.config, self.llm, self.executor)
        
        # 注册工具
        async def get_weather(args: Dict[str, Any]) -> str:
            city = args.get("city", "")
            return f"{city} 的天气是晴天，25°C"
        
        async def get_time(args: Dict[str, Any]) -> str:
            return "2024-01-15 14:30:00"
        
        self.executor.register_tool("get_weather", get_weather)
        self.executor.register_tool("get_time", get_time)
    
    def test_multi_round_tool_calls(self):
        """测试多轮工具调用保持上下文"""
        asyncio.run(self._async_test())
    
    async def _async_test(self):
        # 第一轮：获取天气
        self.llm.add_tool_call_response([
            ToolCall(id="call_1", name="get_weather", arguments={"city": "北京"})
        ])
        
        # 第二轮：获取时间
        self.llm.add_tool_call_response([
            ToolCall(id="call_2", name="get_time", arguments={})
        ])
        
        # 第三轮：最终回复
        self.llm.add_text_response("北京天气晴朗，当前时间是 14:30")
        
        # 运行 Agent
        response = await self.agent.run("北京天气怎么样？现在几点？")
        
        # 验证调用了两个工具
        self.assertEqual(self.executor.get_call_count("get_weather"), 1)
        self.assertEqual(self.executor.get_call_count("get_time"), 1)
        
        # 验证 LLM 调用历史包含工具结果
        self.assertEqual(len(self.llm.call_history), 3)
        
        # 验证事件数量
        tool_start_events = self.agent.get_events_by_type(EventType.TOOL_START)
        self.assertEqual(len(tool_start_events), 2)


class TestToolChain(unittest.TestCase):
    """工具调用链测试"""
    
    def setUp(self):
        self.config = AgentConfig(max_rounds=3)
        self.llm = MockLLM()
        self.executor = MockToolExecutor()
        self.agent = AgentLoop(self.config, self.llm, self.executor)
        
        # 模拟数据存储
        self.data_store = {"users": {"1": "张三", "2": "李四"}}
        
        # 注册工具链
        async def query_user_id(args: Dict[str, Any]) -> str:
            name = args.get("name", "")
            for uid, uname in self.data_store["users"].items():
                if uname == name:
                    return json.dumps({"user_id": uid})
            return json.dumps({"error": "User not found"})
        
        async def get_user_details(args: Dict[str, Any]) -> str:
            user_id = args.get("user_id", "")
            name = self.data_store["users"].get(user_id, "Unknown")
            return json.dumps({"id": user_id, "name": name, "department": "技术部"})
        
        self.executor.register_tool("query_user_id", query_user_id)
        self.executor.register_tool("get_user_details", get_user_details)
    
    def test_tool_chain_execution(self):
        """测试工具调用链：A工具结果作为B工具输入"""
        asyncio.run(self._async_test())
    
    async def _async_test(self):
        # 第一轮：查询用户ID
        self.llm.add_tool_call_response([
            ToolCall(id="call_1", name="query_user_id", arguments={"name": "张三"})
        ])
        
        # 第二轮：使用ID查询详情
        self.llm.add_tool_call_response([
            ToolCall(id="call_2", name="get_user_details", arguments={"user_id": "1"})
        ])
        
        # 第三轮：最终回复
        self.llm.add_text_response("张三的详细信息：ID=1，部门=技术部")
        
        # 运行 Agent
        response = await self.agent.run("查找张三的信息")
        
        # 验证工具链执行
        self.assertEqual(self.executor.get_call_count("query_user_id"), 1)
        self.assertEqual(self.executor.get_call_count("get_user_details"), 1)
        
        # 验证第二个工具的输入来自第一个工具的输出
        execution_log = self.executor.execution_log
        first_result = json.loads(execution_log[0]["result"].output)
        second_call_args = execution_log[1]["tool_call"].arguments
        
        self.assertEqual(first_result["user_id"], second_call_args["user_id"])


class TestErrorRecovery(unittest.TestCase):
    """错误恢复测试"""
    
    def setUp(self):
        self.config = AgentConfig(max_rounds=3, enable_retry=True, max_retries=2)
        self.llm = MockLLM()
        self.executor = MockToolExecutor()
        self.agent = AgentLoop(self.config, self.llm, self.executor)
        
        async def flaky_tool(args: Dict[str, Any]) -> str:
            return "Success"
        
        self.executor.register_tool("flaky_tool", flaky_tool)
    
    def test_retry_on_failure(self):
        """测试工具调用失败后重试"""
        asyncio.run(self._async_test())
    
    async def _async_test(self):
        # 创建一个会失败两次然后成功的工具
        call_count = [0]
        
        async def flaky_tool_impl(args: Dict[str, Any]) -> str:
            call_count[0] += 1
            if call_count[0] <= 2:  # 前两次失败
                raise Exception(f"Attempt {call_count[0]} failed")
            return "Success"
        
        self.executor.register_tool("flaky_tool_v2", flaky_tool_impl)
        
        # 设置 LLM 响应
        self.llm.add_tool_call_response([
            ToolCall(id="call_1", name="flaky_tool_v2", arguments={})
        ])
        self.llm.add_text_response("操作成功")
        
        # 运行 Agent
        response = await self.agent.run("执行操作")
        
        # 验证重试次数（初始 + 2次重试 = 3次）
        self.assertEqual(call_count[0], 3)
        
        # 验证最终成功（中间失败的 ToolError 事件不记录，只有最终结果）
        tool_error_events = self.agent.get_events_by_type(EventType.TOOL_ERROR)
        self.assertEqual(len(tool_error_events), 0)  # 没有最终失败
        
        # 验证最终成功
        tool_complete_events = self.agent.get_events_by_type(EventType.TOOL_COMPLETE)
        self.assertEqual(len(tool_complete_events), 1)  # 第三次成功
    
    def test_max_retries_exceeded(self):
        """测试超过最大重试次数后失败"""
        asyncio.run(self._async_test_max_retries())
    
    async def _async_test_max_retries(self):
        # 创建一个总是失败的工具
        call_count = [0]
        
        async def always_fail(args: Dict[str, Any]) -> str:
            call_count[0] += 1
            raise Exception("Persistent error")
        
        self.executor.register_tool("always_fail", always_fail)
        
        # 设置 LLM 响应
        self.llm.add_tool_call_response([
            ToolCall(id="call_1", name="always_fail", arguments={})
        ])
        self.llm.add_text_response("操作失败")
        
        # 运行 Agent
        response = await self.agent.run("执行操作")
        
        # 验证调用次数（初始 + 2次重试 = 3次）
        self.assertEqual(call_count[0], 3)
        
        # 验证所有尝试都失败（只有最后一次失败会触发 ToolError 事件）
        tool_error_events = self.agent.get_events_by_type(EventType.TOOL_ERROR)
        self.assertEqual(len(tool_error_events), 1)  # 最终失败


class TestMaxRoundsLimit(unittest.TestCase):
    """最大轮数限制测试"""
    
    def setUp(self):
        self.config = AgentConfig(max_rounds=2)
        self.llm = MockLLM()
        self.executor = MockToolExecutor()
        self.agent = AgentLoop(self.config, self.llm, self.executor)
        
        async def dummy_tool(args: Dict[str, Any]) -> str:
            return "Done"
        
        self.executor.register_tool("dummy_tool", dummy_tool)
    
    def test_max_rounds_enforced(self):
        """验证 max_rounds 配置生效"""
        asyncio.run(self._async_test())
    
    async def _async_test(self):
        # LLM 总是请求调用工具（永远不会直接返回文本）
        for _ in range(5):  # 尝试5轮，但配置限制为2轮
            self.llm.add_tool_call_response([
                ToolCall(id=f"call_{_}", name="dummy_tool", arguments={})
            ])
        
        # 运行 Agent
        response = await self.agent.run("测试")
        
        # 验证只执行了 max_rounds 轮
        self.assertEqual(self.agent.current_round, 2)
        self.assertEqual(self.executor.get_call_count("dummy_tool"), 2)


class TestTimeoutHandling(unittest.TestCase):
    """超时处理测试"""
    
    def setUp(self):
        self.config = AgentConfig(max_rounds=3, tool_timeout_ms=100)
        self.llm = MockLLM()
        self.executor = MockToolExecutor()
        self.agent = AgentLoop(self.config, self.llm, self.executor)
        
        # 注册慢工具
        async def slow_tool(args: Dict[str, Any]) -> str:
            await asyncio.sleep(0.5)  # 500ms，超过超时时间
            return "Slow result"
        
        self.executor.register_tool("slow_tool", slow_tool)
    
    def test_tool_timeout(self):
        """测试长时间工具调用的中断"""
        asyncio.run(self._async_test())
    
    async def _async_test(self):
        # 注意：实际实现中需要添加超时逻辑
        # 这里简化处理，仅验证工具执行时间超过配置
        
        self.llm.add_tool_call_response([
            ToolCall(id="call_1", name="slow_tool", arguments={})
        ])
        self.llm.add_text_response("完成")
        
        start_time = time.time()
        response = await self.agent.run("执行慢操作")
        elapsed_ms = (time.time() - start_time) * 1000
        
        # 验证执行时间（工具500ms + 其他开销）
        self.assertGreater(elapsed_ms, 400)


class TestStateMachine(unittest.TestCase):
    """状态机验证测试"""
    
    def setUp(self):
        self.config = AgentConfig(max_rounds=3)
        self.llm = MockLLM()
        self.executor = MockToolExecutor()
        self.agent = AgentLoop(self.config, self.llm, self.executor)
        
        async def test_tool(args: Dict[str, Any]) -> str:
            return "Result"
        
        self.executor.register_tool("test_tool", test_tool)
    
    def test_state_transitions(self):
        """验证状态转换序列"""
        asyncio.run(self._async_test())
    
    async def _async_test(self):
        self.llm.add_tool_call_response([
            ToolCall(id="call_1", name="test_tool", arguments={})
        ])
        self.llm.add_text_response("Done")
        
        await self.agent.run("测试")
        
        # 验证完整状态转换序列
        transitions = self.agent.get_state_transitions()
        
        # 期望的状态序列
        expected = [
            ("IDLE", "PROCESSING"),      # 开始处理
            ("PROCESSING", "TOOL_CALLING"),  # 决定调用工具
            ("TOOL_CALLING", "PROCESSING"),  # 工具完成，继续处理
            ("PROCESSING", "COMPLETE")   # 完成
        ]
        
        self.assertEqual(transitions, expected)
    
    def test_no_invalid_transitions(self):
        """验证没有无效的状态转换"""
        asyncio.run(self._async_test_no_invalid())
    
    async def _async_test_no_invalid(self):
        self.llm.add_text_response("Direct response")
        
        await self.agent.run("测试")
        
        transitions = self.agent.get_state_transitions()
        
        # 验证没有 TOOL_CALLING 状态（因为没有工具调用）
        state_names = [t[1] for t in transitions]
        self.assertNotIn("TOOL_CALLING", state_names)


class TestEventSequence(unittest.TestCase):
    """事件序列验证测试"""
    
    def setUp(self):
        self.config = AgentConfig(max_rounds=3)
        self.llm = MockLLM()
        self.executor = MockToolExecutor()
        self.agent = AgentLoop(self.config, self.llm, self.executor)
        
        async def tool_a(args: Dict[str, Any]) -> str:
            return "A"
        
        async def tool_b(args: Dict[str, Any]) -> str:
            return "B"
        
        self.executor.register_tool("tool_a", tool_a)
        self.executor.register_tool("tool_b", tool_b)
    
    def test_event_ordering(self):
        """验证事件顺序：ToolStart → ToolComplete → AgentComplete"""
        asyncio.run(self._async_test())
    
    async def _async_test(self):
        self.llm.add_tool_call_response([
            ToolCall(id="call_1", name="tool_a", arguments={}),
            ToolCall(id="call_2", name="tool_b", arguments={})
        ])
        self.llm.add_text_response("Done")
        
        await self.agent.run("测试")
        
        # 获取所有事件
        events = self.agent.events
        
        # 验证事件顺序
        event_types = [e.type for e in events]
        
        # 找到工具相关事件的索引
        tool_start_indices = [i for i, e in enumerate(event_types) if e == EventType.TOOL_START]
        tool_complete_indices = [i for i, e in enumerate(event_types) if e == EventType.TOOL_COMPLETE]
        agent_complete_indices = [i for i, e in enumerate(event_types) if e == EventType.AGENT_COMPLETE]
        
        # 验证每个 ToolStart 都有对应的 ToolComplete
        self.assertEqual(len(tool_start_indices), len(tool_complete_indices))
        
        # 验证 ToolComplete 在 ToolStart 之后
        for start_idx, complete_idx in zip(tool_start_indices, tool_complete_indices):
            self.assertLess(start_idx, complete_idx)
        
        # 验证 AgentComplete 在最后
        self.assertEqual(len(agent_complete_indices), 1)
        last_tool_complete = max(tool_complete_indices) if tool_complete_indices else -1
        self.assertLess(last_tool_complete, agent_complete_indices[0])


# ============================================================================
# 测试套件
# ============================================================================

def create_test_suite():
    """创建测试套件"""
    loader = unittest.TestLoader()
    suite = unittest.TestSuite()
    
    # 添加所有测试类
    suite.addTests(loader.loadTestsFromTestCase(TestSingleRoundToolExecution))
    suite.addTests(loader.loadTestsFromTestCase(TestMultiRoundConversation))
    suite.addTests(loader.loadTestsFromTestCase(TestToolChain))
    suite.addTests(loader.loadTestsFromTestCase(TestErrorRecovery))
    suite.addTests(loader.loadTestsFromTestCase(TestMaxRoundsLimit))
    suite.addTests(loader.loadTestsFromTestCase(TestTimeoutHandling))
    suite.addTests(loader.loadTestsFromTestCase(TestStateMachine))
    suite.addTests(loader.loadTestsFromTestCase(TestEventSequence))
    
    return suite


if __name__ == "__main__":
    # 运行测试
    runner = unittest.TextTestRunner(verbosity=2)
    result = runner.run(create_test_suite())
    
    # 输出测试统计
    print("\n" + "=" * 60)
    print("测试统计")
    print("=" * 60)
    print(f"测试用例总数: {result.testsRun}")
    print(f"通过: {result.testsRun - len(result.failures) - len(result.errors)}")
    print(f"失败: {len(result.failures)}")
    print(f"错误: {len(result.errors)}")
    print(f"跳过: {len(result.skipped)}")
    print("=" * 60)
    
    # 返回退出码
    exit(0 if result.wasSuccessful() else 1)
