# Utility functions and helpers for the SDChat Security Scanner Server

import os
import json
import uuid
import logging
from datetime import datetime
from functools import wraps

# Setup logging
logger = logging.getLogger(__name__)

def get_timestamp():
    """Get current timestamp in ISO format"""
    return datetime.utcnow().isoformat()

def generate_unique_id():
    """Generate a UUID"""
    return str(uuid.uuid4())

def load_json_file(file_path):
    """Load and parse a JSON file"""
    try:
        if not os.path.exists(file_path):
            return None
        with open(file_path, 'r') as f:
            return json.load(f)
    except Exception as e:
        logger.error(f"Error loading JSON file {file_path}: {str(e)}")
        return None

def save_json_file(data, file_path):
    """Save data to a JSON file"""
    try:
        with open(file_path, 'w') as f:
            json.dump(data, f, indent=2)
        return True
    except Exception as e:
        logger.error(f"Error saving JSON file {file_path}: {str(e)}")
        return False

def ensure_directory(directory_path):
    """Ensure a directory exists, creating it if necessary"""
    if not os.path.exists(directory_path):
        os.makedirs(directory_path, exist_ok=True)
    return directory_path