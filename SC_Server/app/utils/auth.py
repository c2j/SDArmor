from functools import wraps
from flask import jsonify, request
from flask_jwt_extended import get_jwt, verify_jwt_in_request

def admin_required(fn):
    """Decorator to require admin privileges for a route"""
    @wraps(fn)
    def wrapper(*args, **kwargs):
        verify_jwt_in_request()
        claims = get_jwt()
        if claims.get("is_admin"):
            return fn(*args, **kwargs)
        else:
            return jsonify({"error": "Admin privileges required"}), 403
    return wrapper

def api_key_required(fn):
    """Decorator to require a valid API key for a route"""
    @wraps(fn)
    def wrapper(*args, **kwargs):
        api_key = request.headers.get('X-API-Key')
        if not api_key:
            return jsonify({"error": "API key is required"}), 401
            
        # In a production app, check the API key against a database
        # For now, we'll use a simple environment variable check
        from flask import current_app
        import os
        
        valid_api_keys = os.environ.get('VALID_API_KEYS', '').split(',')
        if not valid_api_keys or api_key not in valid_api_keys:
            return jsonify({"error": "Invalid API key"}), 401
            
        return fn(*args, **kwargs)
    return wrapper

def get_current_user():
    """Get the current user from the JWT token"""
    from app.models import User
    from flask_jwt_extended import get_jwt_identity
    
    user_id = get_jwt_identity()
    if not user_id:
        return None
        
    return User.query.get(user_id)

def hash_password(password):
    """Hash a password using werkzeug's security functions"""
    from werkzeug.security import generate_password_hash
    return generate_password_hash(password)

def verify_password(password_hash, password):
    """Verify a password against a hash"""
    from werkzeug.security import check_password_hash
    return check_password_hash(password_hash, password)

def create_tokens_for_user(user):
    """Create access and refresh tokens for a user"""
    from flask_jwt_extended import create_access_token, create_refresh_token
    
    access_token = create_access_token(
        identity=user.id,
        additional_claims={'is_admin': user.is_admin}
    )
    refresh_token = create_refresh_token(identity=user.id)
    
    return {
        'access_token': access_token,
        'refresh_token': refresh_token
    }