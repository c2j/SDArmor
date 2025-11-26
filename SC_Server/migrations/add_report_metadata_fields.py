#!/usr/bin/env python3
"""
数据库迁移脚本：添加报告元数据字段

此脚本为Report表添加以下字段：
- application_name: 归属应用
- uploader_name: 上传人
- rule_version: 扫描规则版本
- notes: 备注信息

使用方法：
1. 确保服务器已停止运行
2. 执行此脚本: python migrations/add_report_metadata_fields.py
3. 重新启动服务器
"""

import os
import sys
import sqlite3
from datetime import datetime

# 添加项目根目录到Python路径
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))

from app import create_app

def main():
    # 创建应用实例以获取配置
    app = create_app()
    
    # 获取数据库URI
    db_uri = app.config['SQLALCHEMY_DATABASE_URI']
    
    if not db_uri.startswith('sqlite:///'):
        print("此迁移脚本仅支持SQLite数据库")
        print(f"检测到的数据库URI: {db_uri}")
        print("如果您使用的是PostgreSQL或其他数据库，请手动添加字段")
        return 1
    
    # 提取SQLite数据库文件路径
    db_path = db_uri.replace('sqlite:///', '')
    
    # 如果路径是相对路径，转换为绝对路径
    if not os.path.isabs(db_path):
        db_path = os.path.join(app.instance_path, db_path)
    
    print(f"数据库路径: {db_path}")
    
    # 检查数据库文件是否存在
    if not os.path.exists(db_path):
        print(f"错误: 数据库文件不存在: {db_path}")
        return 1
    
    try:
        # 连接到数据库
        conn = sqlite3.connect(db_path)
        cursor = conn.cursor()
        
        # 检查report表是否存在
        cursor.execute("SELECT name FROM sqlite_master WHERE type='table' AND name='report'")
        if not cursor.fetchone():
            print("错误: report表不存在")
            conn.close()
            return 1
        
        # 检查字段是否已存在
        cursor.execute("PRAGMA table_info(report)")
        columns = [column[1] for column in cursor.fetchall()]
        
        # 需要添加的字段
        new_columns = {
            'application_name': 'TEXT',
            'uploader_name': 'TEXT',
            'rule_version': 'TEXT',
            'notes': 'TEXT'
        }
        
        # 添加不存在的字段
        for column_name, column_type in new_columns.items():
            if column_name not in columns:
                print(f"添加字段: {column_name} ({column_type})")
                cursor.execute(f"ALTER TABLE report ADD COLUMN {column_name} {column_type}")
            else:
                print(f"字段已存在: {column_name}")
        
        # 提交更改
        conn.commit()
        
        # 记录迁移时间
        migration_time = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
        print(f"迁移完成: {migration_time}")
        
        # 关闭连接
        conn.close()
        return 0
        
    except sqlite3.Error as e:
        print(f"数据库错误: {e}")
        return 1
    except Exception as e:
        print(f"未知错误: {e}")
        return 1

if __name__ == "__main__":
    sys.exit(main())
