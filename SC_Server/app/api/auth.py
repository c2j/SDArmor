import datetime
from flask import Blueprint, request, jsonify, current_app
from flask_jwt_extended import (
    create_access_token, create_refresh_token, jwt_required,
    get_jwt_identity, get_jwt
)
from sqlalchemy.exc import SQLAlchemyError, IntegrityError
from werkzeug.security import check_password_hash

from app import db, jwt
from app.models import User
from app.utils.auth import admin_required

# Create blueprint
auth_bp = Blueprint('auth', __name__)


@auth_bp.route('/register', methods=['POST'])
def register():
    """Register a new user"""
    data = request.get_json()
    
    if not data:
        return jsonify({'error': 'No data provided'}), 400
    
    # Validate required fields
    required_fields = ['username', 'email', 'password']
    for field in required_fields:
        if field not in data:
            return jsonify({'error': f'Missing required field: {field}'}), 400
    
    # Check if username or email already exists
    if User.query.filter_by(username=data['username']).first():
        return jsonify({'error': 'Username already exists'}), 409
    
    if User.query.filter_by(email=data['email']).first():
        return jsonify({'error': 'Email already exists'}), 409
    
    try:
        # Create new user
        user = User(
            username=data['username'],
            email=data['email']
        )
        user.set_password(data['password'])
        
        db.session.add(user)
        db.session.commit()
        
        return jsonify({
            'message': 'User registered successfully',
            'user_id': user.id
        }), 201
        
    except IntegrityError:
        db.session.rollback()
        return jsonify({'error': 'Username or email already exists'}), 409
    except SQLAlchemyError as e:
        db.session.rollback()
        return jsonify({'error': f'Database error: {str(e)}'}), 500


@auth_bp.route('/login', methods=['POST'])
def login():
    """Authenticate user and return tokens"""
    data = request.get_json()
    
    if not data:
        return jsonify({'error': 'No data provided'}), 400
    
    # Validate required fields
    if 'username' not in data or 'password' not in data:
        return jsonify({'error': 'Username and password are required'}), 400
    
    # Find user by username
    user = User.query.filter_by(username=data['username']).first()
    
    # Check if user exists and password is correct
    if not user or not user.check_password(data['password']):
        return jsonify({'error': 'Invalid username or password'}), 401
    
    # Check if user is active
    if not user.is_active:
        return jsonify({'error': 'Account is disabled'}), 403
    
    # Create tokens
    access_token = create_access_token(
        identity=user.id,
        additional_claims={'is_admin': user.is_admin}
    )
    refresh_token = create_refresh_token(identity=user.id)
    
    return jsonify({
        'access_token': access_token,
        'refresh_token': refresh_token,
        'user': {
            'id': user.id,
            'username': user.username,
            'email': user.email,
            'is_admin': user.is_admin
        }
    })


@auth_bp.route('/refresh', methods=['POST'])
@jwt_required(refresh=True)
def refresh():
    """Refresh access token"""
    current_user_id = get_jwt_identity()
    
    # Get user to check if they're still active and get admin status
    user = User.query.get(current_user_id)
    
    if not user or not user.is_active:
        return jsonify({'error': 'User is inactive or deleted'}), 401
    
    # Create new access token
    access_token = create_access_token(
        identity=current_user_id,
        additional_claims={'is_admin': user.is_admin}
    )
    
    return jsonify({'access_token': access_token})


@auth_bp.route('/me', methods=['GET'])
@jwt_required()
def get_current_user():
    """Get current user info"""
    current_user_id = get_jwt_identity()
    user = User.query.get_or_404(current_user_id)
    
    return jsonify({
        'id': user.id,
        'username': user.username,
        'email': user.email,
        'is_admin': user.is_admin,
        'created_at': user.created_at.isoformat()
    })


@auth_bp.route('/change-password', methods=['POST'])
@jwt_required()
def change_password():
    """Change user password"""
    data = request.get_json()
    
    if not data:
        return jsonify({'error': 'No data provided'}), 400
    
    # Validate required fields
    required_fields = ['current_password', 'new_password']
    for field in required_fields:
        if field not in data:
            return jsonify({'error': f'Missing required field: {field}'}), 400
    
    current_user_id = get_jwt_identity()
    user = User.query.get_or_404(current_user_id)
    
    # Verify current password
    if not user.check_password(data['current_password']):
        return jsonify({'error': 'Current password is incorrect'}), 401
    
    # Set new password
    user.set_password(data['new_password'])
    db.session.commit()
    
    return jsonify({'message': 'Password changed successfully'})


@auth_bp.route('/users', methods=['GET'])
@jwt_required()
@admin_required
def list_users():
    """List all users (admin only)"""
    users = User.query.all()
    
    users_data = []
    for user in users:
        users_data.append({
            'id': user.id,
            'username': user.username,
            'email': user.email,
            'is_admin': user.is_admin,
            'is_active': user.is_active,
            'created_at': user.created_at.isoformat()
        })
    
    return jsonify(users_data)


@auth_bp.route('/users', methods=['POST'])
@jwt_required()
@admin_required
def create_user():
    """Create a new user (admin only)"""
    data = request.get_json()
    
    if not data:
        return jsonify({'error': 'No data provided'}), 400
    
    # Validate required fields
    required_fields = ['username', 'email', 'password']
    for field in required_fields:
        if field not in data:
            return jsonify({'error': f'Missing required field: {field}'}), 400
    
    try:
        user = User(
            username=data['username'],
            email=data['email'],
            is_admin=data.get('is_admin', False),
            is_active=data.get('is_active', True)
        )
        user.set_password(data['password'])
        
        db.session.add(user)
        db.session.commit()
        
        return jsonify({
            'message': 'User created successfully',
            'user_id': user.id
        }), 201
        
    except IntegrityError:
        db.session.rollback()
        return jsonify({'error': 'Username or email already exists'}), 409
    except SQLAlchemyError as e:
        db.session.rollback()
        return jsonify({'error': f'Database error: {str(e)}'}), 500


@auth_bp.route('/users/<int:user_id>', methods=['PUT'])
@jwt_required()
@admin_required
def update_user(user_id):
    """Update user (admin only)"""
    user = User.query.get_or_404(user_id)
    data = request.get_json()
    
    if not data:
        return jsonify({'error': 'No data provided'}), 400
    
    try:
        # Update fields if provided
        if 'username' in data:
            user.username = data['username']
        if 'email' in data:
            user.email = data['email']
        if 'is_admin' in data:
            user.is_admin = data['is_admin']
        if 'is_active' in data:
            user.is_active = data['is_active']
        if 'password' in data:
            user.set_password(data['password'])
        
        db.session.commit()
        
        return jsonify({
            'message': 'User updated successfully'
        })
        
    except IntegrityError:
        db.session.rollback()
        return jsonify({'error': 'Username or email already exists'}), 409
    except SQLAlchemyError as e:
        db.session.rollback()
        return jsonify({'error': f'Database error: {str(e)}'}), 500


@auth_bp.route('/users/<int:user_id>', methods=['DELETE'])
@jwt_required()
@admin_required
def delete_user(user_id):
    """Delete user (admin only)"""
    user = User.query.get_or_404(user_id)
    
    # Cannot delete yourself
    current_user_id = get_jwt_identity()
    if user_id == current_user_id:
        return jsonify({'error': 'Cannot delete yourself'}), 400
    
    try:
        db.session.delete(user)
        db.session.commit()
        
        return jsonify({
            'message': 'User deleted successfully'
        })
        
    except SQLAlchemyError as e:
        db.session.rollback()
        return jsonify({'error': f'Database error: {str(e)}'}), 500


# JWT token handling
@jwt.token_in_blocklist_loader
def check_if_token_revoked(jwt_header, jwt_payload):
    jti = jwt_payload["jti"]
    # In a real app, check against a token blocklist in database or Redis
    # For now, we'll just return False (no tokens are revoked)
    return False