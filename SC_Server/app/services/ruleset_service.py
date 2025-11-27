"""
Rule set service for handling rule set management operations.

This service provides a higher-level abstraction over the RuleSet, FileType,
and Pattern models for rule set management operations.
"""

from typing import Optional, List, Dict, Any
from datetime import datetime
from app import db
from app.models import RuleSet, FileType, Pattern
import logging
import json

logger = logging.getLogger(__name__)


class RuleSetService:
    """Service class for rule set management operations."""

    @staticmethod
    def get_all_rulesets(active_only: bool = False) -> List[RuleSet]:
        """
        Get all rule sets.

        Args:
            active_only: If True, return only active rule sets

        Returns:
            List of RuleSet objects
        """
        try:
            query = RuleSet.query
            if active_only:
                query = query.filter_by(is_active=True)

            rulesets = query.all()
            logger.info(f"Retrieved {len(rulesets)} rule sets")
            return rulesets

        except Exception as e:
            logger.error(f"Error retrieving rule sets: {str(e)}")
            return []

    @staticmethod
    def get_ruleset_by_id(ruleset_id: int) -> Optional[RuleSet]:
        """
        Get rule set by ID.

        Args:
            ruleset_id: The rule set ID

        Returns:
            RuleSet object if found, None otherwise
        """
        try:
            ruleset = RuleSet.query.get(ruleset_id)
            if ruleset:
                logger.info(f"Retrieved rule set {ruleset.name} (ID: {ruleset_id})")
            else:
                logger.warning(f"Rule set with ID {ruleset_id} not found")
            return ruleset

        except Exception as e:
            logger.error(f"Error retrieving rule set {ruleset_id}: {str(e)}")
            return None

    @staticmethod
    def create_ruleset(name: str, version: str, description: str = "",
                      file_types_data: List[Dict] = None,
                      is_active: bool = False) -> Optional[RuleSet]:
        """
        Create a new rule set.

        Args:
            name: The rule set name
            version: The version string (SemVer format)
            description: The rule set description
            file_types_data: List of file types with patterns
            is_active: Whether this rule set should be active

        Returns:
            RuleSet object if created successfully, None otherwise
        """
        try:
            # Check if version already exists
            if RuleSet.query.filter_by(version=version).first():
                logger.warning(f"Rule set version {version} already exists")
                return None

            # Deactivate all other rule sets if this one should be active
            if is_active:
                RuleSet.query.update({'is_active': False})

            # Create new rule set
            ruleset = RuleSet(
                name=name,
                version=version,
                description=description,
                is_active=is_active
            )

            # Add file types and patterns if provided
            if file_types_data:
                for ft_data in file_types_data:
                    file_type = FileType(
                        name=ft_data.get('name', ''),
                        identifiers=ft_data.get('identifiers', [])
                    )

                    # Add patterns
                    patterns_data = ft_data.get('patterns', [])
                    for pattern_data in patterns_data:
                        pattern = Pattern(
                            id_code=pattern_data.get('id', ''),
                            description=pattern_data.get('description', ''),
                            severity=pattern_data.get('severity', 'medium'),
                            regex=pattern_data.get('regex', '')
                        )
                        file_type.patterns.append(pattern)

                    ruleset.file_types.append(file_type)

            db.session.add(ruleset)
            db.session.commit()

            logger.info(f"Created rule set {name} v{version}")
            return ruleset

        except Exception as e:
            logger.error(f"Error creating rule set {name}: {str(e)}")
            db.session.rollback()
            return None

    @staticmethod
    def update_ruleset(ruleset_id: int, name: str = None,
                      description: str = None, is_active: bool = None) -> Optional[RuleSet]:
        """
        Update an existing rule set.

        Args:
            ruleset_id: The rule set ID
            name: New name (optional)
            description: New description (optional)
            is_active: New active status (optional)

        Returns:
            RuleSet object if updated successfully, None otherwise
        """
        try:
            ruleset = RuleSet.query.get(ruleset_id)
            if not ruleset:
                logger.warning(f"Rule set with ID {ruleset_id} not found for update")
                return None

            # Update fields if provided
            if name is not None:
                ruleset.name = name
            if description is not None:
                ruleset.description = description

            # Update active status if provided
            if is_active is not None:
                # Deactivate all other rule sets if this one should be active
                if is_active:
                    RuleSet.query.update({'is_active': False})
                ruleset.is_active = is_active

            db.session.commit()

            logger.info(f"Updated rule set {ruleset.name} (ID: {ruleset_id})")
            return ruleset

        except Exception as e:
            logger.error(f"Error updating rule set {ruleset_id}: {str(e)}")
            db.session.rollback()
            return None

    @staticmethod
    def delete_ruleset(ruleset_id: int) -> bool:
        """
        Delete a rule set.

        Args:
            ruleset_id: The rule set ID

        Returns:
            True if deleted successfully, False otherwise
        """
        try:
            ruleset = RuleSet.query.get(ruleset_id)
            if not ruleset:
                logger.warning(f"Rule set with ID {ruleset_id} not found for deletion")
                return False

            # Cannot delete active rule set
            if ruleset.is_active:
                logger.warning(f"Cannot delete active rule set {ruleset.name}")
                return False

            ruleset_name = ruleset.name
            db.session.delete(ruleset)
            db.session.commit()

            logger.info(f"Deleted rule set {ruleset_name} (ID: {ruleset_id})")
            return True

        except Exception as e:
            logger.error(f"Error deleting rule set {ruleset_id}: {str(e)}")
            db.session.rollback()
            return False

    @staticmethod
    def activate_ruleset(ruleset_id: int) -> Optional[RuleSet]:
        """
        Activate a rule set and deactivate all others.

        Args:
            ruleset_id: The rule set ID

        Returns:
            RuleSet object if activated successfully, None otherwise
        """
        try:
            ruleset = RuleSet.query.get(ruleset_id)
            if not ruleset:
                logger.warning(f"Rule set with ID {ruleset_id} not found for activation")
                return None

            # Deactivate all rule sets
            RuleSet.query.update({'is_active': False})

            # Activate the specified rule set
            ruleset.is_active = True
            db.session.commit()

            logger.info(f"Activated rule set {ruleset.name} (ID: {ruleset_id})")
            return ruleset

        except Exception as e:
            logger.error(f"Error activating rule set {ruleset_id}: {str(e)}")
            db.session.rollback()
            return None

    @staticmethod
    def get_active_ruleset() -> Optional[RuleSet]:
        """
        Get the currently active rule set.

        Returns:
            RuleSet object if active rule set exists, None otherwise
        """
        try:
            ruleset = RuleSet.query.filter_by(is_active=True).first()
            if ruleset:
                logger.info(f"Retrieved active rule set {ruleset.name}")
            else:
                logger.info("No active rule set found")
            return ruleset

        except Exception as e:
            logger.error(f"Error retrieving active rule set: {str(e)}")
            return None

    @staticmethod
    def get_ruleset_summary(ruleset_id: int) -> Optional[Dict[str, Any]]:
        """
        Get rule set summary information.

        Args:
            ruleset_id: The rule set ID

        Returns:
            Dictionary with summary information or None if not found
        """
        try:
            ruleset = RuleSet.query.get(ruleset_id)
            if not ruleset:
                return None

            # Calculate statistics
            severity_counts = {"critical": 0, "high": 0, "medium": 0, "low": 0}
            total_patterns = 0

            for file_type in ruleset.file_types:
                for pattern in file_type.patterns:
                    if pattern.severity in severity_counts:
                        severity_counts[pattern.severity] += 1
                    total_patterns += 1

            summary = {
                "id": ruleset.id,
                "name": ruleset.name,
                "version": ruleset.version,
                "description": ruleset.description,
                "is_active": ruleset.is_active,
                "created_at": ruleset.created_at.isoformat() if ruleset.created_at else None,
                "updated_at": ruleset.updated_at.isoformat() if ruleset.updated_at else None,
                "file_types_count": len(ruleset.file_types),
                "total_patterns": total_patterns,
                "severity_distribution": severity_counts
            }

            return summary

        except Exception as e:
            logger.error(f"Error getting rule set summary {ruleset_id}: {str(e)}")
            return None

    @staticmethod
    def export_ruleset(ruleset_id: int) -> Optional[Dict[str, Any]]:
        """
        Export rule set in JSON format.

        Args:
            ruleset_id: The rule set ID

        Returns:
            Dictionary with rule set data or None if not found
        """
        try:
            ruleset = RuleSet.query.get(ruleset_id)
            if not ruleset:
                return None

            # Convert to export format
            export_data = {
                "name": ruleset.name,
                "version": ruleset.version,
                "description": ruleset.description or "",
                "file_types": []
            }

            for file_type in ruleset.file_types:
                ft_data = {
                    "name": file_type.name,
                    "identifiers": file_type.identifiers or [],
                    "patterns": []
                }

                for pattern in file_type.patterns:
                    pattern_data = {
                        "id": pattern.id_code,
                        "description": pattern.description,
                        "severity": pattern.severity,
                        "regex": pattern.regex
                    }
                    ft_data["patterns"].append(pattern_data)

                export_data["file_types"].append(ft_data)

            logger.info(f"Exported rule set {ruleset.name} (ID: {ruleset_id})")
            return export_data

        except Exception as e:
            logger.error(f"Error exporting rule set {ruleset_id}: {str(e)}")
            return None

    @staticmethod
    def import_ruleset(import_data: Dict[str, Any], activate: bool = False) -> Optional[RuleSet]:
        """
        Import rule set from JSON data.

        Args:
            import_data: Dictionary with rule set data
            activate: Whether to activate the imported rule set

        Returns:
            RuleSet object if imported successfully, None otherwise
        """
        try:
            # Validate required fields
            if not all(key in import_data for key in ['name', 'version', 'file_types']):
                logger.error("Invalid rule set data: missing required fields")
                return None

            # Validate file types structure
            for ft_data in import_data['file_types']:
                if not all(key in ft_data for key in ['name', 'identifiers', 'patterns']):
                    logger.error("Invalid file type data: missing required fields")
                    return None

                # Validate patterns structure
                for pattern_data in ft_data['patterns']:
                    if not all(key in pattern_data for key in ['id', 'description', 'severity', 'regex']):
                        logger.error("Invalid pattern data: missing required fields")
                        return None

                    # Validate severity
                    if pattern_data['severity'] not in ['critical', 'high', 'medium', 'low']:
                        logger.error(f"Invalid severity: {pattern_data['severity']}")
                        return None

            # Create rule set
            ruleset = RuleSetService.create_ruleset(
                name=import_data['name'],
                version=import_data['version'],
                description=import_data.get('description', ''),
                file_types_data=import_data['file_types'],
                is_active=activate
            )

            if ruleset:
                logger.info(f"Imported rule set {ruleset.name} v{ruleset.version}")
            return ruleset

        except Exception as e:
            logger.error(f"Error importing rule set: {str(e)}")
            return None

    @staticmethod
    def validate_ruleset_data(data: Dict[str, Any]) -> Dict[str, str]:
        """
        Validate rule set data for creation/update.

        Args:
            data: Dictionary with rule set data

        Returns:
            Dictionary of validation errors (empty if valid)
        """
        errors = {}

        # Name validation
        if not data.get('name'):
            errors['name'] = 'Name is required'
        elif len(data.get('name', '')) < 1:
            errors['name'] = 'Name must be at least 1 character long'
        elif len(data.get('name', '')) > 100:
            errors['name'] = 'Name must be less than 100 characters long'

        # Version validation
        if not data.get('version'):
            errors['version'] = 'Version is required'
        elif len(data.get('version', '')) < 1:
            errors['version'] = 'Version must be at least 1 character long'
        elif len(data.get('version', '')) > 20:
            errors['version'] = 'Version must be less than 20 characters long'

        # Description validation
        if 'description' in data and len(str(data.get('description', ''))) > 1000:
            errors['description'] = 'Description must be less than 1000 characters long'

        return errors