软件需求说明书（完整版）

1. 系统架构

1.1 组件架构图

```
graph TD
    subgraph 前端界面
        GUI[图形界面] -->|事件| Core
        CLI[命令行] -->|指令| Core
    end

    subgraph 核心引擎
        Core[扫描引擎] --> Scanner[扫描模块]
        Core --> Rules[规则管理器]
        Core --> Report[报告生成器]
        Scanner --> Parser[语法解析器]
    end

    subgraph 服务端
        Rules -->|HTTP/2| RuleServer[规则服务器]
        Report -->|HTTPS| ReportAPI[上报接口]
    end
```

1.2 技术栈拓扑

```
flowchart TB
    subgraph GUI
        direction TB
        egui[egui 0.25] --> wgpu[wgpu 0.18]
        wgpu --> Metal[Metal 2.0]
        wgpu --> Vulkan[Vulkan 1.2]
    end

    subgraph Core
        tokio[Tokio 1.0] --> ThreadPool[Rayon线程池]
        Regex[正则引擎] --> Hyperscan[Hyperscan加速]
    end

    subgraph Network
        reqwest[reqwest 0.11] --> HTTP3[QUIC协议]
    end
```

2. 用户界面设计

2.1 主界面线框图
```
graph LR
    A[侧边导航栏] --> B[扫描配置区]
    A --> C[实时仪表盘]
    A --> D[漏洞详情]

    subgraph 主界面布局
        B -->|包含| E[路径选择器]
        B -->|包含| F[规则筛选器]
        C -->|包含| G[实时3D热力图]
        C -->|包含| H[进度环形图]
        D -->|包含| I[代码差异对比]
    end
```


2.2 核心交互流程

```
sequenceDiagram
    participant User
    participant GUI
    participant Core

    User->>GUI: 点击"新建扫描"
    GUI->>Core: 发送ScanConfig
    Core->>Scanner: 初始化线程池
    loop 扫描过程
        Scanner->>Parser: 解析文件
        Parser->>Rules: 匹配规则
        Rules-->>Scanner: 返回漏洞
        Scanner->>GUI: 推送实时进度
    end
    Scanner->>Core: 生成最终报告
    Core->>GUI: 显示三维热力图
```

3. 详细需求说明
3.1 扫描规则

```
// 规则文件示例：security-rules-v2.json
{
  "version": "2.0",
  "file_types": [
    {
      "name": "Java Web Files",
      "identifiers": [
        {
          "type": "extension",
          "pattern": "\\.(jsp|java)$"
        },
        {
          "type": "content",
          "pattern": "^<%@\\s*page"
        }
      ],
      "patterns": [
        {
          "id": "CWE-434",
          "description": "未验证的文件上传类型",
          "severity": "high",
          "regex": "FileUtils\\.copyFile\\("
        },
        {
          "id": "CWE-22",
          "description": "路径遍历风险",
          "severity": "critical",
          "regex": "File\\.createTempFile\\s*\\([^)]+"
        }
      ]
    },
    {
      "name": "XML Config Files",
      "identifiers": [
        {
          "type": "extension",
          "pattern": "\\.xml$"
        },
        {
          "type": "content",
          "pattern": "^<\\?xml\\s+version"
        }
      ],
      "patterns": [
        {
          "id": "CWE-611",
          "description": "XXE外部实体注入",
          "severity": "critical",
          "regex": "<!ENTITY\\s+\\w+\\s+SYSTEM\\s+[\"']"
        }
      ]
    }
  ]
}
```

4. 安全要求

4.1 通信安全协议

```
graph LR
    Client[客户端] -->|1. TLS 1.3| Gateway[API网关]
    Gateway -->|2. mTLS| RuleServer[规则服务器]
    Gateway -->|3. AES-256-GCM| Database[漏洞数据库]
```


5. 部署方案

5.1 多平台打包配置
toml
复制
# Cross-compile 配置
[target.x86_64-pc-windows-msvc]
deploy = "msi"
webview = { version = "1.0", integration = "edge" }

[target.aarch64-apple-darwin]
deploy = "dmg"
metal = true

[target.x86_64-unknown-linux-gnu]
deploy = "appimage"
vulkan = true
5.2 更新机制
rust
复制
async fn check_update(current: &str) -> Result<UpdateInfo> {
    let signed = reqwest::get(UPDATE_URL)
        .await?
        .json::<SignedResponse>()
        .await?;

    // 要求Ed25519签名验证
    verify_signature(&signed)?;

    if semver::compare(&signed.version, current) == Ordering::Greater {
        Ok(signed.payload)
    } else {
        Err(Error::NoUpdate)
    }
}
6. 附录



6.2 关键数据结构
rust
复制
#[derive(Serialize)]
struct ScanResult {
    hotspots: Vec<Hotspot>, // 三维坐标数据
    code_snippets: Vec<Snippet>,
    stats: Stats,
}

#[derive(Debug)]
struct Hotspot {
    x: f32,
    y: f32,
    z: f32,
    severity: Severity,
    file_path: PathBuf,
}
