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