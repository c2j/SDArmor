import os
import json
import uuid
from datetime import datetime
from flask import Blueprint, request, jsonify, current_app, send_file
from flask_jwt_extended import jwt_required, get_jwt_identity
from werkzeug.utils import secure_filename
from jsonschema import validate, ValidationError
from sqlalchemy.exc import SQLAlchemyError

from app import db
from app.models import RuleSet, FileType, Pattern
from app.utils.auth import admin_required

# Create blueprint
rules_bp = Blueprint('rules', __name__)

# Rule schema for validation
RULE_SCHEMA = {
    "type": "object",
    "required": ["version", "file_types"],
    "properties": {
        "version": {"type": "string"},
        "file_types": {
            "type": "array",
            "items": {
                "type": "object",
                "required": ["name", "identifiers", "patterns"],
                "properties": {
                    "name": {"type": "string"},
                    "identifiers": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "required": ["type", "pattern"],
                            "properties": {
                                "type": {"type": "string", "enum": ["extension", "content"]},
                                "pattern": {"type": "string"}
                            }
                        }
                    },
                    "patterns": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "required": ["id", "description", "severity", "regex"],
                            "properties": {
                                "id": {"type": "string"},
                                "description": {"type": "string"},
                                "severity": {"type": "string", "enum": ["critical", "high", "medium", "low"]},
                                "regex": {"type": "string"}
                            }
                        }
                    }
                }
            }
        }
    }
}


@rules_bp.route('', methods=['GET'])
def get_rules():
    """Get all rule sets or active rule set"""
    active_only = request.args.get('active', 'false').lower() == 'true'

    query = RuleSet.query
    if active_only:
        query = query.filter_by(is_active=True)

    rule_sets = query.all()

    result = []
    for rule_set in rule_sets:
        rule_data = {
            'id': rule_set.id,
            'name': rule_set.name,
            'version': rule_set.version,
            'description': rule_set.description,
            'is_active': rule_set.is_active,
            'created_at': rule_set.created_at.isoformat(),
            'updated_at': rule_set.updated_at.isoformat(),
            'file_types_count': len(rule_set.file_types),
        }
        result.append(rule_data)

    return jsonify(result)


@rules_bp.route('/<int:rule_set_id>', methods=['GET'])
def get_rule_set(rule_set_id):
    """Get a specific rule set by ID"""
    rule_set = RuleSet.query.get_or_404(rule_set_id)

    # Build response with full rule set data
    file_types_data = []

    for file_type in rule_set.file_types:
        patterns_data = []

        for pattern in file_type.patterns:
            patterns_data.append({
                'id': pattern.id_code,
                'description': pattern.description,
                'severity': pattern.severity,
                'regex': pattern.regex
            })

        file_types_data.append({
            'name': file_type.name,
            'identifiers': file_type.identifiers,
            'patterns': patterns_data
        })

    rule_set_data = {
        'id': rule_set.id,
        'name': rule_set.name,
        'version': rule_set.version,
        'description': rule_set.description,
        'is_active': rule_set.is_active,
        'created_at': rule_set.created_at.isoformat(),
        'updated_at': rule_set.updated_at.isoformat(),
        'file_types': file_types_data
    }

    return jsonify(rule_set_data)


@rules_bp.route('/active', methods=['GET'])
def get_active_rule_set():
    """Get the current active rule set"""
    rule_set = RuleSet.query.filter_by(is_active=True).first_or_404()

    # Build response with full rule set data
    file_types_data = []

    for file_type in rule_set.file_types:
        patterns_data = []

        for pattern in file_type.patterns:
            patterns_data.append({
                'id': pattern.id_code,
                'description': pattern.description,
                'severity': pattern.severity,
                'regex': pattern.regex
            })

        file_types_data.append({
            'name': file_type.name,
            'identifiers': file_type.identifiers,
            'patterns': patterns_data
        })

    rule_set_data = {
        'version': rule_set.version,
        'file_types': file_types_data
    }

    return jsonify(rule_set_data)


@rules_bp.route('', methods=['POST'])
@jwt_required()
@admin_required
def create_rule_set():
    """Create a new rule set"""
    data = request.get_json()

    if not data:
        return jsonify({'error': 'No data provided'}), 400

    # Validate against schema
    try:
        validate(instance=data, schema=RULE_SCHEMA)
    except ValidationError as e:
        return jsonify({'error': f'Invalid rule set format: {str(e)}'}), 400

    try:
        # Create new rule set
        rule_set = RuleSet(
            name=data.get('name', f'Rule Set {datetime.utcnow().isoformat()}'),
            version=data['version'],
            description=data.get('description', ''),
            is_active=data.get('is_active', False)
        )

        # Add file types and patterns
        for ft_data in data['file_types']:
            file_type = FileType(
                name=ft_data['name'],
                identifiers=ft_data['identifiers']
            )

            # Add patterns
            for pattern_data in ft_data['patterns']:
                pattern = Pattern(
                    id_code=pattern_data['id'],
                    description=pattern_data['description'],
                    severity=pattern_data['severity'],
                    regex=pattern_data['regex']
                )
                file_type.patterns.append(pattern)

            rule_set.file_types.append(file_type)

        db.session.add(rule_set)
        db.session.commit()

        return jsonify({
            'id': rule_set.id,
            'message': 'Rule set created successfully'
        }), 201

    except SQLAlchemyError as e:
        db.session.rollback()
        return jsonify({'error': f'Database error: {str(e)}'}), 500


@rules_bp.route('/<int:rule_set_id>', methods=['PUT'])
@jwt_required()
@admin_required
def update_rule_set(rule_set_id):
    """Update an existing rule set"""
    rule_set = RuleSet.query.get_or_404(rule_set_id)
    data = request.get_json()

    if not data:
        return jsonify({'error': 'No data provided'}), 400

    # Validate against schema
    try:
        validate(instance=data, schema=RULE_SCHEMA)
    except ValidationError as e:
        return jsonify({'error': f'Invalid rule set format: {str(e)}'}), 400

    try:
        # Update rule set basic info
        rule_set.name = data.get('name', rule_set.name)
        rule_set.version = data.get('version', rule_set.version)
        rule_set.description = data.get('description', rule_set.description)
        rule_set.is_active = data.get('is_active', rule_set.is_active)

        # Clear existing file types and recreate them
        for file_type in rule_set.file_types:
            db.session.delete(file_type)

        # Add new file types and patterns
        for ft_data in data['file_types']:
            file_type = FileType(
                name=ft_data['name'],
                identifiers=ft_data['identifiers']
            )

            # Add patterns
            for pattern_data in ft_data['patterns']:
                pattern = Pattern(
                    id_code=pattern_data['id'],
                    description=pattern_data['description'],
                    severity=pattern_data['severity'],
                    regex=pattern_data['regex']
                )
                file_type.patterns.append(pattern)

            rule_set.file_types.append(file_type)

        db.session.commit()

        return jsonify({
            'id': rule_set.id,
            'message': 'Rule set updated successfully'
        })

    except SQLAlchemyError as e:
        db.session.rollback()
        return jsonify({'error': f'Database error: {str(e)}'}), 500


@rules_bp.route('/<int:rule_set_id>/activate', methods=['POST'])
@jwt_required()
@admin_required
def activate_rule_set(rule_set_id):
    """Activate a specific rule set and deactivate all others"""
    try:
        # Deactivate all rule sets
        RuleSet.query.update({'is_active': False})

        # Activate the specified rule set
        rule_set = RuleSet.query.get_or_404(rule_set_id)
        rule_set.is_active = True

        db.session.commit()

        return jsonify({
            'message': f'Rule set {rule_set.name} v{rule_set.version} activated successfully'
        })

    except SQLAlchemyError as e:
        db.session.rollback()
        return jsonify({'error': f'Database error: {str(e)}'}), 500


@rules_bp.route('/<int:rule_set_id>', methods=['DELETE'])
@jwt_required()
@admin_required
def delete_rule_set(rule_set_id):
    """Delete a rule set"""
    rule_set = RuleSet.query.get_or_404(rule_set_id)

    if rule_set.is_active:
        return jsonify({'error': 'Cannot delete active rule set'}), 400

    try:
        db.session.delete(rule_set)
        db.session.commit()

        return jsonify({
            'message': f'Rule set {rule_set.name} deleted successfully'
        })

    except SQLAlchemyError as e:
        db.session.rollback()
        return jsonify({'error': f'Database error: {str(e)}'}), 500


@rules_bp.route('/export/<int:rule_set_id>', methods=['GET'])
@jwt_required()
def export_rule_set(rule_set_id):
    """Export a rule set as JSON file"""
    rule_set = RuleSet.query.get_or_404(rule_set_id)

    # Build rule set data
    file_types_data = []

    for file_type in rule_set.file_types:
        patterns_data = []

        for pattern in file_type.patterns:
            patterns_data.append({
                'id': pattern.id_code,
                'description': pattern.description,
                'severity': pattern.severity,
                'regex': pattern.regex
            })

        file_types_data.append({
            'name': file_type.name,
            'identifiers': file_type.identifiers,
            'patterns': patterns_data
        })

    rule_set_data = {
        'version': rule_set.version,
        'file_types': file_types_data
    }

    # Create the file
    filename = f"ruleset_{rule_set.name.replace(' ', '_')}_{rule_set.version}.json"
    file_path = os.path.join(current_app.config['RULES_DIR'], filename)

    with open(file_path, 'w') as f:
        json.dump(rule_set_data, f, indent=2)

    return send_file(
        file_path,
        mimetype='application/json',
        as_attachment=True,
        download_name=filename
    )


@rules_bp.route('/latest', methods=['GET'])
def download_latest_rules():
    """Download the latest (active) rule set as JSON file without authentication"""
    try:
        # 获取当前活跃的规则集
        rule_set = RuleSet.query.filter_by(is_active=True).first()

        if not rule_set:
            return jsonify({
                'error': '没有找到活跃的规则集',
                'message': '请先激活一个规则集'
            }), 404

        # 构建规则集数据
        file_types_data = []

        for file_type in rule_set.file_types:
            patterns_data = []

            for pattern in file_type.patterns:
                patterns_data.append({
                    'id': pattern.id_code,
                    'description': pattern.description,
                    'severity': pattern.severity,
                    'regex': pattern.regex
                })

            file_types_data.append({
                'name': file_type.name,
                'identifiers': file_type.identifiers,
                'patterns': patterns_data
            })

        rule_set_data = {
            'version': rule_set.version,
            'file_types': file_types_data
        }

        # 确保规则目录存在
        rules_dir = current_app.config['RULES_DIR']
        os.makedirs(rules_dir, exist_ok=True)

        # 创建临时文件
        import tempfile

        # 创建临时文件
        temp_file = tempfile.NamedTemporaryFile(delete=False, suffix='.json')
        temp_file_path = temp_file.name

        # 写入规则数据
        with open(temp_file_path, 'w') as f:
            json.dump(rule_set_data, f, indent=2)

        # 添加额外的头信息，指示客户端不要缓存此文件
        filename = f"latest_rules_{rule_set.version}.json"
        response = send_file(
            temp_file_path,
            mimetype='application/json',
            as_attachment=True,
            download_name=filename
        )

        # 添加缓存控制头，确保客户端每次都获取最新版本
        response.headers['Cache-Control'] = 'no-cache, no-store, must-revalidate'
        response.headers['Pragma'] = 'no-cache'
        response.headers['Expires'] = '0'

        return response

    except Exception as e:
        # 记录错误并返回友好的错误消息
        current_app.logger.error(f"下载最新规则时出错: {str(e)}")
        return jsonify({
            'error': '下载规则集时出错',
            'message': str(e)
        }), 500


@rules_bp.route('/import', methods=['POST'])
@jwt_required()
@admin_required
def import_rules():
    """Import rules from uploaded file"""
    if 'file' not in request.files:
        return jsonify({'error': '没有上传文件'}), 400

    file = request.files['file']
    if file.filename == '':
        return jsonify({'error': '没有选择文件'}), 400

    if not file.filename.lower().endswith('.json'):
        return jsonify({'error': '只支持JSON格式的规则文件'}), 400

    try:
        # 读取并解析JSON文件
        content = file.read().decode('utf-8')
        rule_data = json.loads(content)

        # 验证规则格式
        validate(instance=rule_data, schema=RULE_SCHEMA)

        # 检查是否已存在相同版本的规则集
        existing_rule = RuleSet.query.filter_by(version=rule_data['version']).first()
        if existing_rule:
            return jsonify({
                'error': f'版本 {rule_data["version"]} 的规则集已存在',
                'existing_id': existing_rule.id
            }), 409

        # 创建新规则集
        rule_set = RuleSet(
            name=rule_data.get('name', f'导入的规则集 {rule_data["version"]}'),
            version=rule_data['version'],
            description=rule_data.get('description', '通过文件导入的规则集'),
            is_active=request.form.get('activate') == 'on'
        )

        # 如果要激活新规则集，先停用其他规则集
        if rule_set.is_active:
            RuleSet.query.update({'is_active': False})

        # 添加文件类型和规则
        for ft_data in rule_data['file_types']:
            file_type = FileType(
                name=ft_data['name'],
                identifiers=ft_data['identifiers']
            )

            # 添加规则模式
            for pattern_data in ft_data['patterns']:
                pattern = Pattern(
                    id_code=pattern_data['id'],
                    description=pattern_data['description'],
                    severity=pattern_data['severity'],
                    regex=pattern_data['regex']
                )
                file_type.patterns.append(pattern)

            rule_set.file_types.append(file_type)

        db.session.add(rule_set)
        db.session.commit()

        return jsonify({
            'id': rule_set.id,
            'message': f'规则集 "{rule_set.name}" 导入成功',
            'version': rule_set.version,
            'file_types_count': len(rule_set.file_types),
            'patterns_count': sum(len(ft.patterns) for ft in rule_set.file_types)
        }), 201

    except json.JSONDecodeError as e:
        return jsonify({'error': f'JSON格式错误: {str(e)}'}), 400
    except ValidationError as e:
        return jsonify({'error': f'规则格式验证失败: {str(e)}'}), 400
    except SQLAlchemyError as e:
        db.session.rollback()
        return jsonify({'error': f'数据库错误: {str(e)}'}), 500
    except Exception as e:
        db.session.rollback()
        return jsonify({'error': f'导入失败: {str(e)}'}), 500


@rules_bp.route('/sample', methods=['GET'])
def get_sample_rules():
    """Get sample rule file format"""
    sample_rules = {
        "name": "示例安全规则集",
        "version": "1.0.0",
        "description": "这是一个示例规则集，展示了规则文件的标准格式",
        "file_types": [
            {
                "name": "JavaScript Files",
                "identifiers": [
                    {
                        "type": "extension",
                        "pattern": "\\.(js|jsx|ts|tsx)$"
                    },
                    {
                        "type": "content",
                        "pattern": "^\\s*(import|export|require)"
                    }
                ],
                "patterns": [
                    {
                        "id": "JS-001",
                        "description": "不安全的eval函数使用",
                        "severity": "high",
                        "regex": "eval\\s*\\("
                    },
                    {
                        "id": "JS-002",
                        "description": "潜在的XSS漏洞 - innerHTML使用",
                        "severity": "medium",
                        "regex": "\\.innerHTML\\s*="
                    },
                    {
                        "id": "JS-003",
                        "description": "不安全的随机数生成",
                        "severity": "low",
                        "regex": "Math\\.random\\(\\)"
                    }
                ]
            },
            {
                "name": "Python Files",
                "identifiers": [
                    {
                        "type": "extension",
                        "pattern": "\\.py$"
                    },
                    {
                        "type": "content",
                        "pattern": "^#!/usr/bin/env python"
                    }
                ],
                "patterns": [
                    {
                        "id": "PY-001",
                        "description": "不安全的exec函数使用",
                        "severity": "critical",
                        "regex": "exec\\s*\\("
                    },
                    {
                        "id": "PY-002",
                        "description": "SQL注入风险 - 字符串拼接",
                        "severity": "high",
                        "regex": "SELECT.*\\+.*%s"
                    },
                    {
                        "id": "PY-003",
                        "description": "不安全的pickle使用",
                        "severity": "medium",
                        "regex": "pickle\\.loads?\\("
                    }
                ]
            }
        ]
    }

    return jsonify(sample_rules)


@rules_bp.route('/file-types', methods=['POST'])
@jwt_required()
@admin_required
def add_file_type():
    """Add a new file type to a rule set"""
    try:
        data = request.get_json()

        if not data:
            return jsonify({'error': '没有提供数据'}), 400

        ruleset_id = data.get('ruleset_id')
        name = data.get('name')
        identifiers = data.get('identifiers')

        if not all([ruleset_id, name, identifiers]):
            return jsonify({'error': '缺少必需字段'}), 400

        # 验证规则集存在
        ruleset = RuleSet.query.get(ruleset_id)
        if not ruleset:
            return jsonify({'error': '规则集不存在'}), 404

        # 创建新文件类型
        file_type = FileType(
            name=name,
            identifiers=identifiers
        )

        ruleset.file_types.append(file_type)
        db.session.commit()

        return jsonify({
            'id': file_type.id,
            'message': f'文件类型 "{name}" 添加成功'
        }), 201

    except Exception as e:
        db.session.rollback()
        return jsonify({'error': f'添加失败: {str(e)}'}), 500


@rules_bp.route('/file-types/<int:file_type_id>', methods=['PUT'])
@jwt_required()
@admin_required
def update_file_type(file_type_id):
    """Update a file type"""
    try:
        data = request.get_json()

        if not data:
            return jsonify({'error': '没有提供数据'}), 400

        file_type = FileType.query.get(file_type_id)
        if not file_type:
            return jsonify({'error': '文件类型不存在'}), 404

        # 更新字段
        if 'name' in data:
            file_type.name = data['name']
        if 'identifiers' in data:
            file_type.identifiers = data['identifiers']

        db.session.commit()

        return jsonify({
            'id': file_type.id,
            'message': f'文件类型 "{file_type.name}" 更新成功'
        })

    except Exception as e:
        db.session.rollback()
        return jsonify({'error': f'更新失败: {str(e)}'}), 500


@rules_bp.route('/file-types/<int:file_type_id>', methods=['DELETE'])
@jwt_required()
@admin_required
def delete_file_type(file_type_id):
    """Delete a file type"""
    try:
        file_type = FileType.query.get(file_type_id)
        if not file_type:
            return jsonify({'error': '文件类型不存在'}), 404

        name = file_type.name
        db.session.delete(file_type)
        db.session.commit()

        return jsonify({
            'message': f'文件类型 "{name}" 删除成功'
        })

    except Exception as e:
        db.session.rollback()
        return jsonify({'error': f'删除失败: {str(e)}'}), 500


@rules_bp.route('/patterns', methods=['POST'])
@jwt_required()
@admin_required
def add_pattern():
    """Add a new pattern to a file type"""
    try:
        data = request.get_json()

        if not data:
            return jsonify({'error': '没有提供数据'}), 400

        file_type_id = data.get('file_type_id')
        id_code = data.get('id_code')
        description = data.get('description')
        severity = data.get('severity')
        regex = data.get('regex')

        if not all([file_type_id, id_code, description, severity, regex]):
            return jsonify({'error': '缺少必需字段'}), 400

        # 验证文件类型存在
        file_type = FileType.query.get(file_type_id)
        if not file_type:
            return jsonify({'error': '文件类型不存在'}), 404

        # 验证严重性级别
        if severity not in ['critical', 'high', 'medium', 'low']:
            return jsonify({'error': '无效的严重性级别'}), 400

        # 检查ID是否已存在
        existing_pattern = Pattern.query.filter_by(id_code=id_code).first()
        if existing_pattern:
            return jsonify({'error': f'规则ID "{id_code}" 已存在'}), 409

        # 创建新规则模式
        pattern = Pattern(
            id_code=id_code,
            description=description,
            severity=severity,
            regex=regex
        )

        file_type.patterns.append(pattern)
        db.session.commit()

        return jsonify({
            'id': pattern.id,
            'message': f'规则 "{id_code}" 添加成功'
        }), 201

    except Exception as e:
        db.session.rollback()
        return jsonify({'error': f'添加失败: {str(e)}'}), 500


@rules_bp.route('/patterns/<int:pattern_id>', methods=['PUT'])
@jwt_required()
@admin_required
def update_pattern(pattern_id):
    """Update a pattern"""
    try:
        data = request.get_json()

        if not data:
            return jsonify({'error': '没有提供数据'}), 400

        pattern = Pattern.query.get(pattern_id)
        if not pattern:
            return jsonify({'error': '规则不存在'}), 404

        # 更新字段
        if 'id_code' in data:
            # 检查新ID是否已存在（排除当前规则）
            existing = Pattern.query.filter(
                Pattern.id_code == data['id_code'],
                Pattern.id != pattern_id
            ).first()
            if existing:
                return jsonify({'error': f'规则ID "{data["id_code"]}" 已存在'}), 409
            pattern.id_code = data['id_code']

        if 'description' in data:
            pattern.description = data['description']

        if 'severity' in data:
            if data['severity'] not in ['critical', 'high', 'medium', 'low']:
                return jsonify({'error': '无效的严重性级别'}), 400
            pattern.severity = data['severity']

        if 'regex' in data:
            pattern.regex = data['regex']

        db.session.commit()

        return jsonify({
            'id': pattern.id,
            'message': f'规则 "{pattern.id_code}" 更新成功'
        })

    except Exception as e:
        db.session.rollback()
        return jsonify({'error': f'更新失败: {str(e)}'}), 500


@rules_bp.route('/patterns/<int:pattern_id>', methods=['DELETE'])
@jwt_required()
@admin_required
def delete_pattern(pattern_id):
    """Delete a pattern"""
    try:
        pattern = Pattern.query.get(pattern_id)
        if not pattern:
            return jsonify({'error': '规则不存在'}), 404

        id_code = pattern.id_code
        db.session.delete(pattern)
        db.session.commit()

        return jsonify({
            'message': f'规则 "{id_code}" 删除成功'
        })

    except Exception as e:
        db.session.rollback()
        return jsonify({'error': f'删除失败: {str(e)}'}), 500