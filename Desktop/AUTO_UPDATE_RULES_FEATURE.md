# 自动更新规则功能

## 功能概述

实现了自动检查并更新扫描规则的功能，用户可以通过界面上的检查框控制是否启用自动更新，系统会定期检查并下载最新的扫描规则。

## 实现的功能

### 1. 自动更新规则检查框
- ✅ 在扫描配置界面添加了"自动检查并更新扫描规则"复选框
- ✅ 配置持久化保存和加载
- ✅ 显示最后更新时间

### 2. 启动时自动检查
- ✅ 应用启动时检查是否启用了自动更新
- ✅ 如果启用且规则过期（超过24小时），自动下载最新规则

### 3. 定期检查机制
- ✅ 在应用运行期间每小时检查一次规则更新
- ✅ 避免重复下载和频繁检查

### 4. 智能更新逻辑
- ✅ 检查最后更新时间，只在需要时更新
- ✅ 避免在已经下载规则时重复检查
- ✅ 更新成功后记录时间戳

## 技术实现

### 配置利用
使用现有的 `ServerConfig` 结构体中的字段：
```rust
pub struct ServerConfig {
    pub auto_update_rules: bool,        // 自动更新规则开关
    pub last_rule_update: Option<String>, // 最后更新时间
    // ... 其他字段
}
```

### 核心修改

#### 1. UI界面增强 (`Desktop/src/main.rs`)
```rust
// 规则更新设置
ui.heading("规则设置");
let mut auto_update_rules = self.config_manager.config().server.auto_update_rules;
if ui.checkbox(&mut auto_update_rules, "自动检查并更新扫描规则").changed() {
    self.config_manager.config_mut().server.auto_update_rules = auto_update_rules;
    self.config_manager.save()?;
}

// 显示最后更新时间
if let Some(last_update) = &self.config_manager.config().server.last_rule_update {
    ui.label(format!("最后更新: {}", last_update));
}
```

#### 2. 应用初始化检查
```rust
// 如果启用了自动更新规则，检查是否需要更新
if app.config_manager.config().server.auto_update_rules {
    app.check_and_update_rules_if_needed();
}
```

#### 3. 定期检查机制
```rust
// 在UI更新循环中调用
fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
    self.periodic_rule_check(); // 定期检查规则更新
    // ... 其他UI更新逻辑
}
```

#### 4. 智能更新逻辑
```rust
fn check_and_update_rules_if_needed(&mut self) {
    // 检查是否启用自动更新
    if !self.config_manager.config().server.auto_update_rules {
        return;
    }

    // 检查最后更新时间，如果超过24小时则更新
    let should_update = match &self.config_manager.config().server.last_rule_update {
        Some(last_update_str) => {
            match chrono::DateTime::parse_from_rfc3339(last_update_str) {
                Ok(last_update) => {
                    let now = chrono::Utc::now();
                    let duration = now.signed_duration_since(last_update.with_timezone(&chrono::Utc));
                    duration.num_hours() >= 24
                },
                Err(_) => true,
            }
        },
        None => true,
    };

    if should_update {
        self.download_latest_rules();
    }
}
```

#### 5. 定期检查优化
```rust
fn periodic_rule_check(&mut self) {
    // 每小时检查一次，避免频繁检查
    let should_check = match self.last_rule_check {
        Some(last_check) => {
            now.duration_since(last_check).as_secs() >= 3600 // 1小时
        },
        None => true,
    };

    if should_check {
        self.last_rule_check = Some(now);
        // 执行规则更新检查逻辑
    }
}
```

## 用户体验

### 界面布局
```
扫描配置
├── 目标路径选择
├── 最近扫描的目录
├── 扫描规则信息
├── 规则设置 (新增)
│   ├── ☐ 自动检查并更新扫描规则
│   └── 最后更新: 2024-01-01 12:00:00
├── 上传设置
│   └── ☐ 自动上传扫描报告到服务器
└── [开始扫描] 按钮
```

### 用户控制
- **启用自动更新**：勾选复选框，系统自动管理规则更新
- **禁用自动更新**：取消勾选，用户需要手动点击"更新规则"按钮
- **透明显示**：显示最后更新时间，让用户了解规则状态

### 更新策略
- **启动检查**：应用启动时检查规则是否过期
- **定期检查**：运行期间每小时检查一次
- **智能更新**：只在规则过期（24小时）时才更新
- **避免冲突**：不在手动下载进行时执行自动更新

## 配置文件示例

```json
{
  "server": {
    "base_url": "http://127.0.0.1:5000/api/v1",
    "auto_update_rules": true,
    "auto_upload_reports": false,
    "last_rule_update": "2024-01-01T12:00:00Z"
  }
}
```

## 日志输出

```
INFO: Application initialized with Tokio runtime and default rules
INFO: Auto-updating rules due to schedule
INFO: Downloading latest rules from http://127.0.0.1:5000/api/v1/rules/latest
INFO: Rules downloaded and loaded successfully
INFO: Periodic rule check: rules are up to date
```

## 兼容性说明

- ✅ **向后兼容** - 现有配置文件自动适配
- ✅ **默认安全** - 默认启用自动更新（配置中已设置）
- ✅ **用户控制** - 用户可以随时启用/禁用
- ✅ **网络友好** - 智能检查避免频繁网络请求

## 错误处理

- **网络错误**：自动更新失败不影响应用正常使用
- **解析错误**：时间戳解析失败时默认需要更新
- **下载冲突**：避免在手动下载时执行自动更新
- **配置错误**：保存配置失败时记录警告日志

## 测试建议

### 基本功能测试
1. 启用自动更新，重启应用，验证是否自动检查
2. 禁用自动更新，验证不会自动下载
3. 修改最后更新时间，验证过期检查逻辑

### 定期检查测试
1. 长时间运行应用，验证每小时检查机制
2. 验证不会在下载进行时重复检查
3. 测试网络断开时的错误处理

### 界面测试
1. 验证复选框状态与配置同步
2. 测试最后更新时间的显示
3. 验证设置修改后的立即保存

## 性能影响

- ✅ **最小影响** - 检查逻辑轻量级，不影响UI响应
- ✅ **网络优化** - 智能检查避免不必要的网络请求
- ✅ **内存友好** - 不增加显著内存使用
- ✅ **CPU友好** - 时间检查计算开销极小
