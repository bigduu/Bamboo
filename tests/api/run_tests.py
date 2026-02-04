#!/usr/bin/env python3
"""
Bamboo API Test Runner
======================
主测试运行脚本，支持：
- 运行特定测试套件
- 生成 HTML 测试报告
- 生成 JUnit XML 报告（CI/CD 集成）
- 测试覆盖率报告
"""

import argparse
import os
import subprocess
import sys
from datetime import datetime
from pathlib import Path
from typing import List, Optional

# 项目根目录
PROJECT_ROOT = Path(__file__).parent.parent.parent
TESTS_DIR = Path(__file__).parent
REPORTS_DIR = TESTS_DIR / "reports"


def ensure_reports_dir() -> Path:
    """确保报告目录存在"""
    REPORTS_DIR.mkdir(parents=True, exist_ok=True)
    return REPORTS_DIR


def get_timestamp() -> str:
    """获取时间戳字符串"""
    return datetime.now().strftime("%Y%m%d_%H%M%S")


def run_command(cmd: List[str], description: str) -> int:
    """运行命令并返回退出码"""
    print(f"\n{'='*60}")
    print(f"📋 {description}")
    print(f"{'='*60}")
    print(f"命令: {' '.join(cmd)}\n")
    
    result = subprocess.run(cmd, cwd=PROJECT_ROOT)
    return result.returncode


def run_tests(
    test_suite: Optional[str] = None,
    html_report: bool = True,
    junit_xml: bool = False,
    coverage: bool = True,
    verbose: bool = False,
    markers: Optional[str] = None,
    keyword: Optional[str] = None,
    failfast: bool = False,
    parallel: bool = False,
    workers: int = 4
) -> int:
    """
    运行测试
    
    Args:
        test_suite: 特定测试套件路径（如 test_agents.py）
        html_report: 是否生成 HTML 报告
        junit_xml: 是否生成 JUnit XML 报告（CI/CD）
        coverage: 是否生成覆盖率报告
        verbose: 详细输出
        markers: 按标记过滤测试（如 'integration'）
        keyword: 按关键字过滤测试
        failfast: 遇到第一个失败时停止
        parallel: 是否并行运行
        workers: 并行工作进程数
    """
    ensure_reports_dir()
    timestamp = get_timestamp()
    
    # 构建 pytest 命令
    cmd = ["python", "-m", "pytest"]
    
    # 测试目标
    if test_suite:
        test_path = TESTS_DIR / test_suite
        if not test_path.exists():
            print(f"❌ 错误: 测试套件不存在: {test_path}")
            return 1
        cmd.append(str(test_path))
    else:
        cmd.append(str(TESTS_DIR))
    
    # 详细输出
    if verbose:
        cmd.append("-v")
    else:
        cmd.append("-v" if not parallel else "-q")
    
    # 失败即停止
    if failfast:
        cmd.append("-x")
    
    # 标记过滤
    if markers:
        cmd.extend(["-m", markers])
    
    # 关键字过滤
    if keyword:
        cmd.extend(["-k", keyword])
    
    # 覆盖率
    if coverage:
        cmd.extend([
            "--cov=crates",
            "--cov-report=term-missing",
            f"--cov-report=html:{REPORTS_DIR / f'coverage_html_{timestamp}'}",
            f"--cov-report=xml:{REPORTS_DIR / f'coverage_{timestamp}.xml'}"
        ])
    
    # HTML 报告
    if html_report:
        html_path = REPORTS_DIR / f"report_{timestamp}.html"
        cmd.extend([f"--html={html_path}", "--self-contained-html"])
        print(f"📊 HTML 报告将保存至: {html_path}")
    
    # JUnit XML 报告（CI/CD）
    if junit_xml:
        junit_path = REPORTS_DIR / f"junit_{timestamp}.xml"
        cmd.extend([f"--junitxml={junit_path}"])
        print(f"📄 JUnit XML 报告将保存至: {junit_path}")
    
    # 并行运行
    if parallel:
        cmd.extend(["-n", str(workers), "--dist=loadfile"])
        print(f"🚀 并行模式: {workers} 个工作者")
    
    # 执行测试
    exit_code = run_command(cmd, "运行 API 测试")
    
    # 打印报告位置
    if exit_code == 0:
        print(f"\n✅ 所有测试通过！")
    else:
        print(f"\n❌ 测试失败（退出码: {exit_code}）")
    
    print(f"\n📁 报告目录: {REPORTS_DIR}")
    
    return exit_code


def run_all_tests(args) -> int:
    """运行所有测试"""
    return run_tests(
        test_suite=None,
        html_report=args.html,
        junit_xml=args.junit,
        coverage=args.coverage,
        verbose=args.verbose,
        markers=args.markers,
        keyword=args.keyword,
        failfast=args.failfast,
        parallel=args.parallel,
        workers=args.workers
    )


def run_unit_tests(args) -> int:
    """运行单元测试（排除集成测试）"""
    print("🧪 运行单元测试（排除集成测试）...")
    return run_tests(
        test_suite=None,
        html_report=args.html,
        junit_xml=args.junit,
        coverage=args.coverage,
        verbose=args.verbose,
        markers="not integration",
        keyword=args.keyword,
        failfast=args.failfast,
        parallel=args.parallel,
        workers=args.workers
    )


def run_integration_tests(args) -> int:
    """运行集成测试"""
    print("🔗 运行集成测试...")
    return run_tests(
        test_suite=None,
        html_report=args.html,
        junit_xml=args.junit,
        coverage=args.coverage,
        verbose=args.verbose,
        markers="integration",
        keyword=args.keyword,
        failfast=args.failfast,
        parallel=args.parallel,
        workers=args.workers
    )


def run_specific_suite(suite_name: str, args) -> int:
    """运行特定测试套件"""
    print(f"🎯 运行测试套件: {suite_name}")
    return run_tests(
        test_suite=suite_name,
        html_report=args.html,
        junit_xml=args.junit,
        coverage=args.coverage,
        verbose=args.verbose,
        markers=args.markers,
        keyword=args.keyword,
        failfast=args.failfast,
        parallel=args.parallel,
        workers=args.workers
    )


def list_test_suites():
    """列出可用的测试套件"""
    print("📚 可用测试套件:\n")
    
    test_files = sorted(TESTS_DIR.glob("test_*.py"))
    
    if not test_files:
        print("  未找到测试文件（test_*.py）")
        return
    
    for test_file in test_files:
        print(f"  • {test_file.name}")


def clean_reports():
    """清理报告目录"""
    if REPORTS_DIR.exists():
        import shutil
        shutil.rmtree(REPORTS_DIR)
        print(f"🧹 已清理报告目录: {REPORTS_DIR}")
    else:
        print("📂 报告目录不存在，无需清理")


def main():
    parser = argparse.ArgumentParser(
        description="Bamboo API 测试运行器",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
示例:
  # 运行所有测试
  python run_tests.py

  # 运行特定测试套件
  python run_tests.py -s test_agents.py

  # 运行单元测试（排除集成测试）
  python run_tests.py unit

  # 运行集成测试
  python run_tests.py integration

  # 生成 CI/CD 报告（JUnit XML）
  python run_tests.py --junit --coverage

  # 并行运行测试
  python run_tests.py --parallel -j 8

  # 按标记过滤
  python run_tests.py -m "not slow"

  # 列出所有测试套件
  python run_tests.py --list
        """
    )
    
    # 子命令
    subparsers = parser.add_subparsers(dest="command", help="可用命令")
    
    # 通用参数
    def add_common_args(p):
        p.add_argument("--html", action="store_true", default=True, help="生成 HTML 报告（默认启用）")
        p.add_argument("--no-html", action="store_false", dest="html", help="禁用 HTML 报告")
        p.add_argument("--junit", action="store_true", help="生成 JUnit XML 报告（CI/CD）")
        p.add_argument("--coverage", "-c", action="store_true", default=True, help="生成覆盖率报告（默认启用）")
        p.add_argument("--no-coverage", action="store_false", dest="coverage", help="禁用覆盖率报告")
        p.add_argument("--verbose", "-v", action="store_true", help="详细输出")
        p.add_argument("--markers", "-m", help="按标记过滤测试（如 'integration' 或 'not slow'）")
        p.add_argument("--keyword", "-k", help="按关键字过滤测试")
        p.add_argument("--failfast", "-x", action="store_true", help="遇到第一个失败时停止")
        p.add_argument("--parallel", "-p", action="store_true", help="并行运行测试")
        p.add_argument("--workers", "-j", type=int, default=4, help="并行工作进程数（默认: 4）")
    
    # all 命令（默认）
    all_parser = subparsers.add_parser("all", help="运行所有测试（默认）")
    add_common_args(all_parser)
    
    # unit 命令
    unit_parser = subparsers.add_parser("unit", help="运行单元测试")
    add_common_args(unit_parser)
    
    # integration 命令
    integration_parser = subparsers.add_parser("integration", help="运行集成测试")
    add_common_args(integration_parser)
    
    # suite 命令
    suite_parser = subparsers.add_parser("suite", help="运行特定测试套件")
    suite_parser.add_argument("name", help="测试套件名称（如 test_agents.py）")
    add_common_args(suite_parser)
    
    # 全局选项
    parser.add_argument("--list", "-l", action="store_true", help="列出可用测试套件")
    parser.add_argument("--clean", action="store_true", help="清理报告目录")
    parser.add_argument("--suite", "-s", help="运行特定测试套件（快捷方式）")
    add_common_args(parser)
    
    args = parser.parse_args()
    
    # 处理特殊选项
    if args.list:
        list_test_suites()
        return 0
    
    if args.clean:
        clean_reports()
        return 0
    
    # 快捷方式：-s 选项
    if args.suite:
        return run_specific_suite(args.suite, args)
    
    # 执行子命令
    if args.command == "all" or args.command is None:
        return run_all_tests(args)
    elif args.command == "unit":
        return run_unit_tests(args)
    elif args.command == "integration":
        return run_integration_tests(args)
    elif args.command == "suite":
        return run_specific_suite(args.name, args)
    else:
        parser.print_help()
        return 0


if __name__ == "__main__":
    sys.exit(main())
