#!/bin/bash
# macOS 打包脚本
# 用于创建 macOS 版本的 SDChat Scanner 安装包

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
echo -e "${BLUE}  SDChat Scanner macOS 打包脚本 v${VERSION}  ${NC}"
echo -e "${BLUE}=========================================${NC}"
echo -e "构建日期: ${BUILD_DATE}\n"

# 检查必要的工具
check_tool() {
    if ! command -v $1 &> /dev/null; then
        if [ "$2" = "optional" ]; then
            echo -e "${YELLOW}警告: 找不到命令 '$1'${NC}"
            echo -e "${YELLOW}$3${NC}"
            return 1
        else
            echo -e "${RED}错误: 找不到命令 '$1'${NC}"
            echo -e "${YELLOW}请先安装 $1${NC}"
            exit 1
        fi
    fi
    return 0
}

check_tool rustup
check_tool cargo
check_tool create-dmg "optional" "将不会创建 DMG 文件，只生成 ZIP 包"

# 构建 macOS 版本
echo -e "\n${BLUE}正在构建 macOS 版本...${NC}"

# 检查目标平台
if ! rustup target list --installed | grep -q "x86_64-apple-darwin"; then
    echo -e "${YELLOW}目标平台 x86_64-apple-darwin 未安装，正在安装...${NC}"
    rustup target add x86_64-apple-darwin
else
    echo -e "${GREEN}目标平台 x86_64-apple-darwin 已安装${NC}"
fi

# 构建
echo -e "${YELLOW}执行 cargo build --release --target x86_64-apple-darwin --features file_dialog${NC}"
cargo build --release --target x86_64-apple-darwin --features file_dialog

# 创建输出目录
PLATFORM_DIR="${OUTPUT_DIR}/${APP_NAME}-${VERSION}-macos"
rm -rf "${PLATFORM_DIR}"
mkdir -p "${PLATFORM_DIR}"

# 创建 .app 包结构
APP_BUNDLE="${PLATFORM_DIR}/${FULL_APP_NAME}.app"
mkdir -p "${APP_BUNDLE}/Contents/MacOS"
mkdir -p "${APP_BUNDLE}/Contents/Resources"

# 复制二进制文件
cp "target/x86_64-apple-darwin/release/${APP_NAME}" "${APP_BUNDLE}/Contents/MacOS/"

# 复制资源文件
if [ -d "resources" ]; then
    cp -r "resources" "${APP_BUNDLE}/Contents/Resources/"
fi

# 创建 Info.plist 文件
cat > "${APP_BUNDLE}/Contents/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleExecutable</key>
    <string>${APP_NAME}</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
    <key>CFBundleIdentifier</key>
    <string>com.sdchat.scanner</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>${FULL_APP_NAME}</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>${VERSION}</string>
    <key>CFBundleVersion</key>
    <string>${VERSION}</string>
    <key>LSMinimumSystemVersion</key>
    <string>10.13</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSHumanReadableCopyright</key>
    <string>Copyright © 2023 SDChat Team. All rights reserved.</string>
</dict>
</plist>
EOF

# 创建图标集
ICONSET="${APP_BUNDLE}/Contents/Resources/AppIcon.iconset"
mkdir -p "${ICONSET}"

# 如果有图标文件，复制到图标集
if [ -d "resources/icons" ]; then
    for icon in resources/icons/*.png; do
        if [ -f "${icon}" ]; then
            size=$(identify -format "%wx%h" "${icon}" | cut -d 'x' -f 1)
            cp "${icon}" "${ICONSET}/icon_${size}x${size}.png"
        fi
    done
    
    # 创建 icns 文件
    iconutil -c icns "${ICONSET}" -o "${APP_BUNDLE}/Contents/Resources/AppIcon.icns"
    rm -rf "${ICONSET}"
fi

# 创建README文件
cat > "${PLATFORM_DIR}/README.txt" << EOF
SDChat Scanner ${VERSION}
构建日期: ${BUILD_DATE}
平台: macOS

使用方法:
1. 将 ${FULL_APP_NAME}.app 拖到应用程序文件夹
2. 双击启动应用程序
3. 选择要扫描的目录
4. 点击"开始扫描"按钮

注意事项:
- 首次运行时，应用程序会自动下载最新的扫描规则
- 扫描报告将保存在用户文档目录下的 SDChat-Scanner/reports 文件夹中
- 如果遇到"未知开发者"警告，请在 Finder 中右键点击应用程序，选择"打开"
EOF

# 创建压缩包
echo -e "${GREEN}正在创建压缩包...${NC}"
(cd "${OUTPUT_DIR}" && zip -r "${APP_NAME}-${VERSION}-macos.zip" "$(basename "${PLATFORM_DIR}")")

# 创建 DMG 文件（如果 create-dmg 可用）
if command -v create-dmg &> /dev/null; then
    echo -e "${GREEN}正在创建 DMG 文件...${NC}"
    create-dmg \
        --volname "${FULL_APP_NAME}" \
        --volicon "${APP_BUNDLE}/Contents/Resources/AppIcon.icns" \
        --window-pos 200 120 \
        --window-size 800 400 \
        --icon-size 100 \
        --icon "${FULL_APP_NAME}.app" 200 190 \
        --hide-extension "${FULL_APP_NAME}.app" \
        --app-drop-link 600 185 \
        "${OUTPUT_DIR}/${APP_NAME}-${VERSION}-macos.dmg" \
        "${PLATFORM_DIR}" || echo -e "${YELLOW}警告: DMG 创建失败${NC}"
fi

echo -e "${GREEN}macOS 版本构建完成: ${OUTPUT_DIR}/${APP_NAME}-${VERSION}-macos.zip${NC}"
if [ -f "${OUTPUT_DIR}/${APP_NAME}-${VERSION}-macos.dmg" ]; then
    echo -e "${GREEN}DMG 文件: ${OUTPUT_DIR}/${APP_NAME}-${VERSION}-macos.dmg${NC}"
fi
