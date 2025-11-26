# SD Armor (SD盾甲) - 智能源码安全扫描器

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Python](https://img.shields.io/badge/python-3.8%2B-blue.svg)](https://www.python.org/)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)

**SD Armor** (SD盾甲) 是一款高性能的安全漏洞扫描工具，专为检测源代码中的潜在安全问题而设计。它采用桌面客户端 + 后端服务的架构，为开发团队提供全面的代码安全检测和管理解决方案。

## 🛡️ 产品名称

- **中文名称**: SD盾甲
- **英文名称**: SD Armor
- **命名理念**: 像古代战士的盾甲一样，为您的代码提供坚固的安全防护

## 🌟 核心特性

### 🖥️ 跨平台桌面客户端
- 基于 Rust 和 egui/eframe 构建的原生跨平台图形界面
- 高性能3D漏洞热图可视化展示
- 实时扫描进度监控
- 详细的漏洞报告和代码片段展示
- 支持 Windows、macOS 和 Linux 平台

### ☁️ 强大的后端服务
- 基于 Flask 的 RESTful API 服务
- 完整的用户认证和权限管理系统
- 可扩展的安全规则集管理
- 扫描报告存储和分析
- 团队协作和报告共享功能

### 🔍 智能安全检测
- 基于规则的漏洞检测引擎
- 支持多种编程语言的安全规则
- 多线程扫描引擎提供高性能
- 可定制的安全规则集
- 支持正则表达式加速的 Hyperscan 引擎

## 🏗️ 技术架构

```
+------------------+     +------------------+
|   桌面客户端      |     |   后端服务        |
|  (Rust/egui)     |<--->|  (Python/Flask)  |
+------------------+     +------------------+
         |                       |
         v                       v
+------------------+     +------------------+
|   3D可视化组件    |     |   数据库层        |
|   (wgpu)         |     |  (PostgreSQL)    |
+------------------+     +------------------+
```

## 🚀 快速开始

### 系统要求
- **桌面客户端**: Rust 1.70.0+ 和平台特定的GUI渲染依赖
- **后端服务**: Python 3.8+ 和 PostgreSQL 12+
- **推荐**: 安装 Hyperscan 库以获得最佳性能

### 安装指南

#### 后端服务安装
```bash
# 克隆仓库
git clone https://github.com/your-organization/sd-armor.git
cd sd-armor/SC_Server

# 创建虚拟环境
python -m venv venv
source venv/bin/activate  # Windows: venv\Scripts\activate

# 安装依赖
pip install -r requirements.txt

# 配置环境变量
export FLASK_APP=server.py
export FLASK_ENV=development
export SECRET_KEY=your_secret_key
export DATABASE_URI=postgresql://user:password@localhost/sdarmor

# 初始化数据库
python init_db.py

# 启动服务器
python server.py
```

#### 桌面客户端安装
```bash
# 在 sd-armor/Desktop 目录中
cd ../Desktop

# 安装 Hyperscan (可选但推荐)
# macOS:
brew install hyperscan

# Ubuntu/Debian:
sudo apt-get install libhyperscan-dev

# 构建和运行
cargo build --release --features hyperscan_engine
cargo run --release
```

## 📊 功能模块

### 用户管理
- 用户注册、登录和认证
- 基于JWT的令牌管理
- 管理员权限控制
- 密码安全和重置功能

### 规则集管理
- 安全规则的创建、编辑、删除
- 规则集版本控制和激活
- 规则导入/导出功能
- 多种文件类型支持

### 报告管理
- 扫描报告的生成和查看
- 报告编辑和备注功能
- 报告对比和分析
- 多种格式导出支持

### 团队协作
- 报告共享和协作
- 团队成员管理
- 权限控制和审计
- 通知和评论系统

## 🛠️ 开发指南

### 项目结构
```
sd-armor/
├── Desktop/              # 桌面客户端 (Rust)
│   ├── src/              # 源代码
│   ├── Cargo.toml        # 项目配置
│   └── ...
├── SC_Server/            # 后端服务 (Python)
│   ├── app/              # 应用代码
│   │   ├── api/          # API路由
│   │   ├── models/       # 数据模型
│   │   ├── services/     # 业务逻辑
│   │   └── utils/        # 工具函数
│   ├── requirements.txt  # Python依赖
│   └── ...
├── design_v2/            # 设计文档
├── specs/                # 功能规格
└── README.md             # 本文档
```

### API文档
完整的API文档请参考 [OpenAPI规范](SC_Server/contracts/openapi.yaml)

## 🤝 贡献指南

我们欢迎任何形式的贡献！请参考以下步骤：

1. Fork 项目
2. 创建功能分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 开启 Pull Request

## 📄 许可证

本项目采用 MIT 许可证 - 查看 [LICENSE](LICENSE) 文件了解详情

## 📞 联系我们

- 项目官网: [https://sdarmor.com](https://sdarmor.com)
- 问题反馈: [Issues](https://github.com/your-organization/sd-armor/issues)
- 邮箱支持: support@sdarmor.com

---

**SD Armor** - 为您的代码穿上坚固的盾甲，守护每一行代码的安全！