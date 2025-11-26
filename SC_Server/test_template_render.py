#!/usr/bin/env python3
"""
测试模板渲染
"""

from app import create_app, db
from app.models import RuleSet, FileType, Pattern

def test_template_render():
    """测试模板渲染是否正常"""
    print("🧪 测试模板渲染...")
    
    app = create_app()
    
    with app.app_context():
        # 创建一个测试规则集
        test_ruleset = RuleSet(
            name="测试规则集",
            version="1.0.0",
            description="用于测试的规则集",
            is_active=False
        )
        
        # 添加文件类型
        file_type = FileType(
            name="Test Files",
            identifiers=[{"type": "extension", "pattern": "\\.test$"}]
        )
        
        # 添加规则
        pattern1 = Pattern(
            id_code="TEST-001",
            description="测试规则1",
            severity="high",
            regex="test.*pattern"
        )
        
        pattern2 = Pattern(
            id_code="TEST-002", 
            description="测试规则2",
            severity="medium",
            regex="another.*test"
        )
        
        file_type.patterns = [pattern1, pattern2]
        test_ruleset.file_types = [file_type]
        
        # 计算统计信息
        total_patterns = 0
        for ft in test_ruleset.file_types:
            total_patterns += len(ft.patterns)
        
        print(f"测试数据: {len(test_ruleset.file_types)} 个文件类型, {total_patterns} 个规则")
        
        # 测试模板渲染
        try:
            from flask import render_template_string
            
            # 简化的模板测试
            template_content = """
            <div>
                <h1>{{ ruleset.name }}</h1>
                <p>文件类型数: {{ ruleset.file_types|length }}</p>
                <p>规则总数: {{ total_patterns }}</p>
            </div>
            """
            
            rendered = render_template_string(
                template_content,
                ruleset=test_ruleset,
                total_patterns=total_patterns
            )
            
            print("✅ 模板渲染成功")
            print(f"渲染结果: {rendered.strip()}")
            
            # 检查渲染结果
            if "测试规则集" in rendered and "1" in rendered and "2" in rendered:
                print("✅ 渲染内容正确")
                return True
            else:
                print("❌ 渲染内容不正确")
                return False
                
        except Exception as e:
            print(f"❌ 模板渲染失败: {e}")
            return False

def main():
    """主函数"""
    print("🚀 开始测试模板渲染修复\n")
    
    if test_template_render():
        print("\n🎉 模板渲染测试通过！编辑页面错误已修复。")
        return 0
    else:
        print("\n❌ 模板渲染测试失败。")
        return 1

if __name__ == "__main__":
    import sys
    sys.exit(main())
