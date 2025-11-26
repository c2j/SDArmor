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
  print_message $BLUE "正在登录获取 JWT 令牌..."
  
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
  local access_token=$(echo $login_response | jq -r '.access_token')
  
  if [ "$access_token" == "null" ] || [ -z "$access_token" ]; then
    print_message $RED "无法获取 JWT 令牌"
    exit 1
  fi
  
  print_message $GREEN "成功获取 JWT 令牌"
  echo $access_token
}

# 函数：上传报告
upload_report() {
  local token=$1
  print_message $BLUE "正在上传测试报告..."
  
  # 注意：这里修改了上传端点，从 /upload 改为空，因为实际的端点是 /api/v1/reports
  local upload_response=$(curl -s -X POST \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $token" \
    -d "$REPORT_DATA" \
    "$SERVER_URL/api/v1/reports")
  
  # 检查上传是否成功
  if echo "$upload_response" | grep -q "error"; then
    print_message $RED "上传失败: $(echo $upload_response | jq -r '.error // .message')"
    return 1
  fi
  
  local report_id=$(echo $upload_response | jq -r '.report_id')
  
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
  
  local report_response=$(curl -s -X GET \
    -H "Authorization: Bearer $token" \
    "$SERVER_URL/api/v1/reports/$report_id")
  
  # 检查获取报告是否成功
  if echo "$report_response" | grep -q "error"; then
    print_message $RED "验证失败: $(echo $report_response | jq -r '.error // .message')"
    return 1
  fi
  
  # 打印完整响应以便调试
  print_message $YELLOW "API 响应: $report_response"
  
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
  
  local delete_response=$(curl -s -X DELETE \
    -H "Authorization: Bearer $token" \
    "$SERVER_URL/api/v1/reports/$report_id")
  
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
  
  # 上传报告
  local report_id=$(upload_report "$token")
  if [ $? -ne 0 ]; then
    print_message $RED "报表上传测试失败"
    exit 1
  fi
  
  # 验证报告
  verify_report "$token" "$report_id"
  if [ $? -ne 0 ]; then
    print_message $RED "报表验证测试失败"
    exit 1
  fi
  
  # 清理测试报告
  cleanup_report "$token" "$report_id"
  
  print_message $GREEN "报表上传功能测试成功完成！"
}

# 执行主函数
main
