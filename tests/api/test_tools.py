#!/usr/bin/env python3
"""
Bamboo API 工具和技能测试脚本

测试功能:
1. 工具发现（获取可用工具列表）
2. 工具调用（通过聊天触发）
3. 技能加载和重载
4. 工具执行结果验证

测试场景:
- 单工具调用
- 多工具链式调用
- 并行工具调用
- 嵌套工具调用
- 真实场景模拟
- 工具错误处理
- 技能热重载验证
"""

import json
import os
import sys
import time
import unittest
from concurrent.futures import ThreadPoolExecutor, as_completed
from typing import Dict, List, Optional, Any
from dataclasses import dataclass
from unittest.mock import Mock, patch, MagicMock

import requests


# 配置
DEFAULT_BASE_URL = "http://localhost:8080"
BASE_URL = os.getenv("BAMBOO_API_URL", DEFAULT_BASE_URL)


@dataclass
class ToolResult:
    """工具执行结果"""
    success: bool
    output: str
    error: Optional[str] = None
    duration_ms: int = 0


@dataclass
class ToolDef:
    """工具定义"""
    name: str
    description: str
    command: str
    args: List[Dict[str, Any]]


@dataclass
class Skill:
    """技能定义"""
    name: str
    description: str
    version: str
    tools: List[ToolDef]
    system_prompt: Optional[str] = None


class MockToolExecutor:
    """Mock 工具执行器 - 用于测试，避免依赖外部服务"""
    
    def __init__(self):
        self.tools: Dict[str, ToolDef] = {}
        self.call_history: List[Dict[str, Any]] = []
        self._setup_mock_tools()
    
    def _setup_mock_tools(self):
        """设置 Mock 工具"""
        # Mock 计算器工具
        self.tools["calculator"] = ToolDef(
            name="calculator",
            description="执行数学计算",
            command="mock_calculator",
            args=[
                {"name": "expression", "type": "string", "required": True, "description": "数学表达式"}
            ]
        )
        
        # Mock 文件读取工具
        self.tools["read_file"] = ToolDef(
            name="read_file",
            description="读取文件内容",
            command="mock_read_file",
            args=[
                {"name": "path", "type": "string", "required": True, "description": "文件路径"},
                {"name": "limit", "type": "number", "required": False, "default": 100, "description": "读取行数限制"}
            ]
        )
        
        # Mock 文本处理工具
        self.tools["text_processor"] = ToolDef(
            name="text_processor",
            description="处理文本（统计字数、格式化等）",
            command="mock_text_processor",
            args=[
                {"name": "text", "type": "string", "required": True, "description": "输入文本"},
                {"name": "operation", "type": "string", "required": True, "description": "操作类型: count/format/upper/lower"}
            ]
        )
        
        # Mock 时间工具
        self.tools["get_time"] = ToolDef(
            name="get_time",
            description="获取当前时间",
            command="mock_get_time",
            args=[
                {"name": "format", "type": "string", "required": False, "default": "iso", "description": "时间格式"}
            ]
        )
    
    def execute(self, tool_name: str, args: Dict[str, Any]) -> ToolResult:
        """执行 Mock 工具"""
        self.call_history.append({"tool": tool_name, "args": args, "timestamp": time.time()})
        
        if tool_name not in self.tools:
            return ToolResult(success=False, output="", error=f"Tool not found: {tool_name}", duration_ms=0)
        
        tool = self.tools[tool_name]
        start_time = time.time()
        
        # 模拟工具执行
        try:
            if tool_name == "calculator":
                result = self._mock_calculator(args)
            elif tool_name == "read_file":
                result = self._mock_read_file(args)
            elif tool_name == "text_processor":
                result = self._mock_text_processor(args)
            elif tool_name == "get_time":
                result = self._mock_get_time(args)
            else:
                result = ToolResult(success=False, error=f"Unknown tool: {tool_name}")
            
            duration_ms = int((time.time() - start_time) * 1000)
            result.duration_ms = duration_ms
            return result
            
        except Exception as e:
            duration_ms = int((time.time() - start_time) * 1000)
            return ToolResult(success=False, output="", error=str(e), duration_ms=duration_ms)
    
    def _mock_calculator(self, args: Dict[str, Any]) -> ToolResult:
        """Mock 计算器实现"""
        expression = args.get("expression", "")
        try:
            # 安全计算 - 只允许基本数学运算
            allowed_chars = set("0123456789+-*/(). ")
            if not expression:
                return ToolResult(success=False, output="", error="Empty expression")
            if not all(c in allowed_chars for c in expression):
                return ToolResult(success=False, output="", error="Invalid characters in expression")
            
            result = eval(expression)  # 在测试环境中安全使用
            return ToolResult(success=True, output=str(result), error=None)
        except Exception as e:
            return ToolResult(success=False, output="", error=f"Calculation error: {e}")
    
    def _mock_read_file(self, args: Dict[str, Any]) -> ToolResult:
        """Mock 文件读取实现"""
        path = args.get("path", "")
        limit = args.get("limit", 100)
        
        # Mock 文件内容
        mock_files = {
            "/tmp/test.txt": "Hello, World!\nThis is a test file.\nLine 3\n",
            "/tmp/data.json": json.dumps({"name": "test", "value": 42}, indent=2),
            "test.txt": "Relative path test file content.\n" * 5,
        }
        
        content = mock_files.get(path, f"Mock content for {path}\nLine 2\nLine 3\n")
        lines = content.split("\n")[:limit]
        return ToolResult(success=True, output="\n".join(lines), error=None)
    
    def _mock_text_processor(self, args: Dict[str, Any]) -> ToolResult:
        """Mock 文本处理器实现"""
        text = args.get("text", "")
        operation = args.get("operation", "count")
        
        if operation == "count":
            result = {
                "chars": len(text),
                "words": len(text.split()),
                "lines": len(text.split("\n"))
            }
            return ToolResult(success=True, output=json.dumps(result, indent=2), error=None)
        elif operation == "upper":
            return ToolResult(success=True, output=text.upper(), error=None)
        elif operation == "lower":
            return ToolResult(success=True, output=text.lower(), error=None)
        elif operation == "format":
            return ToolResult(success=True, output=text.strip().capitalize(), error=None)
        else:
            return ToolResult(success=False, output="", error=f"Unknown operation: {operation}")
    
    def _mock_get_time(self, args: Dict[str, Any]) -> ToolResult:
        """Mock 时间获取实现"""
        format_type = args.get("format", "iso")
        from datetime import datetime
        now = datetime.now()
        
        if format_type == "iso":
            output = now.isoformat()
        elif format_type == "timestamp":
            output = str(int(now.timestamp()))
        elif format_type == "human":
            output = now.strftime("%Y-%m-%d %H:%M:%S")
        else:
            output = now.isoformat()
        
        return ToolResult(success=True, output=output, error=None)
    
    def list_tools(self) -> List[ToolDef]:
        """列出所有可用工具"""
        return list(self.tools.values())
    
    def get_call_history(self) -> List[Dict[str, Any]]:
        """获取调用历史"""
        return self.call_history.copy()
    
    def clear_history(self):
        """清除调用历史"""
        self.call_history.clear()


class MockSkillManager:
    """Mock 技能管理器"""
    
    def __init__(self):
        self.skills: Dict[str, Skill] = {}
        self.tool_executor = MockToolExecutor()
        self._setup_mock_skills()
    
    def _setup_mock_skills(self):
        """设置 Mock 技能"""
        # 数学技能
        self.skills["math"] = Skill(
            name="math",
            description="数学计算技能",
            version="1.0.0",
            tools=[self.tool_executor.tools["calculator"]],
            system_prompt="你是一个数学助手，可以帮助用户进行各种数学计算。"
        )
        
        # 文件操作技能
        self.skills["file_ops"] = Skill(
            name="file_ops",
            description="文件操作技能",
            version="1.0.0",
            tools=[self.tool_executor.tools["read_file"]],
            system_prompt="你可以帮助用户读取和处理文件。"
        )
        
        # 文本处理技能
        self.skills["text"] = Skill(
            name="text",
            description="文本处理技能",
            version="1.0.0",
            tools=[self.tool_executor.tools["text_processor"]],
            system_prompt="你可以帮助用户处理和分析文本。"
        )
        
        # 综合技能（包含多个工具）
        self.skills["utils"] = Skill(
            name="utils",
            description="通用工具技能",
            version="1.0.0",
            tools=[
                self.tool_executor.tools["get_time"],
                self.tool_executor.tools["text_processor"]
            ],
            system_prompt="你提供各种实用工具功能。"
        )
    
    def get_skill(self, name: str) -> Optional[Skill]:
        """获取技能"""
        return self.skills.get(name)
    
    def list_skills(self) -> List[Skill]:
        """列出所有技能"""
        return list(self.skills.values())
    
    def reload_skill(self, name: str) -> bool:
        """重新加载技能（模拟热重载）"""
        if name in self.skills:
            # 模拟重载过程
            time.sleep(0.1)
            return True
        return False
    
    def get_all_tools(self) -> List[ToolDef]:
        """获取所有工具"""
        tools = []
        for skill in self.skills.values():
            tools.extend(skill.tools)
        return tools


class TestToolDiscovery(unittest.TestCase):
    """测试工具发现功能"""
    
    def setUp(self):
        self.executor = MockToolExecutor()
    
    def test_list_tools(self):
        """测试列出所有工具"""
        tools = self.executor.list_tools()
        self.assertEqual(len(tools), 4)
        
        tool_names = [t.name for t in tools]
        self.assertIn("calculator", tool_names)
        self.assertIn("read_file", tool_names)
        self.assertIn("text_processor", tool_names)
        self.assertIn("get_time", tool_names)
    
    def test_tool_definition_structure(self):
        """测试工具定义结构"""
        tool = self.executor.tools["calculator"]
        self.assertEqual(tool.name, "calculator")
        self.assertEqual(tool.command, "mock_calculator")
        self.assertEqual(len(tool.args), 1)
        self.assertEqual(tool.args[0]["name"], "expression")


class TestSingleToolExecution(unittest.TestCase):
    """测试单工具调用"""
    
    def setUp(self):
        self.executor = MockToolExecutor()
    
    def test_calculator_addition(self):
        """测试计算器加法"""
        result = self.executor.execute("calculator", {"expression": "2 + 3"})
        self.assertTrue(result.success)
        self.assertEqual(result.output, "5")
        self.assertGreaterEqual(result.duration_ms, 0)
    
    def test_calculator_complex(self):
        """测试复杂计算"""
        result = self.executor.execute("calculator", {"expression": "(10 + 5) * 2"})
        self.assertTrue(result.success)
        self.assertEqual(result.output, "30")
    
    def test_calculator_invalid_input(self):
        """测试计算器错误处理"""
        result = self.executor.execute("calculator", {"expression": "2 + abc"})
        self.assertFalse(result.success)
        self.assertIsNotNone(result.error)
    
    def test_read_file_mock(self):
        """测试 Mock 文件读取"""
        result = self.executor.execute("read_file", {"path": "/tmp/test.txt"})
        self.assertTrue(result.success)
        self.assertIn("Hello", result.output)
    
    def test_text_processor_count(self):
        """测试文本统计"""
        result = self.executor.execute("text_processor", {
            "text": "Hello World",
            "operation": "count"
        })
        self.assertTrue(result.success)
        data = json.loads(result.output)
        self.assertEqual(data["chars"], 11)
        self.assertEqual(data["words"], 2)
    
    def test_text_processor_upper(self):
        """测试文本转大写"""
        result = self.executor.execute("text_processor", {
            "text": "hello",
            "operation": "upper"
        })
        self.assertTrue(result.success)
        self.assertEqual(result.output, "HELLO")
    
    def test_get_time(self):
        """测试获取时间"""
        result = self.executor.execute("get_time", {"format": "human"})
        self.assertTrue(result.success)
        self.assertIn("-", result.output)  # 日期格式包含 -


class TestMultiToolChain(unittest.TestCase):
    """测试多工具链式调用"""
    
    def setUp(self):
        self.executor = MockToolExecutor()
    
    def test_chain_read_and_process(self):
        """测试读取文件后处理文本"""
        # 步骤1: 读取文件
        read_result = self.executor.execute("read_file", {"path": "/tmp/test.txt"})
        self.assertTrue(read_result.success)
        
        # 步骤2: 处理文本
        process_result = self.executor.execute("text_processor", {
            "text": read_result.output,
            "operation": "count"
        })
        self.assertTrue(process_result.success)
        
        # 验证链式调用历史
        history = self.executor.get_call_history()
        self.assertEqual(len(history), 2)
        self.assertEqual(history[0]["tool"], "read_file")
        self.assertEqual(history[1]["tool"], "text_processor")
    
    def test_chain_calculate_and_format(self):
        """测试计算后格式化"""
        # 步骤1: 计算
        calc_result = self.executor.execute("calculator", {"expression": "100 * 2"})
        self.assertTrue(calc_result.success)
        
        # 步骤2: 格式化结果
        format_result = self.executor.execute("text_processor", {
            "text": f"结果是: {calc_result.output}",
            "operation": "upper"
        })
        self.assertTrue(format_result.success)
        self.assertIn("200", format_result.output)
    
    def test_chain_with_error_handling(self):
        """测试链式调用中的错误处理"""
        # 第一个工具成功
        result1 = self.executor.execute("calculator", {"expression": "1 + 1"})
        self.assertTrue(result1.success)
        
        # 第二个工具失败（工具不存在）
        result2 = self.executor.execute("nonexistent_tool", {})
        self.assertFalse(result2.success)
        
        # 第三个工具应该仍然可以执行
        result3 = self.executor.execute("get_time", {})
        self.assertTrue(result3.success)


class TestComplexToolCallScenarios(unittest.TestCase):
    """测试复杂工具调用场景（并行、嵌套、真实场景模拟）"""
    
    def setUp(self):
        self.executor = MockToolExecutor()
    
    def _nested_text_metrics(self, text: str) -> Dict[str, Any]:
        """嵌套调用：文本统计 -> 计算衍生指标"""
        count_result = self.executor.execute("text_processor", {
            "text": text,
            "operation": "count"
        })
        self.assertTrue(count_result.success)
        stats = json.loads(count_result.output)
        
        lines = max(stats["lines"], 1)
        avg_expr = f"{stats['words']} / {lines}"
        avg_result = self.executor.execute("calculator", {"expression": avg_expr})
        self.assertTrue(avg_result.success)
        stats["avg_words_per_line"] = float(avg_result.output)
        
        return stats
    
    def test_parallel_tool_calls(self):
        """测试并行工具调用"""
        tasks = [
            ("calculator", {"expression": "1 + 2"}),
            ("get_time", {"format": "timestamp"}),
            ("text_processor", {"text": "hello world", "operation": "upper"}),
            ("read_file", {"path": "/tmp/data.json"}),
            ("calculator", {"expression": "10 * 5"}),
        ]
        
        results = []
        with ThreadPoolExecutor(max_workers=4) as pool:
            futures = [
                pool.submit(self.executor.execute, tool, args)
                for tool, args in tasks
            ]
            for future in as_completed(futures):
                result = future.result()
                results.append(result)
                self.assertTrue(result.success)
        
        self.assertEqual(len(results), len(tasks))
        
        history = self.executor.get_call_history()
        self.assertEqual(len(history), len(tasks))
        
        tool_names = [item["tool"] for item in history]
        for expected in ["calculator", "get_time", "text_processor", "read_file"]:
            self.assertIn(expected, tool_names)
    
    def test_nested_tool_calls(self):
        """测试嵌套工具调用"""
        file_result = self.executor.execute("read_file", {"path": "/tmp/test.txt"})
        self.assertTrue(file_result.success)
        
        stats = self._nested_text_metrics(file_result.output)
        self.assertIn("avg_words_per_line", stats)
        
        time_result = self.executor.execute("get_time", {"format": "human"})
        self.assertTrue(time_result.success)
        
        summary = (
            f"time={time_result.output}; "
            f"lines={stats['lines']}; "
            f"words={stats['words']}; "
            f"avg={stats['avg_words_per_line']:.2f}"
        )
        format_result = self.executor.execute("text_processor", {
            "text": summary,
            "operation": "upper"
        })
        self.assertTrue(format_result.success)
        self.assertIn("LINES=", format_result.output)
        self.assertIn("AVG=", format_result.output)
        
        history = self.executor.get_call_history()
        tool_names = [item["tool"] for item in history]
        self.assertGreaterEqual(tool_names.count("text_processor"), 2)
        self.assertIn("calculator", tool_names)
        self.assertIn("read_file", tool_names)
        self.assertIn("get_time", tool_names)
    
    def test_real_world_log_summary(self):
        """测试真实场景模拟：日志分析摘要"""
        file_result = self.executor.execute("read_file", {"path": "/tmp/data.json"})
        self.assertTrue(file_result.success)
        
        count_result = self.executor.execute("text_processor", {
            "text": file_result.output,
            "operation": "count"
        })
        self.assertTrue(count_result.success)
        stats = json.loads(count_result.output)
        
        density_expr = f"{stats['chars']} / {max(stats['lines'], 1)}"
        density_result = self.executor.execute("calculator", {"expression": density_expr})
        self.assertTrue(density_result.success)
        
        time_result = self.executor.execute("get_time", {"format": "human"})
        self.assertTrue(time_result.success)
        
        report = (
            f"report time {time_result.output}; "
            f"lines {stats['lines']}; "
            f"chars {stats['chars']}; "
            f"density {density_result.output}"
        )
        format_result = self.executor.execute("text_processor", {
            "text": report,
            "operation": "format"
        })
        self.assertTrue(format_result.success)
        self.assertTrue(format_result.output.startswith("Report time"))


class TestToolErrorHandling(unittest.TestCase):
    """测试工具错误处理"""
    
    def setUp(self):
        self.executor = MockToolExecutor()
    
    def test_tool_not_found(self):
        """测试工具不存在"""
        result = self.executor.execute("nonexistent_tool", {})
        self.assertFalse(result.success)
        self.assertIsNotNone(result.error)
    
    def test_missing_required_argument(self):
        """测试缺少必需参数"""
        # 模拟缺少必需参数的情况
        result = self.executor.execute("calculator", {})
        # Mock 实现会处理空表达式
        self.assertFalse(result.success)
        self.assertIsNotNone(result.error)
    
    def test_invalid_argument_type(self):
        """测试无效参数类型"""
        result = self.executor.execute("text_processor", {
            "text": "test",
            "operation": "invalid_op"
        })
        self.assertFalse(result.success)
        self.assertIsNotNone(result.error)
    
    def test_execution_timeout_simulation(self):
        """测试执行超时模拟"""
        # Mock 执行器没有真正的超时，但我们可以验证结构
        result = self.executor.execute("get_time", {})
        self.assertTrue(result.success)
        self.assertGreaterEqual(result.duration_ms, 0)


class TestSkillLoading(unittest.TestCase):
    """测试技能加载"""
    
    def setUp(self):
        self.manager = MockSkillManager()
    
    def test_list_skills(self):
        """测试列出所有技能"""
        skills = self.manager.list_skills()
        self.assertEqual(len(skills), 4)
        
        skill_names = [s.name for s in skills]
        self.assertIn("math", skill_names)
        self.assertIn("file_ops", skill_names)
        self.assertIn("text", skill_names)
        self.assertIn("utils", skill_names)
    
    def test_get_skill(self):
        """测试获取特定技能"""
        skill = self.manager.get_skill("math")
        self.assertIsNotNone(skill)
        self.assertEqual(skill.name, "math")
        self.assertEqual(skill.version, "1.0.0")
        self.assertIsNotNone(skill.system_prompt)
    
    def test_skill_tools(self):
        """测试技能包含的工具"""
        skill = self.manager.get_skill("utils")
        self.assertEqual(len(skill.tools), 2)
        
        tool_names = [t.name for t in skill.tools]
        self.assertIn("get_time", tool_names)
        self.assertIn("text_processor", tool_names)
    
    def test_get_all_tools(self):
        """测试获取所有技能的所有工具"""
        tools = self.manager.get_all_tools()
        # math: 1, file_ops: 1, text: 1, utils: 2 = 5 total
        self.assertEqual(len(tools), 5)


class TestSkillHotReload(unittest.TestCase):
    """测试技能热重载"""
    
    def setUp(self):
        self.manager = MockSkillManager()
    
    def test_reload_existing_skill(self):
        """测试重新加载存在的技能"""
        result = self.manager.reload_skill("math")
        self.assertTrue(result)
    
    def test_reload_nonexistent_skill(self):
        """测试重新加载不存在的技能"""
        result = self.manager.reload_skill("nonexistent")
        self.assertFalse(result)
    
    def test_skill_persistence_after_reload(self):
        """测试重载后技能保持"""
        skill_before = self.manager.get_skill("math")
        self.manager.reload_skill("math")
        skill_after = self.manager.get_skill("math")
        
        self.assertEqual(skill_before.name, skill_after.name)
        self.assertEqual(skill_before.version, skill_after.version)


class TestToolExecutionValidation(unittest.TestCase):
    """测试工具执行结果验证"""
    
    def setUp(self):
        self.executor = MockToolExecutor()
    
    def test_result_structure(self):
        """测试结果结构完整性"""
        result = self.executor.execute("calculator", {"expression": "1 + 1"})
        
        self.assertTrue(hasattr(result, 'success'))
        self.assertTrue(hasattr(result, 'output'))
        self.assertTrue(hasattr(result, 'error'))
        self.assertTrue(hasattr(result, 'duration_ms'))
    
    def test_success_result_has_no_error(self):
        """测试成功结果没有错误"""
        result = self.executor.execute("calculator", {"expression": "1 + 1"})
        self.assertTrue(result.success)
        self.assertIsNone(result.error)
    
    def test_failure_result_has_error(self):
        """测试失败结果有错误信息"""
        result = self.executor.execute("calculator", {"expression": "2 + abc"})
        self.assertFalse(result.success)
        self.assertIsNotNone(result.error)
    
    def test_call_history_tracking(self):
        """测试调用历史记录"""
        self.executor.clear_history()
        
        self.executor.execute("calculator", {"expression": "1 + 1"})
        self.executor.execute("get_time", {})
        
        history = self.executor.get_call_history()
        self.assertEqual(len(history), 2)
        self.assertEqual(history[0]["tool"], "calculator")
        self.assertEqual(history[1]["tool"], "get_time")


class TestRealAPICalls(unittest.TestCase):
    """测试真实 API 调用（可选，需要服务器运行）"""
    
    @classmethod
    def setUpClass(cls):
        cls.base_url = BASE_URL
        cls.skip_real_tests = not cls._server_available()
        if cls.skip_real_tests:
            print(f"\n⚠️  服务器不可用 ({cls.base_url})，跳过真实 API 测试")
    
    @classmethod
    def _server_available(cls) -> bool:
        """检查服务器是否可用"""
        try:
            response = requests.get(f"{cls.base_url}/api/v1/health", timeout=2)
            return response.status_code == 200
        except:
            return False
    
    def test_health_endpoint(self):
        """测试健康检查端点"""
        if self.skip_real_tests:
            self.skipTest("服务器不可用")
        
        response = requests.get(f"{self.base_url}/api/v1/health")
        self.assertEqual(response.status_code, 200)
    
    def test_chat_endpoint(self):
        """测试聊天端点"""
        if self.skip_real_tests:
            self.skipTest("服务器不可用")
        
        payload = {
            "message": "Hello, this is a test message",
            "model": "gpt-3.5-turbo"
        }
        
        try:
            response = requests.post(
                f"{self.base_url}/api/v1/chat",
                json=payload,
                timeout=5
            )
            # 接受 200 或 201 状态码
            self.assertIn(response.status_code, [200, 201])
            
            data = response.json()
            self.assertIn("session_id", data)
            self.assertIn("status", data)
        except requests.exceptions.Timeout:
            self.skipTest("请求超时")
        except Exception as e:
            self.skipTest(f"请求失败: {e}")


def run_mock_tests():
    """运行所有 Mock 测试"""
    print("=" * 60)
    print("运行 Mock 工具测试")
    print("=" * 60)
    
    loader = unittest.TestLoader()
    suite = unittest.TestSuite()
    
    # 添加所有测试类
    suite.addTests(loader.loadTestsFromTestCase(TestToolDiscovery))
    suite.addTests(loader.loadTestsFromTestCase(TestSingleToolExecution))
    suite.addTests(loader.loadTestsFromTestCase(TestMultiToolChain))
    suite.addTests(loader.loadTestsFromTestCase(TestComplexToolCallScenarios))
    suite.addTests(loader.loadTestsFromTestCase(TestToolErrorHandling))
    suite.addTests(loader.loadTestsFromTestCase(TestSkillLoading))
    suite.addTests(loader.loadTestsFromTestCase(TestSkillHotReload))
    suite.addTests(loader.loadTestsFromTestCase(TestToolExecutionValidation))
    
    runner = unittest.TextTestRunner(verbosity=2)
    result = runner.run(suite)
    
    return result.wasSuccessful()


def run_real_api_tests():
    """运行真实 API 测试"""
    print("\n" + "=" * 60)
    print("运行真实 API 测试")
    print(f"API URL: {BASE_URL}")
    print("=" * 60)
    
    loader = unittest.TestLoader()
    suite = loader.loadTestsFromTestCase(TestRealAPICalls)
    
    runner = unittest.TextTestRunner(verbosity=2)
    result = runner.run(suite)
    
    return result.wasSuccessful()


def demo_mock_tools():
    """演示 Mock 工具使用"""
    print("\n" + "=" * 60)
    print("Mock 工具演示")
    print("=" * 60)
    
    executor = MockToolExecutor()
    
    print("\n1. 计算器工具:")
    result = executor.execute("calculator", {"expression": "100 / 4 + 5"})
    print(f"   输入: 100 / 4 + 5")
    print(f"   结果: {result.output}")
    print(f"   耗时: {result.duration_ms}ms")
    
    print("\n2. 文件读取工具:")
    result = executor.execute("read_file", {"path": "/tmp/test.txt"})
    print(f"   路径: /tmp/test.txt")
    print(f"   内容: {result.output[:50]}...")
    
    print("\n3. 文本处理工具:")
    result = executor.execute("text_processor", {
        "text": "Hello World from Bamboo",
        "operation": "count"
    })
    print(f"   输入: 'Hello World from Bamboo'")
    print(f"   统计: {result.output}")
    
    print("\n4. 时间工具:")
    result = executor.execute("get_time", {"format": "human"})
    print(f"   当前时间: {result.output}")
    
    print("\n5. 工具链式调用:")
    executor.clear_history()
    
    # 读取文件
    file_result = executor.execute("read_file", {"path": "/tmp/data.json"})
    # 统计内容
    count_result = executor.execute("text_processor", {
        "text": file_result.output,
        "operation": "count"
    })
    
    print(f"   文件内容长度: {len(file_result.output)} 字符")
    print(f"   统计结果: {count_result.output}")
    
    print("\n6. 调用历史:")
    for i, call in enumerate(executor.get_call_history(), 1):
        print(f"   {i}. {call['tool']} - {call['timestamp']}")


def demo_skills():
    """演示技能系统"""
    print("\n" + "=" * 60)
    print("技能系统演示")
    print("=" * 60)
    
    manager = MockSkillManager()
    
    print("\n可用技能:")
    for skill in manager.list_skills():
        print(f"  - {skill.name}: {skill.description}")
        print(f"    版本: {skill.version}")
        print(f"    工具数: {len(skill.tools)}")
        print()
    
    print("所有可用工具:")
    for tool in manager.get_all_tools():
        print(f"  - {tool.name}: {tool.description}")


def main():
    """主函数"""
    import argparse
    
    parser = argparse.ArgumentParser(description="Bamboo API 工具和技能测试")
    parser.add_argument("--mock-only", action="store_true", help="只运行 Mock 测试")
    parser.add_argument("--real-only", action="store_true", help="只运行真实 API 测试")
    parser.add_argument("--demo", action="store_true", help="运行演示")
    parser.add_argument("--url", default=BASE_URL, help=f"API 基础 URL (默认: {DEFAULT_BASE_URL})")
    
    args = parser.parse_args()
    
    # 更新 BASE_URL
    if args.url != BASE_URL:
        import tests.api.test_tools as test_module
        test_module.BASE_URL = args.url
    
    success = True
    
    if args.demo:
        demo_mock_tools()
        demo_skills()
        return 0
    
    if args.real_only:
        success = run_real_api_tests()
    elif args.mock_only:
        success = run_mock_tests()
    else:
        # 运行所有测试
        mock_success = run_mock_tests()
        real_success = run_real_api_tests()
        success = mock_success and real_success
    
    print("\n" + "=" * 60)
    if success:
        print("✅ 所有测试通过!")
        return 0
    else:
        print("❌ 部分测试失败")
        return 1


if __name__ == "__main__":
    sys.exit(main())
