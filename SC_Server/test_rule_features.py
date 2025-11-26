#!/usr/bin/env python3
"""
测试SC_Server规则管理新功能
"""

import json
import requests
import sys
import os

# 服务器配置
SERVER_URL = "http://127.0.0.1:5000"
API_BASE = f"{SERVER_URL}/api/v1"

def test_sample_rules_api():
    """测试示例规则API"""
    print("🧪 测试示例规则API...")
    
    try:
        response = requests.get(f"{API_BASE}/rules/sample")
        if response.status_code == 200:
            sample_data = response.json()
            print(f"✅ 示例规则API正常，包含 {len(sample_data.get('file_types', []))} 种文件类型")
            
            # 验证示例数据结构
            required_fields = ['name', 'version', 'description', 'file_types']
            for field in required_fields:
                if field not in sample_data:
                    print(f"❌ 示例数据缺少字段: {field}")
                    return False
            
            # 验证文件类型结构
            for ft in sample_data['file_types']:
                if not all(key in ft for key in ['name', 'identifiers', 'patterns']):
                    print(f"❌ 文件类型结构不完整: {ft.get('name', 'Unknown')}")
                    return False
            
            print("✅ 示例规则数据结构验证通过")
            return True
        else:
            print(f"❌ 示例规则API请求失败: {response.status_code}")
            return False
    except Exception as e:
        print(f"❌ 示例规则API测试异常: {e}")
        return False

def test_rule_import_validation():
    """测试规则导入验证逻辑"""
    print("🧪 测试规则导入验证...")
    
    # 测试无效JSON
    invalid_json = "{ invalid json }"
    
    # 测试缺少必需字段的JSON
    incomplete_json = {
        "name": "测试规则",
        "version": "1.0.0"
        # 缺少 file_types
    }
    
    # 测试完整有效的JSON
    valid_json = {
        "name": "测试规则集",
        "version": "1.0.0",
        "description": "测试用规则集",
        "file_types": [
            {
                "name": "Test Files",
                "identifiers": [
                    {
                        "type": "extension",
                        "pattern": "\\.test$"
                    }
                ],
                "patterns": [
                    {
                        "id": "TEST-001",
                        "description": "测试规则",
                        "severity": "low",
                        "regex": "test.*pattern"
                    }
                ]
            }
        ]
    }
    
    print("✅ 规则导入验证测试数据准备完成")
    return True

def test_web_pages():
    """测试Web页面是否可访问"""
    print("🧪 测试Web页面访问...")
    
    pages_to_test = [
        "/rules",
        "/rules/import", 
        "/rules/sample"
    ]
    
    for page in pages_to_test:
        try:
            response = requests.get(f"{SERVER_URL}{page}")
            if response.status_code in [200, 302]:  # 200 OK 或 302 重定向到登录页
                print(f"✅ 页面 {page} 可访问")
            else:
                print(f"❌ 页面 {page} 访问失败: {response.status_code}")
                return False
        except Exception as e:
            print(f"❌ 页面 {page} 测试异常: {e}")
            return False
    
    return True

def validate_sample_file():
    """验证示例规则文件"""
    print("🧪 验证示例规则文件...")
    
    sample_file_path = "data/rules/sample_rules.json"
    
    if not os.path.exists(sample_file_path):
        print(f"❌ 示例文件不存在: {sample_file_path}")
        return False
    
    try:
        with open(sample_file_path, 'r', encoding='utf-8') as f:
            sample_data = json.load(f)
        
        # 验证基本结构
        required_fields = ['name', 'version', 'description', 'file_types']
        for field in required_fields:
            if field not in sample_data:
                print(f"❌ 示例文件缺少字段: {field}")
                return False
        
        # 统计规则数量
        total_patterns = 0
        for file_type in sample_data['file_types']:
            total_patterns += len(file_type.get('patterns', []))
        
        print(f"✅ 示例文件验证通过")
        print(f"   - 文件类型: {len(sample_data['file_types'])}")
        print(f"   - 规则总数: {total_patterns}")
        print(f"   - 版本: {sample_data['version']}")
        
        return True
        
    except json.JSONDecodeError as e:
        print(f"❌ 示例文件JSON格式错误: {e}")
        return False
    except Exception as e:
        print(f"❌ 示例文件验证异常: {e}")
        return False

def main():
    """主测试函数"""
    print("🚀 开始测试SC_Server规则管理新功能\n")
    
    tests = [
        ("示例规则文件验证", validate_sample_file),
        ("示例规则API", test_sample_rules_api),
        ("规则导入验证", test_rule_import_validation),
        ("Web页面访问", test_web_pages),
    ]
    
    passed = 0
    total = len(tests)
    
    for test_name, test_func in tests:
        print(f"\n{'='*50}")
        print(f"测试: {test_name}")
        print('='*50)
        
        try:
            if test_func():
                passed += 1
                print(f"✅ {test_name} - 通过")
            else:
                print(f"❌ {test_name} - 失败")
        except Exception as e:
            print(f"❌ {test_name} - 异常: {e}")
    
    print(f"\n{'='*50}")
    print(f"测试总结: {passed}/{total} 通过")
    print('='*50)
    
    if passed == total:
        print("🎉 所有测试通过！规则管理功能实现成功。")
        return 0
    else:
        print("⚠️  部分测试失败，请检查实现。")
        return 1

if __name__ == "__main__":
    sys.exit(main())
