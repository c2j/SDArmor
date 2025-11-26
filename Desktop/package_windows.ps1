# Windows 打包脚本
# 用于创建 Windows 版本的 SDChat Scanner 安装包

# 获取版本信息
$version = (Select-String -Path "Cargo.toml" -Pattern '^version = "(.+)"').Matches.Groups[1].Value
$appName = "sdchat-scanner"
$fullAppName = "SDChat Scanner"
$buildDate = Get-Date -Format "yyyy-MM-dd"
$outputDir = ".\dist"

# 创建输出目录
if (-not (Test-Path $outputDir)) {
    New-Item -ItemType Directory -Path $outputDir | Out-Null
}

Write-Host "========================================="
Write-Host "  SDChat Scanner Windows 打包脚本 v$version"
Write-Host "========================================="
Write-Host "构建日期: $buildDate"
Write-Host ""

# 检查必要的工具
function Check-Tool {
    param (
        [string]$toolName,
        [string]$command
    )
    
    try {
        Invoke-Expression "$command" | Out-Null
        Write-Host "✓ $toolName 已安装" -ForegroundColor Green
    }
    catch {
        Write-Host "✗ $toolName 未安装" -ForegroundColor Red
        Write-Host "请先安装 $toolName" -ForegroundColor Yellow
        exit 1
    }
}

Check-Tool "Rust" "rustc --version"
Check-Tool "Cargo" "cargo --version"

# 构建 Windows 版本
Write-Host "`n正在构建 Windows 版本..." -ForegroundColor Blue

# 检查目标平台
$targetInstalled = rustup target list --installed | Select-String "x86_64-pc-windows-msvc"
if (-not $targetInstalled) {
    Write-Host "目标平台 x86_64-pc-windows-msvc 未安装，正在安装..." -ForegroundColor Yellow
    rustup target add x86_64-pc-windows-msvc
}
else {
    Write-Host "目标平台 x86_64-pc-windows-msvc 已安装" -ForegroundColor Green
}

# 构建
Write-Host "执行 cargo build --release --target x86_64-pc-windows-msvc --features file_dialog,static_link" -ForegroundColor Yellow
$env:RUSTFLAGS = "-C target-feature=+crt-static"
cargo build --release --target x86_64-pc-windows-msvc --features file_dialog,static_link

# 创建输出目录
$platformDir = "$outputDir\$appName-$version-windows"
if (Test-Path $platformDir) {
    Remove-Item -Path $platformDir -Recurse -Force
}
New-Item -ItemType Directory -Path $platformDir | Out-Null

# 复制二进制文件
Copy-Item "target\x86_64-pc-windows-msvc\release\$appName.exe" -Destination "$platformDir\"

# 复制必要的资源文件
if (Test-Path "resources") {
    Copy-Item -Path "resources" -Destination "$platformDir\" -Recurse
}

# 创建README文件
@"
SDChat Scanner $version
构建日期: $buildDate
平台: Windows

使用方法:
1. 运行 $appName.exe 启动应用程序
2. 选择要扫描的目录
3. 点击"开始扫描"按钮

注意事项:
- 首次运行时，应用程序会自动下载最新的扫描规则
- 扫描报告将保存在用户文档目录下的 SDChat-Scanner/reports 文件夹中
"@ | Out-File -FilePath "$platformDir\README.txt" -Encoding utf8

# 创建压缩包
Write-Host "正在创建压缩包..." -ForegroundColor Green
Compress-Archive -Path $platformDir -DestinationPath "$outputDir\$appName-$version-windows.zip" -Force

Write-Host "Windows 版本构建完成: $outputDir\$appName-$version-windows.zip" -ForegroundColor Green
