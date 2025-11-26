#!/bin/bash
# 跨平台构建脚本 - 用于构建 Windows, Linux 和 macOS 版本的 SDChat Scanner

set -e  # 遇到错误立即退出

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 版本信息
VERSION=$(grep '^version' Cargo.toml | head -n 1 | cut -d '"' -f 2)
APP_NAME="sdchat-scanner"
BUILD_DATE=$(date +"%Y-%m-%d")

# 输出目录
OUTPUT_DIR="./dist"
mkdir -p "$OUTPUT_DIR"

# 打印标题
echo -e "${BLUE}=========================================${NC}"
echo -e "${BLUE}  SDChat Scanner 跨平台构建脚本 v${VERSION}  ${NC}"
echo -e "${BLUE}=========================================${NC}"
echo -e "构建日期: ${BUILD_DATE}\n"

# 检查必要的工具
check_tool() {
    if ! command -v $1 &> /dev/null; then
        echo -e "${RED}错误: 找不到命令 '$1'${NC}"
        echo -e "${YELLOW}请先安装 $1${NC}"
        exit 1
    fi
}

check_tool rustup
check_tool cargo

# 检查目标平台是否已安装
check_target() {
    if ! rustup target list --installed | grep -q "$1"; then
        echo -e "${YELLOW}目标平台 $1 未安装，正在安装...${NC}"
        rustup target add $1
    else
        echo -e "${GREEN}目标平台 $1 已安装${NC}"
    fi
}

# 构建特定平台的版本
build_platform() {
    local target=$1
    local platform_name=$2
    local binary_ext=$3
    local feature_flags=$4

    echo -e "\n${BLUE}正在构建 ${platform_name} 版本...${NC}"

    # 检查目标平台
    check_target $target

    # 构建
    echo -e "${YELLOW}执行 cargo build --release --target ${target} ${feature_flags}${NC}"
    RUSTFLAGS="-C target-feature=+crt-static" cargo build --release --target ${target} ${feature_flags}

    # 创建输出目录
    local platform_dir="${OUTPUT_DIR}/${APP_NAME}-${VERSION}-${platform_name}"
    mkdir -p "${platform_dir}"

    # 复制二进制文件
    local binary_name="${APP_NAME}${binary_ext}"
    cp "target/${target}/release/${binary_name}" "${platform_dir}/"

    # 复制必要的资源文件
    cp -r "resources" "${platform_dir}/" 2>/dev/null || true

    # 创建README文件
    cat > "${platform_dir}/README.txt" << EOF
SDChat Scanner ${VERSION}
构建日期: ${BUILD_DATE}
平台: ${platform_name}

使用方法:
1. 运行 ${binary_name} 启动应用程序
2. 选择要扫描的目录
3. 点击"开始扫描"按钮

注意事项:
- 首次运行时，应用程序会自动下载最新的扫描规则
- 扫描报告将保存在用户文档目录下的 SDChat-Scanner/reports 文件夹中
EOF

    # 创建压缩包
    echo -e "${GREEN}正在创建压缩包...${NC}"
    (cd "${OUTPUT_DIR}" && zip -r "${APP_NAME}-${VERSION}-${platform_name}.zip" "$(basename "${platform_dir}")")

    echo -e "${GREEN}${platform_name} 版本构建完成: ${OUTPUT_DIR}/${APP_NAME}-${VERSION}-${platform_name}.zip${NC}"
}

# 清理旧的构建
echo -e "${YELLOW}清理旧的构建...${NC}"
cargo clean

# 构建 Windows 版本
echo -e "${BLUE}正在构建 Windows 版本...${NC}"
./package_windows.sh

# 构建 Linux 版本
build_platform "x86_64-unknown-linux-gnu" "linux" "" "--features file_dialog"

# 构建 macOS 版本
build_platform "x86_64-apple-darwin" "macos" "" "--features file_dialog"

echo -e "\n${GREEN}所有平台构建完成!${NC}"
echo -e "${BLUE}输出目录: ${OUTPUT_DIR}${NC}"
