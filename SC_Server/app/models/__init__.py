from datetime import datetime
from uuid import uuid4
from app import db
from sqlalchemy.dialects.postgresql import JSON
from sqlalchemy.ext.declarative import declared_attr
from werkzeug.security import generate_password_hash, check_password_hash
from flask_login import UserMixin

class BaseModel:
    """Base model with common fields for all models"""
    id = db.Column(db.Integer, primary_key=True)
    created_at = db.Column(db.DateTime, default=datetime.utcnow)
    updated_at = db.Column(db.DateTime, default=datetime.utcnow, onupdate=datetime.utcnow)

    @declared_attr
    def __tablename__(cls):
        return cls.__name__.lower()


class User(BaseModel, UserMixin, db.Model):
    """User model for authentication"""
    username = db.Column(db.String(80), unique=True, nullable=False)
    email = db.Column(db.String(120), unique=True, nullable=False)
    password_hash = db.Column(db.String(256), nullable=False)
    is_admin = db.Column(db.Boolean, default=False)
    is_active = db.Column(db.Boolean, default=True)

    def set_password(self, password):
        self.password_hash = generate_password_hash(password)

    def check_password(self, password):
        return check_password_hash(self.password_hash, password)

    # Flask-Login interface methods
    def get_id(self):
        return str(self.id)

    def is_authenticated(self):
        return True

    def is_active(self):
        return self.is_active

    def is_anonymous(self):
        return False

    def __repr__(self):
        return f'<User {self.username}>'


class RuleSet(BaseModel, db.Model):
    """Rule set model for security scanning rules"""
    name = db.Column(db.String(100), nullable=False)
    version = db.Column(db.String(20), nullable=False)
    description = db.Column(db.Text)
    is_active = db.Column(db.Boolean, default=True)
    file_types = db.relationship('FileType', backref='ruleset', lazy=True, cascade='all, delete-orphan')

    def __repr__(self):
        return f'<RuleSet {self.name} v{self.version}>'


class FileType(BaseModel, db.Model):
    """File type model for rule definitions"""
    name = db.Column(db.String(100), nullable=False)
    ruleset_id = db.Column(db.Integer, db.ForeignKey('ruleset.id'), nullable=False)
    identifiers = db.Column(JSON, nullable=False, default=list)  # List of identifiers as JSON
    patterns = db.relationship('Pattern', backref='filetype', lazy=True, cascade='all, delete-orphan')

    def __repr__(self):
        return f'<FileType {self.name}>'


class Pattern(BaseModel, db.Model):
    """Pattern model for vulnerability detection"""
    id_code = db.Column(db.String(20), nullable=False)  # e.g., CWE-434
    description = db.Column(db.Text, nullable=False)
    severity = db.Column(db.String(20), nullable=False)  # critical, high, medium, low
    regex = db.Column(db.Text, nullable=False)
    filetype_id = db.Column(db.Integer, db.ForeignKey('filetype.id'), nullable=False)

    def __repr__(self):
        return f'<Pattern {self.id_code} ({self.severity})>'


class Report(BaseModel, db.Model):
    """Report model for storing scan results"""
    report_id = db.Column(db.String(36), default=lambda: str(uuid4()), unique=True)
    title = db.Column(db.String(200), nullable=False)
    scan_target = db.Column(db.String(255), nullable=False)
    summary = db.Column(db.Text)
    results_json = db.Column(JSON, nullable=False)  # Store full scan results as JSON
    stats = db.Column(JSON)  # Store statistics as JSON
    uploaded_by = db.Column(db.Integer, db.ForeignKey('user.id'), nullable=True)
    user = db.relationship('User', backref='reports')

    # 新增字段
    application_name = db.Column(db.String(200))  # 归属应用
    uploader_name = db.Column(db.String(100))     # 上传人
    rule_version = db.Column(db.String(50))       # 扫描规则版本
    notes = db.Column(db.Text)                    # 备注信息

    def __repr__(self):
        return f'<Report {self.title} ({self.report_id})>'