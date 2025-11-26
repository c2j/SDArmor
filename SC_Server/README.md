# SDChat 安全扫描器服务器

## 概述

SDChat 安全扫描器服务器是安全漏洞扫描系统的后端组件。它为SDChat扫描器桌面客户端应用程序提供规则管理、报告存储和API服务。

## 功能特点

- **规则管理**：存储、更新和提供安全扫描规则
- **报告存储**：接收并存储来自客户端应用程序的扫描报告
- **RESTful API**：用于客户端通信的全面API
- **身份验证**：基于JWT的身份验证，确保安全访问
- **管理仪表板**：用于管理规则、用户和报告的Web界面（即将推出）

## 系统要求

- Python 3.8+
- Flask和依赖项（见requirements.txt）
- SQLite（默认）或PostgreSQL数据库

## 安装指南

1. 克隆仓库：
   ```
   git clone <repository-url>
   cd SDChat-SC/SC_Server
   ```

2. 创建并激活虚拟环境：
   ```
   python -m venv venv
   source venv/bin/activate  # 在Windows上使用：venv\Scripts\activate
   ```

3. 安装依赖项：
   ```
   pip install -r requirements.txt
   ```

4. 设置环境变量：
   ```
   export FLASK_APP=server.py
   export FLASK_ENV=development
   export SECRET_KEY=your_secret_key
   export DATABASE_URI=sqlite:///app.db
   ```

5. 初始化数据库：
   ```
   python init_db.py
   ```

6. 启动服务器：
   ```
   python server.py
   ```

## API 文档

服务器提供以下API端点，用于与桌面客户端交互：

### 身份验证 API

| 端点 | 方法 | 描述 | 身份验证 |
|----------|--------|-------------|----------------|
| `/auth/register` | POST | 注册新用户 | 无 |
| `/auth/login` | POST | 身份验证并获取令牌 | 无 |
| `/auth/refresh` | POST | 刷新访问令牌 | JWT (刷新令牌) |
| `/auth/me` | GET | 获取当前用户信息 | JWT |
| `/auth/change-password` | POST | 更改用户密码 | JWT |
| `/auth/users` | GET | 列出所有用户 | JWT (管理员) |
| `/auth/users` | POST | 创建新用户 | JWT (管理员) |
| `/auth/users/<id>` | PUT | 更新用户 | JWT (管理员) |
| `/auth/users/<id>` | DELETE | 删除用户 | JWT (管理员) |

### 规则 API

| 端点 | 方法 | 描述 | 身份验证 |
|----------|--------|-------------|----------------|
| `/rules` | GET | 获取所有规则集 | 无 |
| `/rules/<id>` | GET | 获取特定规则集 | 无 |
| `/rules/active` | GET | 获取活动规则集 | 无 |
| `/rules` | POST | 创建新规则集 | JWT (管理员) |
| `/rules/<id>` | PUT | 更新规则集 | JWT (管理员) |
| `/rules/<id>/activate` | POST | 激活规则集 | JWT (管理员) |
| `/rules/<id>` | DELETE | 删除规则集 | JWT (管理员) |
| `/rules/export/<id>` | GET | 将规则集导出为JSON | JWT |

### 报告 API

| 端点 | 方法 | 描述 | 身份验证 |
|----------|--------|-------------|----------------|
| `/reports` | GET | 获取所有报告 | JWT |
| `/reports/<id>` | GET | 获取特定报告 | JWT |
| `/reports` | POST | 上传新报告 | JWT |
| `/reports/<id>` | DELETE | 删除报告 | JWT |
| `/reports/<id>/export` | GET | 将报告导出为JSON | JWT |
| `/reports/statistics` | GET | 获取报告统计信息 | JWT (管理员) |

### 身份验证

服务器使用两种身份验证方法：

1. **API密钥身份验证**：
   - 为桌面客户端提供简单的身份验证
   - API密钥包含在`X-API-Key`头部中
   - 用于基本操作，如规则下载

2. **JWT身份验证**：
   - 更安全的基于令牌的身份验证
   - 访问令牌作为`Bearer <token>`包含在`Authorization`头部中
   - 刷新令牌用于获取新的访问令牌
   - 敏感操作需要此验证

### 响应格式

所有API响应遵循标准JSON格式：

```json
{
  "data": { ... },  // 响应数据（成功请求）
  "error": "...",   // 错误消息（失败请求）
  "message": "..."  // 成功消息（成功请求）
}
```

HTTP状态码用于指示请求结果：
- 200：成功
- 201：已创建
- 400：错误请求
- 401：未授权
- 403：禁止访问
- 404：未找到
- 500：服务器错误

### 桌面客户端集成

服务器设计为与SDChat安全扫描器桌面客户端集成：

1. **规则分发**：
   - 桌面客户端定期检查规则更新
   - 服务器提供最新的活动规则集
   - 规则版本化用于跟踪和兼容性

2. **报告收集**：
   - 桌面客户端完成后上传扫描报告
   - 服务器存储报告用于分析和历史跟踪
   - 报告可以导出为各种格式

3. **身份验证流程**：
   - 桌面客户端使用API密钥或JWT进行身份验证
   - 服务器验证凭据并提供访问权限
   - 根据用户角色强制执行权限

## 配置

服务器可以使用环境变量或.env文件进行配置。主要配置选项包括：

- `SECRET_KEY`：Flask会话安全的密钥
- `DATABASE_URI`：数据库连接字符串
- `JWT_SECRET_KEY`：JWT令牌生成的密钥
- `RULES_DIR`：规则存储目录
- `REPORTS_DIR`：报告存储目录

查看`.env.example`获取所有可用的配置选项。

## 使用方法

### 启动服务器

使用以下命令运行服务器：

```
python server.py
```

对于生产部署，使用Gunicorn：

```
gunicorn -w 4 -b 0.0.0.0:5000 "app:create_app()"
```

## 开发

### 项目结构

```
SC_Server/
├── app/                  # 主应用程序包
│   ├── api/              # API蓝图和路由处理程序
│   ├── models/           # 数据库模型
│   ├── static/           # 静态文件
│   ├── templates/        # HTML模板
│   └── utils/            # 实用函数
├── data/                 # 数据存储
│   ├── rules/            # 规则定义
│   └── reports/          # 存储的报告
├── server.py             # 服务器入口点
├── init_db.py            # 数据库初始化脚本
├── requirements.txt      # Python依赖项
└── README.md             # 本文件
```

### 运行测试

```
pytest
```

## 许可证

版权所有 © 2023 SDChat安全扫描器团队。保留所有权利。