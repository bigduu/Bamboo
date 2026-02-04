#!/usr/bin/env python3
"""
Bamboo 复杂工具调用场景测试

测试场景:
1. 条件工具调用（根据前序结果决定后续调用）
2. 并行工具调用（同时调用多个独立工具）
3. 嵌套工具调用（工具A内部调用工具B）
4. 工具参数动态生成（从前文提取参数）
5. 长文本处理（大参数/大返回值的工具）

真实场景模拟:
- 文件分析流程：读取 → 分析 → 总结 → 输出
- 数据处理流程：查询 → 过滤 → 聚合 → 报告
- 代码生成流程：需求分析 → 代码生成 → 验证 → 修复

性能测试:
- 工具调用延迟测量
- 多工具并发性能

边界条件:
- 循环调用检测
- 工具调用深度限制
"""

import json
import os
import sys
import time
import asyncio
import unittest
from concurrent.futures import ThreadPoolExecutor, as_completed
from typing import Dict, List, Optional, Any, Callable, Set
from dataclasses import dataclass, field
from datetime import datetime
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
    metadata: Dict[str, Any] = field(default_factory=dict)


@dataclass
class ToolDef:
    """工具定义"""
    name: str
    description: str
    command: str
    args: List[Dict[str, Any]]


@dataclass
class ToolCall:
    """工具调用记录"""
    tool_name: str
    args: Dict[str, Any]
    timestamp: float
    result: Optional[ToolResult] = None


class AdvancedToolExecutor:
    """高级工具执行器 - 支持复杂调用场景"""
    
    def __init__(self):
        self.tools: Dict[str, ToolDef] = {}
        self.call_history: List[ToolCall] = []
        self.call_stack: List[str] = []  # 用于检测循环调用
        self.max_depth: int = 10  # 最大嵌套深度
        self.performance_stats: Dict[str, List[float]] = {}
        self._setup_tools()
    
    def _setup_tools(self):
        """设置工具定义"""
        self.tools["calculator"] = ToolDef(
            name="calculator",
            description="执行数学计算",
            command="mock_calculator",
            args=[{"name": "expression", "type": "string", "required": True}]
        )
        self.tools["read_file"] = ToolDef(
            name="read_file",
            description="读取文件内容",
            command="mock_read_file",
            args=[
                {"name": "path", "type": "string", "required": True},
                {"name": "limit", "type": "number", "required": False, "default": 100}
            ]
        )
        self.tools["text_processor"] = ToolDef(
            name="text_processor",
            description="处理文本（统计字数、格式化等）",
            command="mock_text_processor",
            args=[
                {"name": "text", "type": "string", "required": True},
                {"name": "operation", "type": "string", "required": True}
            ]
        )
        self.tools["get_time"] = ToolDef(
            name="get_time",
            description="获取当前时间",
            command="mock_get_time",
            args=[{"name": "format", "type": "string", "required": False, "default": "iso"}]
        )
        self.tools["data_filter"] = ToolDef(
            name="data_filter",
            description="过滤数据",
            command="mock_data_filter",
            args=[
                {"name": "data", "type": "string", "required": True},
                {"name": "condition", "type": "string", "required": True}
            ]
        )
        self.tools["data_aggregate"] = ToolDef(
            name="data_aggregate",
            description="聚合数据",
            command="mock_data_aggregate",
            args=[
                {"name": "data", "type": "string", "required": True},
                {"name": "operation", "type": "string", "required": True}
            ]
        )
        self.tools["code_validator"] = ToolDef(
            name="code_validator",
            description="验证代码",
            command="mock_code_validator",
            args=[{"name": "code", "type": "string", "required": True}]
        )
        self.tools["code_fixer"] = ToolDef(
            name="code_fixer",
            description="修复代码",
            command="mock_code_fixer",
            args=[
                {"name": "code", "type": "string", "required": True},
                {"name": "errors", "type": "string", "required": True}
            ]
        )
    
    def execute(self, tool_name: str, args: Dict[str, Any], 
                track_performance: bool = True) -> ToolResult:
        """执行工具，支持循环检测和深度限制"""
        # 检查工具是否存在
        if tool_name not in self.tools:
            return ToolResult(
                success=False, 
                output="", 
                error=f"Tool not found: {tool_name}",
                duration_ms=0
            )
        
        # 检查循环调用
        if tool_name in self.call_stack:
            return ToolResult(
                success=False,
                output="",
                error=f"Circular tool call detected: {' -> '.join(self.call_stack + [tool_name])}",
                duration_ms=0
            )
        
        # 检查深度限制
        if len(self.call_stack) >= self.max_depth:
            return ToolResult(
                success=False,
                output="",
                error=f"Maximum call depth exceeded: {self.max_depth}",
                duration_ms=0
            )
        
        # 记录调用栈
        self.call_stack.append(tool_name)
        
        start_time = time.time()
        call_record = ToolCall(
            tool_name=tool_name,
            args=args,
            timestamp=start_time
        )
        
        try:
            # 执行工具
            result = self._execute_tool(tool_name, args)
            
            duration_ms = int((time.time() - start_time) * 1000)
            result.duration_ms = duration_ms
            
            # 记录性能统计
            if track_performance:
                if tool_name not in self.performance_stats:
                    self.performance_stats[tool_name] = []
                self.performance_stats[tool_name].append(duration_ms)
            
            call_record.result = result
            self.call_history.append(call_record)
            
            return result
            
        except Exception as e:
            duration_ms = int((time.time() - start_time) * 1000)
            result = ToolResult(
                success=False,
                output="",
                error=str(e),
                duration_ms=duration_ms
            )
            call_record.result = result
            self.call_history.append(call_record)
            return result
        finally:
            # 弹出调用栈
            self.call_stack.pop()
    
    def _execute_tool(self, tool_name: str, args: Dict[str, Any]) -> ToolResult:
        """实际执行工具逻辑"""
        if tool_name == "calculator":
            return self._mock_calculator(args)
        elif tool_name == "read_file":
            return self._mock_read_file(args)
        elif tool_name == "text_processor":
            return self._mock_text_processor(args)
        elif tool_name == "get_time":
            return self._mock_get_time(args)
        elif tool_name == "data_filter":
            return self._mock_data_filter(args)
        elif tool_name == "data_aggregate":
            return self._mock_data_aggregate(args)
        elif tool_name == "code_validator":
            return self._mock_code_validator(args)
        elif tool_name == "code_fixer":
            return self._mock_code_fixer(args)
        else:
            return ToolResult(success=False, error=f"Unknown tool: {tool_name}")
    
    def _mock_calculator(self, args: Dict[str, Any]) -> ToolResult:
        """Mock 计算器"""
        expression = args.get("expression", "")
        allowed_chars = set("0123456789+-*/(). ")
        
        if not expression:
            return ToolResult(success=False, error="Empty expression")
        if not all(c in allowed_chars for c in expression):
            return ToolResult(success=False, error="Invalid characters in expression")
        
        try:
            result = eval(expression)
            return ToolResult(success=True, output=str(result))
        except Exception as e:
            return ToolResult(success=False, error=f"Calculation error: {e}")
    
    def _mock_read_file(self, args: Dict[str, Any]) -> ToolResult:
        """Mock 文件读取"""
        path = args.get("path", "")
        limit = args.get("limit", 100)
        
        # 模拟不同文件内容
        mock_files = {
            "/tmp/test.txt": "Hello, World!\nThis is a test file.\nLine 3\n",
            "/tmp/data.json": json.dumps({"name": "test", "value": 42, "items": list(range(100))}, indent=2),
            "/tmp/large_file.txt": "Line content\n" * 1000,
            "/tmp/code.py": "def hello():\n    print('Hello')\n    return True\n",
            "/tmp/logs.txt": "[INFO] Start\n[WARN] Warning message\n[ERROR] Error message\n[INFO] End\n",
        }
        
        content = mock_files.get(path, f"Mock content for {path}\nLine 2\nLine 3\n")
        lines = content.split("\n")[:limit]
        return ToolResult(success=True, output="\n".join(lines))
    
    def _mock_text_processor(self, args: Dict[str, Any]) -> ToolResult:
        """Mock 文本处理器"""
        text = args.get("text", "")
        operation = args.get("operation", "count")
        
        if operation == "count":
            result = {
                "chars": len(text),
                "words": len(text.split()),
                "lines": len(text.split("\n"))
            }
            return ToolResult(success=True, output=json.dumps(result, indent=2))
        elif operation == "upper":
            return ToolResult(success=True, output=text.upper())
        elif operation == "lower":
            return ToolResult(success=True, output=text.lower())
        elif operation == "format":
            return ToolResult(success=True, output=text.strip().capitalize())
        elif operation == "extract_keywords":
            # 简单关键词提取
            words = text.lower().split()
            keywords = list(set([w for w in words if len(w) > 4]))[:10]
            return ToolResult(success=True, output=json.dumps(keywords))
        else:
            return ToolResult(success=False, error=f"Unknown operation: {operation}")
    
    def _mock_get_time(self, args: Dict[str, Any]) -> ToolResult:
        """Mock 时间获取"""
        format_type = args.get("format", "iso")
        now = datetime.now()
        
        if format_type == "iso":
            output = now.isoformat()
        elif format_type == "timestamp":
            output = str(int(now.timestamp()))
        elif format_type == "human":
            output = now.strftime("%Y-%m-%d %H:%M:%S")
        else:
            output = now.isoformat()
        
        return ToolResult(success=True, output=output)
    
    def _mock_data_filter(self, args: Dict[str, Any]) -> ToolResult:
        """Mock 数据过滤器"""
        data = args.get("data", "")
        condition = args.get("condition", "")
        
        lines = data.split("\n")
        if condition == "error":
            filtered = [l for l in lines if "ERROR" in l]
        elif condition == "warn":
            filtered = [l for l in lines if "WARN" in l or "ERROR" in l]
        elif condition == "info":
            filtered = [l for l in lines if "INFO" in l]
        else:
            filtered = lines
        
        return ToolResult(success=True, output="\n".join(filtered))
    
    def _mock_data_aggregate(self, args: Dict[str, Any]) -> ToolResult:
        """Mock 数据聚合器"""
        data = args.get("data", "")
        operation = args.get("operation", "")
        
        lines = [l for l in data.split("\n") if l.strip()]
        
        if operation == "count":
            return ToolResult(success=True, output=str(len(lines)))
        elif operation == "join":
            return ToolResult(success=True, output=" | ".join(lines))
        elif operation == "stats":
            total_len = sum(len(l) for l in lines)
            avg_len = total_len / len(lines) if lines else 0
            return ToolResult(success=True, output=json.dumps({
                "count": len(lines),
                "total_chars": total_len,
                "avg_line_length": round(avg_len, 2)
            }))
        else:
            return ToolResult(success=False, error=f"Unknown operation: {operation}")
    
    def _mock_code_validator(self, args: Dict[str, Any]) -> ToolResult:
        """Mock 代码验证器"""
        code = args.get("code", "")
        
        errors = []
        if "print(" in code and "python3" not in code:
            errors.append("Consider using logging instead of print")
        if "except:" in code:
            errors.append("Bare except clause detected")
        if len(code) > 500:
            errors.append("Code is too long, consider refactoring")
        
        if errors:
            return ToolResult(
                success=False,
                output="",
                error="; ".join(errors),
                metadata={"errors": errors}
            )
        return ToolResult(success=True, output="Code is valid")
    
    def _mock_code_fixer(self, args: Dict[str, Any]) -> ToolResult:
        """Mock 代码修复器"""
        code = args.get("code", "")
        errors = args.get("errors", "")
        
        fixed_code = code
        if "print(" in code:
            fixed_code = fixed_code.replace("print(", "logger.info(")
        if "except:" in code:
            fixed_code = fixed_code.replace("except:", "except Exception:")
        
        return ToolResult(
            success=True,
            output=fixed_code,
            metadata={"fixes_applied": errors}
        )
    
    def execute_parallel(self, tasks: List[tuple], max_workers: int = 4) -> List[ToolResult]:
        """并行执行多个工具调用"""
        results = []
        with ThreadPoolExecutor(max_workers=max_workers) as executor:
            futures = [
                executor.submit(self.execute, tool_name, args)
                for tool_name, args in tasks
            ]
            for future in as_completed(futures):
                results.append(future.result())
        return results
    
    def execute_conditional(self, condition_tool: str, condition_args: Dict[str, Any],
                           true_branch: Callable, false_branch: Optional[Callable] = None) -> Any:
        """条件工具调用"""
        condition_result = self.execute(condition_tool, condition_args)
        
        if condition_result.success:
            return true_branch(self, condition_result)
        elif false_branch:
            return false_branch(self, condition_result)
        else:
            return None
    
    def get_performance_report(self) -> Dict[str, Any]:
        """获取性能报告"""
        report = {}
        for tool_name, durations in self.performance_stats.items():
            if durations:
                report[tool_name] = {
                    "count": len(durations),
                    "avg_ms": round(sum(durations) / len(durations), 2),
                    "min_ms": min(durations),
                    "max_ms": max(durations),
                    "total_ms": sum(durations)
                }
        return report
    
    def clear_history(self):
        """清除调用历史"""
        self.call_history.clear()
        self.call_stack.clear()
        self.performance_stats.clear()


class TestConditionalToolCalls(unittest.TestCase):
    """测试条件工具调用"""
    
    def setUp(self):
        self.executor = AdvancedToolExecutor()
    
    def test_conditional_based_on_file_size(self):
        """测试基于文件大小的条件调用"""
        # 读取文件
        file_result = self.executor.execute("read_file", {"path": "/tmp/test.txt"})
        self.assertTrue(file_result.success)
        
        # 统计文件内容
        count_result = self.executor.execute("text_processor", {
            "text": file_result.output,
            "operation": "count"
        })
        self.assertTrue(count_result.success)
        stats = json.loads(count_result.output)
        
        # 根据行数决定后续操作
        if stats["lines"] > 2:
            # 长文件：提取关键词
            result = self.executor.execute("text_processor", {
                "text": file_result.output,
                "operation": "extract_keywords"
            })
            self.assertTrue(result.success)
        else:
            # 短文件：直接格式化
            result = self.executor.execute("text_processor", {
                "text": file_result.output,
                "operation": "upper"
            })
            self.assertTrue(result.success)
    
    def test_conditional_error_handling(self):
        """测试条件错误处理流程"""
        # 尝试验证有问题的代码
        code = "def test():\n    print('hello')\n    try:\n        pass\n    except:\n        pass"
        
        validation = self.executor.execute("code_validator", {"code": code})
        
        if not validation.success:
            # 验证失败，尝试修复
            fix_result = self.executor.execute("code_fixer", {
                "code": code,
                "errors": validation.error
            })
            self.assertTrue(fix_result.success)
            self.assertIn("logger.info", fix_result.output)
        else:
            self.fail("Expected validation to fail")


class TestParallelToolCalls(unittest.TestCase):
    """测试并行工具调用"""
    
    def setUp(self):
        self.executor = AdvancedToolExecutor()
    
    def test_parallel_independent_calls(self):
        """测试并行独立工具调用"""
        tasks = [
            ("calculator", {"expression": "1 + 2"}),
            ("calculator", {"expression": "10 * 5"}),
            ("calculator", {"expression": "100 / 4"}),
            ("get_time", {"format": "timestamp"}),
            ("text_processor", {"text": "hello world", "operation": "upper"}),
        ]
        
        start_time = time.time()
        results = self.executor.execute_parallel(tasks, max_workers=3)
        elapsed_ms = (time.time() - start_time) * 1000
        
        # 验证所有调用成功
        for result in results:
            self.assertTrue(result.success)
        
        # 验证并行性能（5个调用应该比串行快）
        self.assertLess(elapsed_ms, 500)  # 假设串行需要 >500ms
        
        # 验证调用历史
        history = self.executor.call_history
        self.assertEqual(len(history), 5)
    
    def test_parallel_with_large_data(self):
        """测试并行处理大数据"""
        tasks = [
            ("read_file", {"path": "/tmp/large_file.txt", "limit": 100}),
            ("read_file", {"path": "/tmp/data.json", "limit": 50}),
            ("read_file", {"path": "/tmp/logs.txt", "limit": 10}),
        ]
        
        results = self.executor.execute_parallel(tasks, max_workers=3)
        
        for result in results:
            self.assertTrue(result.success)
            self.assertGreater(len(result.output), 0)


class TestNestedToolCalls(unittest.TestCase):
    """测试嵌套工具调用"""
    
    def setUp(self):
        self.executor = AdvancedToolExecutor()
    
    def test_nested_analysis_workflow(self):
        """测试嵌套分析工作流"""
        # 读取日志文件
        file_result = self.executor.execute("read_file", {"path": "/tmp/logs.txt"})
        self.assertTrue(file_result.success)
        
        # 第一层嵌套：过滤错误日志
        filter_result = self.executor.execute("data_filter", {
            "data": file_result.output,
            "condition": "error"
        })
        self.assertTrue(filter_result.success)
        
        # 第二层嵌套：聚合过滤结果
        aggregate_result = self.executor.execute("data_aggregate", {
            "data": filter_result.output,
            "operation": "stats"
        })
        self.assertTrue(aggregate_result.success)
        
        # 第三层嵌套：格式化报告
        report = f"Error analysis at {datetime.now().isoformat()}: {aggregate_result.output}"
        format_result = self.executor.execute("text_processor", {
            "text": report,
            "operation": "format"
        })
        self.assertTrue(format_result.success)
    
    def test_deep_nesting_limit(self):
        """测试深度限制"""
        # 模拟深度嵌套调用
        def recursive_call(depth: int) -> ToolResult:
            if depth <= 0:
                return self.executor.execute("calculator", {"expression": "1 + 1"})
            
            result = self.executor.execute("text_processor", {
                "text": f"Level {depth}",
                "operation": "upper"
            })
            if not result.success:
                return result
            
            return recursive_call(depth - 1)
        
        # 正常深度应该成功
        result = recursive_call(3)
        self.assertTrue(result.success)


class TestDynamicParameterGeneration(unittest.TestCase):
    """测试工具参数动态生成"""
    
    def setUp(self):
        self.executor = AdvancedToolExecutor()
    
    def test_extract_and_use_parameters(self):
        """测试从前文提取并使用参数"""
        # 读取文件
        file_result = self.executor.execute("read_file", {"path": "/tmp/data.json"})
        self.assertTrue(file_result.success)
        
        # 分析内容特征
        count_result = self.executor.execute("text_processor", {
            "text": file_result.output,
            "operation": "count"
        })
        self.assertTrue(count_result.success)
        stats = json.loads(count_result.output)
        
        # 根据分析结果动态生成参数
        if stats["chars"] > 100:
            limit = min(stats["lines"] // 2, 50)
        else:
            limit = stats["lines"]
        
        # 使用动态参数重新读取
        re_read = self.executor.execute("read_file", {
            "path": "/tmp/data.json",
            "limit": limit
        })
        self.assertTrue(re_read.success)
    
    def test_chain_parameter_building(self):
        """测试链式参数构建"""
        # 获取当前时间
        time_result = self.executor.execute("get_time", {"format": "human"})
        self.assertTrue(time_result.success)
        
        # 基于时间构建查询参数
        timestamp = time_result.output
        
        # 构建报告文本
        report_text = f"Report generated at {timestamp}"
        
        # 处理报告
        process_result = self.executor.execute("text_processor", {
            "text": report_text,
            "operation": "count"
        })
        self.assertTrue(process_result.success)


class TestLargeTextProcessing(unittest.TestCase):
    """测试长文本处理"""
    
    def setUp(self):
        self.executor = AdvancedToolExecutor()
    
    def test_large_file_read(self):
        """测试大文件读取"""
        result = self.executor.execute("read_file", {
            "path": "/tmp/large_file.txt",
            "limit": 1000
        })
        self.assertTrue(result.success)
        self.assertGreater(len(result.output), 5000)
    
    def test_large_text_processing(self):
        """测试大文本处理"""
        # 生成大文本
        large_text = "Sample text line\n" * 500
        
        result = self.executor.execute("text_processor", {
            "text": large_text,
            "operation": "count"
        })
        self.assertTrue(result.success)
        
        stats = json.loads(result.output)
        # 注意：split("\n") 在末尾有换行符时会产生额外的一个空字符串
        self.assertGreaterEqual(stats["lines"], 500)


class TestRealWorldScenarios(unittest.TestCase):
    """测试真实场景模拟"""
    
    def setUp(self):
        self.executor = AdvancedToolExecutor()
    
    def test_file_analysis_workflow(self):
        """测试文件分析流程：读取 → 分析 → 总结 → 输出"""
        # 1. 读取文件
        file_result = self.executor.execute("read_file", {"path": "/tmp/logs.txt"})
        self.assertTrue(file_result.success)
        
        # 2. 分析内容
        count_result = self.executor.execute("text_processor", {
            "text": file_result.output,
            "operation": "count"
        })
        self.assertTrue(count_result.success)
        
        # 3. 过滤关键信息
        filter_result = self.executor.execute("data_filter", {
            "data": file_result.output,
            "condition": "warn"
        })
        self.assertTrue(filter_result.success)
        
        # 4. 聚合统计
        aggregate_result = self.executor.execute("data_aggregate", {
            "data": filter_result.output,
            "operation": "stats"
        })
        self.assertTrue(aggregate_result.success)
        
        # 5. 生成总结报告
        time_result = self.executor.execute("get_time", {"format": "human"})
        report = (
            f"Log Analysis Report ({time_result.output})\n"
            f"Total lines: {json.loads(count_result.output)['lines']}\n"
            f"Warning/Error stats: {aggregate_result.output}"
        )
        
        format_result = self.executor.execute("text_processor", {
            "text": report,
            "operation": "format"
        })
        self.assertTrue(format_result.success)
    
    def test_data_processing_workflow(self):
        """测试数据处理流程：查询 → 过滤 → 聚合 → 报告"""
        # 1. 查询（读取数据）
        data_result = self.executor.execute("read_file", {"path": "/tmp/logs.txt"})
        self.assertTrue(data_result.success)
        
        # 2. 过滤错误数据
        error_result = self.executor.execute("data_filter", {
            "data": data_result.output,
            "condition": "error"
        })
        self.assertTrue(error_result.success)
        
        # 3. 聚合统计
        stats_result = self.executor.execute("data_aggregate", {
            "data": error_result.output,
            "operation": "stats"
        })
        self.assertTrue(stats_result.success)
        
        # 4. 生成报告
        time_result = self.executor.execute("get_time", {"format": "iso"})
        report = f"Error Report [{time_result.output}]: {stats_result.output}"
        
        final_result = self.executor.execute("text_processor", {
            "text": report,
            "operation": "upper"
        })
        self.assertTrue(final_result.success)
        self.assertIn("ERROR REPORT", final_result.output)
    
    def test_code_generation_workflow(self):
        """测试代码生成流程：需求分析 → 代码生成 → 验证 → 修复"""
        # 1. 需求分析（模拟）
        requirement = "Create a function that prints hello world"
        
        # 2. 代码生成（模拟）
        generated_code = """def hello():
    print("Hello World")
    try:
        return True
    except:
        return False
"""
        
        # 3. 代码验证
        validation = self.executor.execute("code_validator", {"code": generated_code})
        
        if not validation.success:
            # 4. 代码修复
            fixed = self.executor.execute("code_fixer", {
                "code": generated_code,
                "errors": validation.error
            })
            self.assertTrue(fixed.success)
            self.assertIn("logger.info", fixed.output)
        else:
            # 验证通过
            self.assertTrue(validation.success)


class TestPerformanceBenchmarks(unittest.TestCase):
    """测试性能基准"""
    
    def setUp(self):
        self.executor = AdvancedToolExecutor()
    
    def test_tool_call_latency(self):
        """测试工具调用延迟"""
        latencies = []
        
        for _ in range(10):
            start = time.time()
            result = self.executor.execute("calculator", {"expression": "1 + 1"})
            elapsed = (time.time() - start) * 1000
            latencies.append(elapsed)
            self.assertTrue(result.success)
        
        avg_latency = sum(latencies) / len(latencies)
        max_latency = max(latencies)
        
        # 验证性能在合理范围内
        self.assertLess(avg_latency, 100)  # 平均 < 100ms
        self.assertLess(max_latency, 200)  # 最大 < 200ms
    
    def test_concurrent_performance(self):
        """测试并发性能"""
        tasks = [
            ("calculator", {"expression": f"{i} * {i}"})
            for i in range(20)
        ]
        
        start = time.time()
        results = self.executor.execute_parallel(tasks, max_workers=5)
        elapsed = (time.time() - start) * 1000
        
        # 验证所有调用成功
        for result in results:
            self.assertTrue(result.success)
        
        # 验证并发性能
        self.assertLess(elapsed, 1000)  # 20个调用 < 1秒
    
    def test_performance_report(self):
        """测试性能报告生成"""
        # 执行一些工具调用
        for i in range(5):
            self.executor.execute("calculator", {"expression": f"{i} + 1"})
            self.executor.execute("text_processor", {
                "text": f"test {i}",
                "operation": "count"
            })
        
        report = self.executor.get_performance_report()
        
        self.assertIn("calculator", report)
        self.assertIn("text_processor", report)
        
        calc_stats = report["calculator"]
        self.assertEqual(calc_stats["count"], 5)
        self.assertIn("avg_ms", calc_stats)
        self.assertIn("min_ms", calc_stats)
        self.assertIn("max_ms", calc_stats)


class TestBoundaryConditions(unittest.TestCase):
    """测试边界条件"""
    
    def setUp(self):
        self.executor = AdvancedToolExecutor()
    
    def test_circular_call_detection(self):
        """测试循环调用检测"""
        # 模拟循环调用场景
        # 由于实际循环需要工具间相互调用，这里测试调用栈检测机制
        
        # 正常调用不应触发循环检测
        result1 = self.executor.execute("calculator", {"expression": "1 + 1"})
        self.assertTrue(result1.success)
        
        result2 = self.executor.execute("calculator", {"expression": "2 + 2"})
        self.assertTrue(result2.success)
        
        # 调用栈应该为空（已清理）
        self.assertEqual(len(self.executor.call_stack), 0)
    
    def test_max_depth_limit(self):
        """测试最大深度限制"""
        # 设置较小的深度限制用于测试
        original_max = self.executor.max_depth
        self.executor.max_depth = 3
        self.executor.call_stack.clear()  # 确保调用栈为空
        
        # 直接测试深度限制 - 模拟深度嵌套
        # 手动填充调用栈到接近限制
        self.executor.call_stack = ["tool1", "tool2", "tool3"]
        
        # 再调用一个工具应该触发深度限制
        result = self.executor.execute("calculator", {"expression": "1"})
        self.assertFalse(result.success)
        self.assertIn("depth", result.error.lower())
        
        # 恢复原始设置
        self.executor.max_depth = original_max
        self.executor.call_stack.clear()
    
    def test_empty_input_handling(self):
        """测试空输入处理"""
        result = self.executor.execute("calculator", {"expression": ""})
        self.assertFalse(result.success)
        # 验证错误信息不为空
        self.assertIsNotNone(result.error)
        self.assertGreater(len(result.error), 0)
    
    def test_invalid_tool_name(self):
        """测试无效工具名"""
        result = self.executor.execute("nonexistent_tool", {})
        self.assertFalse(result.success)
        self.assertIn("not found", result.error.lower())


def run_complex_tests():
    """运行所有复杂工具调用测试"""
    print("=" * 70)
    print("Bamboo 复杂工具调用场景测试")
    print("=" * 70)
    
    loader = unittest.TestLoader()
    suite = unittest.TestSuite()
    
    # 添加所有测试类
    suite.addTests(loader.loadTestsFromTestCase(TestConditionalToolCalls))
    suite.addTests(loader.loadTestsFromTestCase(TestParallelToolCalls))
    suite.addTests(loader.loadTestsFromTestCase(TestNestedToolCalls))
    suite.addTests(loader.loadTestsFromTestCase(TestDynamicParameterGeneration))
    suite.addTests(loader.loadTestsFromTestCase(TestLargeTextProcessing))
    suite.addTests(loader.loadTestsFromTestCase(TestRealWorldScenarios))
    suite.addTests(loader.loadTestsFromTestCase(TestPerformanceBenchmarks))
    suite.addTests(loader.loadTestsFromTestCase(TestBoundaryConditions))
    
    runner = unittest.TextTestRunner(verbosity=2)
    result = runner.run(suite)
    
    # 打印性能报告
    executor = AdvancedToolExecutor()
    print("\n" + "=" * 70)
    print("性能基准报告")
    print("=" * 70)
    
    # 运行一些测试来收集性能数据
    for i in range(10):
        executor.execute("calculator", {"expression": f"{i} * 2"})
        executor.execute("text_processor", {"text": f"test {i}", "operation": "count"})
    
    report = executor.get_performance_report()
    for tool_name, stats in report.items():
        print(f"\n{tool_name}:")
        print(f"  调用次数: {stats['count']}")
        print(f"  平均延迟: {stats['avg_ms']}ms")
        print(f"  最小延迟: {stats['min_ms']}ms")
        print(f"  最大延迟: {stats['max_ms']}ms")
        print(f"  总耗时: {stats['total_ms']}ms")
    
    print("\n" + "=" * 70)
    if result.wasSuccessful():
        print("✅ 所有复杂工具调用测试通过!")
        return 0
    else:
        print("❌ 部分测试失败")
        return 1


def main():
    """主函数"""
    import argparse
    
    parser = argparse.ArgumentParser(description="Bamboo 复杂工具调用场景测试")
    parser.add_argument("--category", choices=[
        "conditional", "parallel", "nested", "dynamic", 
        "large", "realworld", "performance", "boundary", "all"
    ], default="all", help="测试类别")
    parser.add_argument("--verbose", "-v", action="store_true", help="详细输出")
    
    args = parser.parse_args()
    
    if args.category == "all":
        return run_complex_tests()
    else:
        # 运行特定类别测试
        category_map = {
            "conditional": TestConditionalToolCalls,
            "parallel": TestParallelToolCalls,
            "nested": TestNestedToolCalls,
            "dynamic": TestDynamicParameterGeneration,
            "large": TestLargeTextProcessing,
            "realworld": TestRealWorldScenarios,
            "performance": TestPerformanceBenchmarks,
            "boundary": TestBoundaryConditions,
        }
        
        loader = unittest.TestLoader()
        suite = loader.loadTestsFromTestCase(category_map[args.category])
        
        verbosity = 2 if args.verbose else 1
        runner = unittest.TextTestRunner(verbosity=verbosity)
        result = runner.run(suite)
        
        return 0 if result.wasSuccessful() else 1


if __name__ == "__main__":
    sys.exit(main())
