#!/bin/bash
# Bamboo API 工具和技能测试脚本
# 运行方式: ./scripts/test_tools.sh [options]

set -e

# 颜色定义
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 脚本目录
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TEST_SCRIPT="$PROJECT_ROOT/tests/api/test_tools.py"

# 默认配置
RUN_MOCK=true
RUN_REAL=false
RUN_DEMO=false
API_URL="http://localhost:8080"

# 显示帮助
show_help() {
    echo "Bamboo API 工具和技能测试脚本"
    echo ""
    echo "用法: $0 [选项]"
    echo ""
    echo "选项:"
    echo "  --mock-only     只运行 Mock 测试（默认）"
    echo "  --real-only     只运行真实 API 测试"
    echo "  --all           运行所有测试（Mock + 真实 API）"
    echo "  --demo          运行演示模式"
    echo "  --url URL       指定 API 基础 URL (默认: $API_URL)"
    echo "  --help          显示此帮助信息"
    echo ""
    echo "示例:"
    echo "  $0                    # 运行 Mock 测试"
    echo "  $0 --demo             # 运行演示"
    echo "  $0 --all              # 运行所有测试"
    echo "  $0 --real-only        # 只运行真实 API 测试"
    echo "  $0 --url http://localhost:8081  # 使用自定义 API 地址"
}

# 检查依赖
check_dependencies() {
    if ! command -v python3 &> /dev/null; then
        echo -e "${RED}✗ 错误: 未找到 python3${NC}"
        exit 1
    fi
    
    if [ ! -f "$TEST_SCRIPT" ]; then
        echo -e "${RED}✗ 错误: 测试脚本不存在: $TEST_SCRIPT${NC}"
        exit 1
    fi
    
    echo -e "${GREEN}✓ 依赖检查通过${NC}"
}

# 解析参数
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            --mock-only)
                RUN_MOCK=true
                RUN_REAL=false
                shift
                ;;
            --real-only)
                RUN_MOCK=false
                RUN_REAL=true
                shift
                ;;
            --all)
                RUN_MOCK=true
                RUN_REAL=true
                shift
                ;;
            --demo)
                RUN_DEMO=true
                RUN_MOCK=false
                RUN_REAL=false
                shift
                ;;
            --url)
                API_URL="$2"
                shift 2
                ;;
            --help)
                show_help
                exit 0
                ;;
            *)
                echo -e "${RED}✗ 未知选项: $1${NC}"
                show_help
                exit 1
                ;;
        esac
    done
}

# 运行测试
run_tests() {
    echo ""
    echo "=========================================="
    echo "Bamboo API 工具和技能测试"
    echo "=========================================="
    echo ""
    
    if [ "$RUN_DEMO" = true ]; then
        echo -e "${BLUE}▶ 运行演示模式${NC}"
        python3 "$TEST_SCRIPT" --demo --url "$API_URL"
        return $?
    fi
    
    local exit_code=0
    
    if [ "$RUN_MOCK" = true ]; then
        echo -e "${BLUE}▶ 运行 Mock 测试${NC}"
        if python3 "$TEST_SCRIPT" --mock-only --url "$API_URL"; then
            echo -e "${GREEN}✓ Mock 测试通过${NC}"
        else
            echo -e "${RED}✗ Mock 测试失败${NC}"
            exit_code=1
        fi
    fi
    
    if [ "$RUN_REAL" = true ]; then
        echo ""
        echo -e "${BLUE}▶ 运行真实 API 测试${NC}"
        echo "  API URL: $API_URL"
        
        if python3 "$TEST_SCRIPT" --real-only --url "$API_URL"; then
            echo -e "${GREEN}✓ 真实 API 测试通过${NC}"
        else
            echo -e "${YELLOW}⚠ 真实 API 测试未通过（服务器可能未运行）${NC}"
            # 真实 API 测试失败不视为致命错误
        fi
    fi
    
    return $exit_code
}

# 主函数
main() {
    parse_args "$@"
    check_dependencies
    run_tests
    
    local exit_code=$?
    
    echo ""
    echo "=========================================="
    if [ $exit_code -eq 0 ]; then
        echo -e "${GREEN}✅ 测试完成!${NC}"
    else
        echo -e "${RED}❌ 测试失败${NC}"
    fi
    echo "=========================================="
    
    exit $exit_code
}

main "$@"
