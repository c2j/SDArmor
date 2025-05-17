#!/usr/bin/env python3
"""
Database initialization script for the SDChat Scanner Server.

This script:
1. Creates all database tables
2. Loads the default rule set
3. Creates an admin user
"""

import os
import sys
import json
import logging
from getpass import getpass
from flask import Flask
from dotenv import load_dotenv

# Set up logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s',
    handlers=[logging.StreamHandler(sys.stdout)]
)
logger = logging.getLogger(__name__)

# Load environment variables
load_dotenv()

# Import app modules
sys.path.append(os.path.dirname(os.path.abspath(__file__)))
from app import create_app, db
from app.models import User, RuleSet, FileType, Pattern

def init_db():
    """Initialize the database"""
    app = create_app()
    
    with app.app_context():
        # Create all tables
        logger.info("Creating database tables...")
        db.create_all()
        
        # Check if any users exist
        if User.query.count() > 0:
            logger.info("Database already has users. Skipping admin user creation.")
        else:
            # Create admin user
            logger.info("Creating admin user...")
            create_admin_user()
            
        # Check if any rule sets exist
        if RuleSet.query.count() > 0:
            logger.info("Database already has rule sets. Skipping default rule set loading.")
        else:
            # Load default rules
            logger.info("Loading default rule set...")
            load_default_rules()
            
        logger.info("Database initialization complete!")

def create_admin_user():
    """Create an admin user"""
    # Get admin credentials
    username = input("Admin username [admin]: ") or "admin"
    email = input("Admin email [admin@example.com]: ") or "admin@example.com"
    password = getpass("Admin password [default-secure-password]: ") or "default-secure-password"
    
    # Create user
    admin = User(
        username=username,
        email=email,
        is_admin=True
    )
    admin.set_password(password)
    
    try:
        db.session.add(admin)
        db.session.commit()
        logger.info(f"Admin user '{username}' created successfully!")
    except Exception as e:
        db.session.rollback()
        logger.error(f"Error creating admin user: {str(e)}")
        sys.exit(1)

def load_default_rules():
    """Load default rule set from JSON file"""
    # Path to default rules
    rules_file = os.path.join(os.path.dirname(os.path.abspath(__file__)), 
                             "data/rules/default_rules.json")
    
    try:
        # Load rule set from file
        with open(rules_file, 'r') as f:
            rule_data = json.load(f)
            
        # Create rule set
        rule_set = RuleSet(
            name="Default Security Rules",
            version=rule_data.get("version", "1.0"),
            description="Default security vulnerability detection rules",
            is_active=True
        )
        
        # Add file types
        for ft_data in rule_data.get("file_types", []):
            file_type = FileType(
                name=ft_data["name"],
                identifiers=ft_data["identifiers"]
            )
            
            # Add patterns
            for pattern_data in ft_data.get("patterns", []):
                pattern = Pattern(
                    id_code=pattern_data["id"],
                    description=pattern_data["description"],
                    severity=pattern_data["severity"],
                    regex=pattern_data["regex"]
                )
                file_type.patterns.append(pattern)
            
            rule_set.file_types.append(file_type)
        
        # Save to database
        db.session.add(rule_set)
        db.session.commit()
        logger.info(f"Loaded default rule set with {len(rule_set.file_types)} file types")
        
    except FileNotFoundError:
        logger.error(f"Default rules file not found: {rules_file}")
        sys.exit(1)
    except json.JSONDecodeError as e:
        logger.error(f"Error parsing JSON: {str(e)}")
        sys.exit(1)
    except Exception as e:
        db.session.rollback()
        logger.error(f"Error loading default rules: {str(e)}")
        sys.exit(1)

if __name__ == "__main__":
    init_db()