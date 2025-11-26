#!/bin/bash
# Linux 打包脚本
# 用于创建 Linux 版本的 SDChat Scanner 安装包

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
echo -e "${BLUE}  SDChat Scanner Linux 打包脚本 v${VERSION}  ${NC}"
echo -e "${BLUE}=========================================${NC}"
echo -e "构建日期: ${BUILD_DATE}\n"

# 检查必要的工具
check_tool() {
    if ! command -v $1 &> /dev/null; then
        echo -e "${RED}错误: 找不到命令 '$1'${NC}"
        echo -e "${YELLOW}请先安装 $1${NC}"
        return 1
    fi
    return 0
}

# 检查当前系统或直接构建选项
if [[ "$OSTYPE" != "linux-gnu"* && "$1" != "--direct-build" ]]; then
    echo -e "${YELLOW}检测到非 Linux 系统，将使用 Docker 进行构建...${NC}"

    # 检查是否安装了 Docker
    if ! check_tool docker; then
        echo -e "${RED}错误: 未安装 Docker${NC}"
        echo -e "${YELLOW}请先安装 Docker: https://docs.docker.com/get-docker/${NC}"
        exit 1
    fi

    # 获取当前目录的绝对路径
    CURRENT_DIR=$(pwd)

    # 创建临时 Dockerfile
    DOCKER_FILE="Dockerfile.build"
    cat > ${DOCKER_FILE} << EOF
FROM rust:latest

# 安装构建依赖
RUN apt-get update && apt-get install -y \\
    libgtk-3-dev \\
    libssl-dev \\
    pkg-config \\
    libx11-dev \\
    libxcb1-dev \\
    libxcb-render0-dev \\
    libxcb-shape0-dev \\
    libxcb-xfixes0-dev \\
    libxkbcommon-dev \\
    libwayland-dev \\
    && rm -rf /var/lib/apt/lists/*

# 添加目标平台
RUN rustup target add x86_64-unknown-linux-gnu

WORKDIR /app
CMD ["bash", "-c", "RUSTFLAGS='-C target-feature=-crt-static -C link-arg=-Wl,--no-as-needed -C link-arg=-ldl' cargo build --release --target x86_64-unknown-linux-gnu --features file_dialog,vendored-openssl && chown -R \$(stat -c '%u:%g' .) /app"]
EOF

    echo -e "${BLUE}构建 Docker 镜像...${NC}"
    docker build -t sdchat-scanner-builder -f ${DOCKER_FILE} .

    echo -e "${BLUE}在 Docker 中构建应用程序...${NC}"
    docker run --rm -v "${CURRENT_DIR}:/app" sdchat-scanner-builder

    # 清理临时文件
    rm ${DOCKER_FILE}

    echo -e "${GREEN}Docker 构建完成${NC}"
else
    # 在 Linux 上直接构建或使用直接构建选项
    if [[ "$1" == "--direct-build" ]]; then
        echo -e "${GREEN}使用直接构建选项，跳过Docker...${NC}"
    else
        echo -e "${GREEN}在 Linux 系统上直接构建${NC}"
    fi

    # 检查 Rust 工具链
    check_tool rustup
    check_tool cargo

    # 检查目标平台
    if ! rustup target list --installed | grep -q "x86_64-unknown-linux-gnu"; then
        echo -e "${YELLOW}目标平台 x86_64-unknown-linux-gnu 未安装，正在安装...${NC}"
        rustup target add x86_64-unknown-linux-gnu
    else
        echo -e "${GREEN}目标平台 x86_64-unknown-linux-gnu 已安装${NC}"
    fi

    # 安装构建依赖
    echo -e "${BLUE}检查构建依赖...${NC}"
    if command -v apt-get &> /dev/null; then
        echo -e "${BLUE}使用 apt-get 安装依赖...${NC}"
        sudo apt-get update
        sudo apt-get install -y libgtk-3-dev libssl-dev pkg-config
    elif command -v dnf &> /dev/null; then
        echo -e "${BLUE}使用 dnf 安装依赖...${NC}"
        sudo dnf install -y gtk3-devel openssl-devel pkg-config
    elif command -v pacman &> /dev/null; then
        echo -e "${BLUE}使用 pacman 安装依赖...${NC}"
        sudo pacman -Syu --noconfirm gtk3 openssl pkg-config
    else
        echo -e "${YELLOW}无法自动安装依赖，请手动安装 GTK3 开发库和 OpenSSL 开发库${NC}"
    fi

    # 在 Linux 上直接构建，使用更保守的构建选项以提高兼容性
    # 构建
    if [[ "$1" == "--direct-build" ]]; then
        echo -e "${YELLOW}执行 cargo build --release --features file_dialog,vendored-openssl${NC}"
        # 根据操作系统类型使用不同的链接选项
        if [[ "$OSTYPE" == "darwin"* ]]; then
            # macOS不支持--no-as-needed和-ldl选项
            RUSTFLAGS="-C target-feature=-crt-static" cargo build --release --features file_dialog,vendored-openssl
        else
            # Linux系统使用完整的链接选项
            RUSTFLAGS="-C target-feature=-crt-static -C link-arg=-Wl,--no-as-needed -C link-arg=-ldl" cargo build --release --features file_dialog,vendored-openssl
        fi
        # 创建符号链接以便后续步骤能找到二进制文件
        mkdir -p "target/x86_64-unknown-linux-gnu/release/"
        ln -sf "../../target/release/${APP_NAME}" "target/x86_64-unknown-linux-gnu/release/${APP_NAME}"
    else
        echo -e "${YELLOW}执行 cargo build --release --target x86_64-unknown-linux-gnu --features file_dialog,vendored-openssl${NC}"
        # 使用较旧的glibc目标以提高兼容性
        RUSTFLAGS="-C target-feature=-crt-static -C link-arg=-Wl,--no-as-needed -C link-arg=-ldl" cargo build --release --target x86_64-unknown-linux-gnu --features file_dialog,vendored-openssl
    fi
fi

# 创建输出目录
PLATFORM_DIR="${OUTPUT_DIR}/${APP_NAME}-${VERSION}-linux"
rm -rf "${PLATFORM_DIR}"
mkdir -p "${PLATFORM_DIR}"

# 复制二进制文件
if [[ "$1" == "--direct-build" && "$OSTYPE" == "darwin"* ]]; then
    # 在macOS上直接构建时，使用本地构建的二进制文件
    cp "target/release/${APP_NAME}" "${PLATFORM_DIR}/"
else
    # 在Linux上或使用Docker构建时，使用交叉编译的二进制文件
    cp "target/x86_64-unknown-linux-gnu/release/${APP_NAME}" "${PLATFORM_DIR}/"
fi

# 复制必要的资源文件
if [ -d "resources" ]; then
    cp -r "resources" "${PLATFORM_DIR}/"
fi

# 创建启动脚本
cat > "${PLATFORM_DIR}/run.sh" << EOF
#!/bin/bash
# 启动 SDChat Scanner

# 获取脚本所在目录
SCRIPT_DIR="\$(cd "\$(dirname "\${BASH_SOURCE[0]}")" && pwd)"

# 运行应用程序
"\${SCRIPT_DIR}/${APP_NAME}" "\$@"
EOF

# 设置可执行权限
chmod +x "${PLATFORM_DIR}/run.sh"
chmod +x "${PLATFORM_DIR}/${APP_NAME}"

# 创建README文件
cat > "${PLATFORM_DIR}/README.txt" << EOF
SDChat Scanner ${VERSION}
构建日期: ${BUILD_DATE}
平台: Linux

使用方法:
1. 打开终端并进入此目录
2. 运行 ./run.sh 启动应用程序
   或者直接运行 ./${APP_NAME}
3. 选择要扫描的目录
4. 点击"开始扫描"按钮

注意事项:
- 首次运行时，应用程序会自动下载最新的扫描规则
- 扫描报告将保存在用户文档目录下的 SDChat-Scanner/reports 文件夹中
- 如果遇到权限问题，请确保二进制文件具有可执行权限：chmod +x ./${APP_NAME}
- 如果遇到缺少库的问题，请安装以下依赖：
  Ubuntu/Debian: sudo apt-get install libgtk-3-0 libc6 libm6
  Fedora/RHEL: sudo dnf install gtk3 glibc
  Arch Linux: sudo pacman -S gtk3 glibc

  注意：
  1. 本应用已静态链接OpenSSL库，不需要额外安装OpenSSL
  2. 如果遇到libm.so.6或libc.so.6缺失问题，请确保系统glibc版本不低于2.31
  3. 对于较旧的Linux发行版，可能需要升级glibc或使用兼容层
EOF

# 创建 .desktop 文件
mkdir -p "${PLATFORM_DIR}/desktop-integration"
cat > "${PLATFORM_DIR}/desktop-integration/sdchat-scanner.desktop" << EOF
[Desktop Entry]
Name=${FULL_APP_NAME}
Comment=Security vulnerability scanner client
Exec=${APP_NAME}
Icon=sdchat-scanner
Terminal=false
Type=Application
Categories=Development;Security;
EOF

# 创建安装脚本
cat > "${PLATFORM_DIR}/install.sh" << EOF
#!/bin/bash
# 安装 SDChat Scanner 到系统

set -e

# 获取脚本所在目录
SCRIPT_DIR="\$(cd "\$(dirname "\${BASH_SOURCE[0]}")" && pwd)"

# 安装目录
INSTALL_DIR="/opt/${APP_NAME}"
BIN_DIR="/usr/local/bin"
DESKTOP_DIR="/usr/share/applications"
ICON_DIR="/usr/share/icons/hicolor"

# 需要 root 权限
if [ "\$(id -u)" -ne 0 ]; then
    echo "错误: 需要 root 权限来安装应用程序"
    echo "请使用 sudo 运行此脚本"
    exit 1
fi

echo "正在安装 ${FULL_APP_NAME} v${VERSION}..."

# 检查依赖
echo "检查系统依赖..."
if command -v apt-get &> /dev/null; then
    apt-get update
    apt-get install -y libgtk-3-0 libc6
elif command -v dnf &> /dev/null; then
    dnf install -y gtk3 glibc
elif command -v pacman &> /dev/null; then
    pacman -Syu --noconfirm gtk3 glibc
else
    echo "警告: 无法自动安装依赖，请确保系统已安装 GTK3 库和基本的C库(glibc)"
fi

echo "注意: 本应用已静态链接OpenSSL库，不需要额外安装OpenSSL"
echo "如果遇到libm.so.6或libc.so.6缺失问题，请确保系统glibc版本不低于2.31"

# 创建安装目录
mkdir -p "\${INSTALL_DIR}"

# 复制文件
cp -r "\${SCRIPT_DIR}"/* "\${INSTALL_DIR}/"

# 创建符号链接
ln -sf "\${INSTALL_DIR}/${APP_NAME}" "\${BIN_DIR}/${APP_NAME}"

# 安装桌面文件
cp "\${SCRIPT_DIR}/desktop-integration/sdchat-scanner.desktop" "\${DESKTOP_DIR}/"

# 安装图标
if [ -d "\${SCRIPT_DIR}/resources/icons" ]; then
    for icon in "\${SCRIPT_DIR}"/resources/icons/*.png; do
        if [ -f "\${icon}" ]; then
            size=\$(identify -format "%wx%h" "\${icon}" | cut -d 'x' -f 1)
            mkdir -p "\${ICON_DIR}/\${size}x\${size}/apps"
            cp "\${icon}" "\${ICON_DIR}/\${size}x\${size}/apps/sdchat-scanner.png"
        fi
    done
fi

echo "${FULL_APP_NAME} 已成功安装!"
echo "您可以从应用程序菜单启动它，或者在终端中运行 '${APP_NAME}'"
EOF

# 设置可执行权限
chmod +x "${PLATFORM_DIR}/install.sh"

# 创建压缩包
echo -e "${GREEN}正在创建压缩包...${NC}"
(cd "${OUTPUT_DIR}" && tar -czf "${APP_NAME}-${VERSION}-linux.tar.gz" "$(basename "${PLATFORM_DIR}")")

echo -e "${GREEN}Linux 版本构建完成: ${OUTPUT_DIR}/${APP_NAME}-${VERSION}-linux.tar.gz${NC}"

# 如果是非 Linux 系统，添加提示
if [[ "$OSTYPE" != "linux-gnu"* ]]; then
    echo -e "${YELLOW}注意: 此二进制文件是在 Docker 中构建的，应该可以在大多数 Linux 发行版上运行。${NC}"
    echo -e "${YELLOW}如果遇到问题，请确保目标系统安装了 GTK3 库。${NC}"
fi
