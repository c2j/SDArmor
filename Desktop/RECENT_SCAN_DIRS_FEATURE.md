# 记住最后一次选择的扫描目录功能

## 功能概述

实现了记住最后一次选择的扫描目录功能，用户下次启动应用时会自动加载上次选择的目录，并提供最近扫描目录的快速选择功能。

## 实现的功能

### 1. 自动加载默认扫描目录
- ✅ 应用启动时自动加载上次选择的扫描目录
- ✅ 如果没有历史记录，显示"未选择扫描目录"

### 2. 保存选择的目录
- ✅ 用户选择新目录时自动保存为默认目录
- ✅ 配置持久化到配置文件中

### 3. 最近扫描目录列表
- ✅ 维护最近扫描的目录列表（最多10个）
- ✅ 在扫描配置界面显示最近5个目录
- ✅ 支持快速选择最近的目录

### 4. 用户友好的界面
- ✅ 长路径自动截断显示
- ✅ 工具提示显示完整路径
- ✅ 复制路径到剪贴板功能
- ✅ 滚动区域支持多个目录

## 技术实现

### 配置结构利用
使用现有的 `PathsConfig` 结构体：
```rust
pub struct PathsConfig {
    pub default_scan_dir: Option<PathBuf>,  // 默认扫描目录
    pub recent_scans: Vec<PathBuf>,          // 最近扫描列表
    // ... 其他字段
}
```

### 核心修改

#### 1. 应用初始化 (`Desktop/src/main.rs`)
```rust
// 从配置中加载默认扫描目录
let default_scan_path = config_manager.config().paths.default_scan_dir.clone();

Self {
    scan_path: default_scan_path,  // 设置为配置中的默认目录
    // ... 其他字段
}
```

#### 2. 目录选择逻辑
```rust
// 设置文件对话框的默认目录
let mut dialog = rfd::FileDialog::new();
if let Some(current_path) = &self.scan_path {
    dialog = dialog.set_directory(current_path);
}

if let Some(path) = dialog.pick_folder() {
    // 保存为默认扫描目录
    self.config_manager.config_mut().paths.default_scan_dir = Some(path.clone());
    
    // 添加到最近扫描列表
    self.add_to_recent_scans(path);
    
    // 保存配置
    self.config_manager.save()?;
}
```

#### 3. 最近目录管理
```rust
fn add_to_recent_scans(&mut self, path: PathBuf) {
    let recent_scans = &mut self.config_manager.config_mut().paths.recent_scans;
    
    // 如果路径已存在，先移除它
    recent_scans.retain(|p| p != &path);
    
    // 添加到列表开头
    recent_scans.insert(0, path);
    
    // 限制列表长度为10个
    if recent_scans.len() > 10 {
        recent_scans.truncate(10);
    }
}
```

#### 4. UI界面增强
- 显示最近扫描目录的滚动列表
- 支持点击快速选择
- 路径截断显示（超过50字符）
- 复制路径功能

## 用户体验改进

### 启动体验
- 首次使用：显示"未选择扫描目录"
- 再次使用：自动加载上次选择的目录

### 目录选择体验
- 文件对话框默认打开上次选择的目录
- 选择新目录后立即保存和生效

### 快速访问
- 最近5个目录显示在界面上
- 一键选择常用目录
- 支持复制路径到剪贴板

## 界面布局

```
扫描配置
├── 目标路径选择
│   ├── [选择目录] 按钮
│   └── 当前选择: /path/to/directory
├── 最近扫描的目录:
│   ├── [/recent/path/1] [📋]
│   ├── [/recent/path/2] [📋]
│   ├── [/recent/path/3] [📋]
│   └── ...
└── 其他配置选项...
```

## 配置文件示例

```json
{
  "paths": {
    "default_scan_dir": "/Users/username/Projects/MyApp",
    "recent_scans": [
      "/Users/username/Projects/MyApp",
      "/Users/username/Documents/Code",
      "/Users/username/Desktop/TestProject",
      "/opt/projects/webapp"
    ]
  }
}
```

## 兼容性说明

- ✅ **向后兼容** - 现有配置文件会自动适配
- ✅ **渐进增强** - 没有历史记录时正常工作
- ✅ **跨平台** - 支持不同操作系统的路径格式

## 测试建议

### 基本功能测试
1. 首次启动应用，验证无默认目录时的行为
2. 选择一个目录，重启应用，验证是否自动加载
3. 选择多个不同目录，验证最近列表的更新

### 界面测试
1. 验证长路径的截断显示
2. 测试复制路径功能
3. 验证滚动区域在多个目录时的表现

### 边界情况测试
1. 目录被删除后的处理
2. 权限不足的目录处理
3. 配置文件损坏时的恢复

## 后续优化建议

1. **智能排序** - 按使用频率排序最近目录
2. **目录验证** - 检查目录是否仍然存在
3. **收藏夹功能** - 允许用户标记常用目录
4. **搜索功能** - 在历史目录中搜索
5. **导入/导出** - 支持配置的导入导出
