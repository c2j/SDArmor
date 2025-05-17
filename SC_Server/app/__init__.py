import os
from flask import Flask
from flask_cors import CORS
from flask_jwt_extended import JWTManager
from flask_sqlalchemy import SQLAlchemy
from datetime import timedelta
from dotenv import load_dotenv

# Load environment variables from .env file
load_dotenv()

# Initialize extensions
db = SQLAlchemy()
jwt = JWTManager()

def create_app(test_config=None):
    """Create and configure the Flask application."""
    app = Flask(__name__, instance_relative_config=True)
    
    # Configure the app
    app.config.from_mapping(
        SECRET_KEY=os.environ.get('SECRET_KEY', 'dev-key-for-development-only'),
        SQLALCHEMY_DATABASE_URI=os.environ.get('DATABASE_URI', 'sqlite:///data/sdchat.db'),
        SQLALCHEMY_TRACK_MODIFICATIONS=False,
        JWT_SECRET_KEY=os.environ.get('JWT_SECRET_KEY', 'jwt-secret-key-dev-only'),
        JWT_ACCESS_TOKEN_EXPIRES=timedelta(hours=1),
        JWT_REFRESH_TOKEN_EXPIRES=timedelta(days=30),
        RULES_DIR=os.environ.get('RULES_DIR', os.path.join(app.instance_path, 'rules')),
        REPORTS_DIR=os.environ.get('REPORTS_DIR', os.path.join(app.instance_path, 'reports')),
        MAX_CONTENT_LENGTH=16 * 1024 * 1024,  # 16MB max upload size
    )
    
    # Override config with test config if provided
    if test_config:
        app.config.update(test_config)
    
    # Ensure the instance folder exists
    try:
        os.makedirs(app.instance_path, exist_ok=True)
        os.makedirs(app.config['RULES_DIR'], exist_ok=True)
        os.makedirs(app.config['REPORTS_DIR'], exist_ok=True)
    except OSError:
        pass
    
    # Initialize extensions with app
    db.init_app(app)
    jwt.init_app(app)
    
    # Setup CORS
    CORS(app, resources={r"/api/*": {"origins": "*"}})
    
    # Register blueprints
    from app.api.rules import rules_bp
    from app.api.reports import reports_bp
    from app.api.auth import auth_bp
    
    app.register_blueprint(rules_bp, url_prefix='/api/v1/rules')
    app.register_blueprint(reports_bp, url_prefix='/api/v1/reports')
    app.register_blueprint(auth_bp, url_prefix='/api/v1/auth')
    
    # Create a simple index route
    @app.route('/')
    def index():
        return {"message": "Welcome to SDChat Security Scanner API", 
                "version": "1.0.0"}
    
    # Create database tables
    with app.app_context():
        db.create_all()
    
    return app