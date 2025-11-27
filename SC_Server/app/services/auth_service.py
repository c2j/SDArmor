"""
Authentication service for handling user authentication operations.

This service provides a higher-level abstraction over the User model
for authentication-related operations.
"""

from typing import Optional, Dict, Any
from datetime import datetime, timedelta
from app import db
from app.models import User
from flask_jwt_extended import create_access_token, create_refresh_token
from werkzeug.security import check_password_hash
import logging

logger = logging.getLogger(__name__)


class AuthService:
    """Service class for authentication operations."""

    @staticmethod
    def authenticate_user(username: str, password: str) -> Optional[User]:
        """
        Authenticate a user with username and password.

        Args:
            username: The username to authenticate
            password: The password to verify

        Returns:
            User object if authentication successful, None otherwise
        """
        try:
            user = User.query.filter_by(username=username).first()

            if user and user.check_password(password) and user.is_active:
                logger.info(f"User {username} authenticated successfully")
                return user
            else:
                logger.warning(f"Failed authentication attempt for user {username}")
                return None

        except Exception as e:
            logger.error(f"Error during authentication for user {username}: {str(e)}")
            return None

    @staticmethod
    def create_tokens(user: User) -> Dict[str, Any]:
        """
        Create JWT tokens for an authenticated user.

        Args:
            user: The authenticated user object

        Returns:
            Dictionary containing access_token, refresh_token, and user info
        """
        try:
            # Create access token with user identity and additional claims
            additional_claims = {
                'is_admin': user.is_admin,
                'username': user.username
            }

            access_token = create_access_token(
                identity=user.id,
                additional_claims=additional_claims
            )

            refresh_token = create_refresh_token(identity=user.id)

            user_info = {
                'id': user.id,
                'username': user.username,
                'email': user.email,
                'is_admin': user.is_admin,
                'created_at': user.created_at.isoformat() if user.created_at else None
            }

            logger.info(f"Tokens created successfully for user {user.username}")

            return {
                'access_token': access_token,
                'refresh_token': refresh_token,
                'user': user_info
            }

        except Exception as e:
            logger.error(f"Error creating tokens for user {user.username}: {str(e)}")
            raise

    @staticmethod
    def register_user(username: str, email: str, password: str, is_admin: bool = False) -> Optional[User]:
        """
        Register a new user.

        Args:
            username: The username for the new user
            email: The email address for the new user
            password: The password for the new user
            is_admin: Whether the user should be an admin (default False)

        Returns:
            User object if registration successful, None otherwise
        """
        try:
            # Check if username or email already exists
            if User.query.filter_by(username=username).first():
                logger.warning(f"Registration failed: username {username} already exists")
                return None

            if User.query.filter_by(email=email).first():
                logger.warning(f"Registration failed: email {email} already exists")
                return None

            # Create new user
            new_user = User(
                username=username,
                email=email,
                is_admin=is_admin,
                is_active=True
            )
            new_user.set_password(password)

            db.session.add(new_user)
            db.session.commit()

            logger.info(f"New user {username} registered successfully")
            return new_user

        except Exception as e:
            logger.error(f"Error during user registration for {username}: {str(e)}")
            db.session.rollback()
            return None

    @staticmethod
    def change_password(user: User, current_password: str, new_password: str) -> bool:
        """
        Change user password.

        Args:
            user: The user object
            current_password: The current password to verify
            new_password: The new password to set

        Returns:
            True if password changed successfully, False otherwise
        """
        try:
            # Verify current password
            if not user.check_password(current_password):
                logger.warning(f"Password change failed for user {user.username}: invalid current password")
                return False

            # Set new password
            user.set_password(new_password)
            db.session.commit()

            logger.info(f"Password changed successfully for user {user.username}")
            return True

        except Exception as e:
            logger.error(f"Error changing password for user {user.username}: {str(e)}")
            db.session.rollback()
            return False

    @staticmethod
    def get_user_by_id(user_id: int) -> Optional[User]:
        """
        Get user by ID.

        Args:
            user_id: The user ID

        Returns:
            User object if found, None otherwise
        """
        try:
            return User.query.get(user_id)
        except Exception as e:
            logger.error(f"Error getting user by ID {user_id}: {str(e)}")
            return None

    @staticmethod
    def update_user_status(user_id: int, is_active: bool) -> bool:
        """
        Update user active status.

        Args:
            user_id: The user ID
            is_active: The new active status

        Returns:
            True if updated successfully, False otherwise
        """
        try:
            user = User.query.get(user_id)
            if not user:
                logger.warning(f"User with ID {user_id} not found")
                return False

            user.is_active = is_active
            db.session.commit()

            logger.info(f"User {user.username} status updated to {'active' if is_active else 'inactive'}")
            return True

        except Exception as e:
            logger.error(f"Error updating user status for ID {user_id}: {str(e)}")
            db.session.rollback()
            return False

    @staticmethod
    def validate_user_data(username: str, email: str, password: str) -> Dict[str, str]:
        """
        Validate user registration data.

        Args:
            username: The username to validate
            email: The email to validate
            password: The password to validate

        Returns:
            Dictionary of validation errors (empty if valid)
        """
        errors = {}

        # Username validation
        if not username:
            errors['username'] = 'Username is required'
        elif len(username) < 3:
            errors['username'] = 'Username must be at least 3 characters long'
        elif len(username) > 80:
            errors['username'] = 'Username must be less than 80 characters long'

        # Email validation
        if not email:
            errors['email'] = 'Email is required'
        elif '@' not in email or '.' not in email:
            errors['email'] = 'Invalid email format'
        elif len(email) > 120:
            errors['email'] = 'Email must be less than 120 characters long'

        # Password validation
        if not password:
            errors['password'] = 'Password is required'
        elif len(password) < 8:
            errors['password'] = 'Password must be at least 8 characters long'
        elif not any(c.isupper() for c in password):
            errors['password'] = 'Password must contain at least one uppercase letter'
        elif not any(c.islower() for c in password):
            errors['password'] = 'Password must contain at least one lowercase letter'
        elif not any(c.isdigit() for c in password):
            errors['password'] = 'Password must contain at least one digit'

        return errors