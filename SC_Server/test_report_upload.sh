#!/bin/bash

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 服务器地址
SERVER_URL="http://127.0.0.1:5000"

# 测试用户凭证
USERNAME="admin"
PASSWORD="admin123"

# 调试模式（设置为true以启用详细输出）
DEBUG=true

# 测试报告数据
REPORT_DATA='{
  "title": "测试扫描报告",
  "scan_target": "/path/to/test/project",
  "summary": "这是一个用于测试的扫描报告",
  "results": {
    "scanned_files": 120,
    "scan_duration": 5.2,
    "findings": [
      {
        "file": "test.js",
        "line": 42,
        "code": "eval(userInput)",
        "rule_id": "CWE-95",
        "severity": "critical",
        "description": "发现使用 eval() 执行用户输入，可能导致代码注入"
      },
      {
        "file": "config.php",
        "line": 15,
        "code": "$password = \"hardcoded_password\";",
        "rule_id": "CWE-798",
        "severity": "high",
        "description": "发现硬编码密码"
      }
    ]
  },
  "stats": {
    "critical": 1,
    "high": 1,
    "medium": 0,
    "low": 0
  }
}'

# 函数：打印带颜色的消息
print_message() {
  local color=$1
  local message=$2
  echo -e "${color}${message}${NC}"
}

# 函数：检查命令是否存在
check_command() {
  if ! command -v $1 &> /dev/null; then
    print_message $RED "错误: 找不到命令 '$1'，请先安装它。"
    exit 1
  fi
}

# 检查必要的命令
check_command curl
check_command jq

# 函数：登录并获取 JWT 令牌
get_jwt_token() {
  #print_message $BLUE "正在登录获取 JWT 令牌..."

  local login_response=$(curl -s -X POST \
    -H "Content-Type: application/json" \
    -d "{\"username\":\"$USERNAME\",\"password\":\"$PASSWORD\"}" \
    "$SERVER_URL/api/v1/auth/login")

  # 检查登录是否成功
  if echo "$login_response" | grep -q "error"; then
    print_message $RED "登录失败: $(echo $login_response | jq -r '.error // .message')"
    exit 1
  fi

  # 提取 JWT 令牌
  local access_token=$(echo "$login_response" | jq -r '.access_token')

  if [ "$access_token" == "null" ] || [ -z "$access_token" ]; then
    print_message $RED "无法获取 JWT 令牌"
    exit 1
  fi

  #print_message $GREEN "成功获取 JWT 令牌"
  # 直接返回令牌，不打印任何其他内容
  printf "%s" "$access_token"
}

# 函数：上传报告
upload_report() {
  local token=$1
  print_message $BLUE "正在上传测试报告..."

  # 检查令牌是否为空
  if [ -z "$token" ]; then
    print_message $RED "JWT令牌为空，无法上传报告"
    return 1
  fi

  print_message $BLUE "使用令牌: ${token:0:20}..."

  # 根据调试模式决定是否使用详细输出
  if [ "$DEBUG" = true ]; then
    print_message $BLUE "发送请求到: $SERVER_URL/api/v1/reports/upload"
    local upload_response=$(curl -v -X POST \
      -H "Content-Type: application/json" \
      -H "Authorization: Bearer $token" \
      -d "$REPORT_DATA" \
      "$SERVER_URL/api/v1/reports/upload" 2>&1)
  else
    local upload_response=$(curl -s -X POST \
      -H "Content-Type: application/json" \
      -H "Authorization: Bearer $token" \
      -d "$REPORT_DATA" \
      "$SERVER_URL/api/v1/reports/upload")
  fi

  # 打印上传响应以便调试
  print_message $YELLOW "上传响应: $upload_response"

  # 检查上传是否成功
  if echo "$upload_response" | grep -q "error"; then
    print_message $RED "上传失败: $(echo $upload_response | jq -r '.error // .message')"
    return 1
  fi

  # 检查是否收到了405错误
  if echo "$upload_response" | grep -q "Method Not Allowed"; then
    print_message $RED "上传失败: 405 Method Not Allowed - 请检查API端点是否正确"
    return 1
  fi

  # 尝试提取报告ID
  # 首先检查响应是否是有效的JSON
  if ! echo "$upload_response" | jq . > /dev/null 2>&1; then
    # 如果不是有效的JSON，尝试从响应中提取report_id
    local report_id=$(echo "$upload_response" | grep -o '"report_id":"[^"]*"' | cut -d'"' -f4)
    print_message $BLUE "从非JSON响应中提取的报告ID: '$report_id'"
  else
    # 如果是有效的JSON，使用jq提取
    local report_id=$(echo "$upload_response" | jq -r '.report_id')
    print_message $BLUE "从JSON响应中提取的报告ID: '$report_id'"
  fi

  if [ "$report_id" == "null" ] || [ -z "$report_id" ]; then
    print_message $RED "上传成功但无法获取报告 ID"
    return 1
  fi

  print_message $GREEN "报告上传成功，报告 ID: $report_id"
  echo $report_id
}

# 函数：验证报告是否存在
verify_report() {
  local token=$1
  local report_id=$2
  print_message $BLUE "正在验证报告是否存在..."

  # 检查报告ID是否为空
  if [ -z "$report_id" ]; then
    print_message $RED "报告ID为空，无法验证"
    return 1
  fi

  print_message $BLUE "报告ID: '$report_id'"

  # 根据调试模式决定是否使用详细输出
  if [ "$DEBUG" = true ]; then
    print_message $BLUE "发送请求到: $SERVER_URL/api/v1/reports/$report_id"
    local report_response=$(curl -v -X GET \
      -H "Content-Type: application/json" \
      -H "Authorization: Bearer $token" \
      "$SERVER_URL/api/v1/reports/$report_id" 2>&1)
  else
    local report_response=$(curl -s -X GET \
      -H "Content-Type: application/json" \
      -H "Authorization: Bearer $token" \
      "$SERVER_URL/api/v1/reports/$report_id")
  fi

  # 打印完整响应以便调试
  print_message $YELLOW "API 响应: $report_response"

  # 检查获取报告是否成功
  if echo "$report_response" | grep -q "error"; then
    print_message $RED "验证失败: $(echo $report_response | jq -r '.error // .message')"
    return 1
  fi

  local title=$(echo $report_response | jq -r '.title')

  if [ "$title" == "null" ] || [ -z "$title" ]; then
    print_message $RED "验证失败: 无法获取报告标题"
    return 1
  fi

  print_message $GREEN "验证成功: 找到报告 '$title'"
  return 0
}

# 函数：清理测试报告
cleanup_report() {
  local token=$1
  local report_id=$2
  print_message $BLUE "正在清理测试报告..."

  # 检查参数是否为空
  if [ -z "$token" ]; then
    print_message $YELLOW "警告: JWT令牌为空，无法删除报告"
    return 1
  fi

  if [ -z "$report_id" ]; then
    print_message $YELLOW "警告: 报告ID为空，无法删除报告"
    return 1
  fi

  # 根据调试模式决定是否使用详细输出
  if [ "$DEBUG" = true ]; then
    print_message $BLUE "发送请求到: $SERVER_URL/api/v1/reports/$report_id"
    local delete_response=$(curl -v -X DELETE \
      -H "Content-Type: application/json" \
      -H "Authorization: Bearer $token" \
      "$SERVER_URL/api/v1/reports/$report_id" 2>&1)
  else
    local delete_response=$(curl -s -X DELETE \
      -H "Content-Type: application/json" \
      -H "Authorization: Bearer $token" \
      "$SERVER_URL/api/v1/reports/$report_id")
  fi

  # 打印删除响应以便调试
  print_message $YELLOW "删除响应: $delete_response"

  # 检查删除是否成功
  if echo "$delete_response" | grep -q "error"; then
    print_message $YELLOW "警告: 无法删除测试报告: $(echo $delete_response | jq -r '.error // .message')"
    return 1
  fi

  print_message $GREEN "测试报告已成功删除"
  return 0
}

# 主函数
main() {
  print_message $BLUE "开始测试报表上传功能..."

  # 检查服务器是否在运行
  if ! curl -s "$SERVER_URL" > /dev/null; then
    print_message $RED "错误: 无法连接到服务器 $SERVER_URL"
    print_message $YELLOW "请确保服务器正在运行，然后再次尝试"
    exit 1
  fi

  # 获取 JWT 令牌
  local token=$(get_jwt_token)

  if [ -z "$token" ]; then
    print_message $RED "无法获取有效的JWT令牌"
    exit 1
  fi

  # 上传报告
  local report_id=$(upload_report "$token")
  local upload_status=$?

  if [ $upload_status -ne 0 ]; then
    print_message $RED "报表上传测试失败"
    exit 1
  fi

  # 验证报告
  verify_report "$token" "$report_id"
  local verify_status=$?

  if [ $verify_status -ne 0 ]; then
    print_message $RED "报表验证测试失败"
    exit 1
  fi

  # 清理测试报告
  cleanup_report "$token" "$report_id"

  print_message $GREEN "报表上传功能测试成功完成！"
}

# 执行主函数
main
