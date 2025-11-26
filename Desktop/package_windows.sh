#!/bin/bash
# Windows 打包脚本 (Bash 版本)
# 用于在 macOS/Linux 上交叉编译 Windows 版本的 SDChat Scanner

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
FULL_APP_NAME="SDChat Scanner"
BUILD_DATE=$(date +"%Y-%m-%d")

# 输出目录
OUTPUT_DIR="./dist"
mkdir -p "$OUTPUT_DIR"

# 打印标题
echo -e "${BLUE}=========================================${NC}"
echo -e "${BLUE}  SDChat Scanner Windows 打包脚本 v${VERSION}  ${NC}"
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

# 安装 MinGW 工具链
if [[ "$OSTYPE" == "darwin"* ]]; then
    echo -e "${YELLOW}检测到 macOS 环境，配置 MinGW 交叉编译环境...${NC}"
    
    # 检查是否已安装 mingw-w64
    if ! brew list mingw-w64 &>/dev/null; then
        echo -e "${YELLOW}安装 mingw-w64 (Windows 交叉编译工具链)...${NC}"
        brew install mingw-w64
    else
        echo -e "${GREEN}mingw-w64 已安装${NC}"
    fi
fi

# 构建 Windows 版本
echo -e "\n${BLUE}正在构建 Windows 版本...${NC}"

# 检查目标平台
if ! rustup target list --installed | grep -q "x86_64-pc-windows-gnu"; then
    echo -e "${YELLOW}目标平台 x86_64-pc-windows-gnu 未安装，正在安装...${NC}"
    rustup target add x86_64-pc-windows-gnu
else
    echo -e "${GREEN}目标平台 x86_64-pc-windows-gnu 已安装${NC}"
fi

# 构建
echo -e "${YELLOW}执行 cargo build --release --target x86_64-pc-windows-gnu --features file_dialog,static_link${NC}"
RUSTFLAGS="-C target-feature=+crt-static" cargo build --release --target x86_64-pc-windows-gnu --features file_dialog,static_link

# 创建输出目录
PLATFORM_DIR="${OUTPUT_DIR}/${APP_NAME}-${VERSION}-windows"
rm -rf "${PLATFORM_DIR}"
mkdir -p "${PLATFORM_DIR}"

# 复制二进制文件
cp "target/x86_64-pc-windows-gnu/release/${APP_NAME}.exe" "${PLATFORM_DIR}/"

# 复制必要的资源文件
if [ -d "resources" ]; then
    cp -r "resources" "${PLATFORM_DIR}/"
fi

# 创建README文件
cat > "${PLATFORM_DIR}/README.txt" << EOF
SDChat Scanner ${VERSION}
构建日期: ${BUILD_DATE}
平台: Windows

使用方法:
1. 运行 ${APP_NAME}.exe 启动应用程序
2. 选择要扫描的目录
3. 点击"开始扫描"按钮

注意事项:
- 首次运行时，应用程序会自动下载最新的扫描规则
- 扫描报告将保存在用户文档目录下的 SDChat-Scanner/reports 文件夹中
EOF

# 创建压缩包
echo -e "${GREEN}正在创建压缩包...${NC}"
if command -v zip &> /dev/null; then
    (cd "${OUTPUT_DIR}" && zip -r "${APP_NAME}-${VERSION}-windows.zip" "$(basename "${PLATFORM_DIR}")")
else
    echo -e "${YELLOW}警告: 未找到 zip 命令，跳过创建 zip 文件${NC}"
    echo -e "${YELLOW}请手动压缩 ${PLATFORM_DIR} 目录${NC}"
fi

echo -e "${GREEN}Windows 版本构建完成: ${OUTPUT_DIR}/${APP_NAME}-${VERSION}-windows.zip${NC}"
