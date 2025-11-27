"""
Report service for handling report management operations.

This service provides a higher-level abstraction over the Report model
for report management operations.
"""

from typing import Optional, List, Dict, Any
from datetime import datetime, timedelta
from uuid import uuid4
from app import db
from app.models import Report, User
from sqlalchemy import and_, or_, func
import logging
import json

logger = logging.getLogger(__name__)


class ReportService:
    """Service class for report management operations."""

    @staticmethod
    def create_report(title: str, scan_target: str, summary: str = "",
                    results_json: Dict = None, stats: Dict = None,
                    uploaded_by: int = None, application_name: str = "",
                    uploader_name: str = "", rule_version: str = "",
                    notes: str = "") -> Optional[Report]:
        """
        Create a new report.

        Args:
            title: The report title
            scan_target: The path that was scanned
            summary: The report summary
            results_json: The scan results as JSON
            stats: Statistics about the scan
            uploaded_by: User ID who uploaded the report
            application_name: Name of the application scanned
            uploader_name: Name of the uploader
            rule_version: Version of rules used
            notes: Additional notes

        Returns:
            Report object if created successfully, None otherwise
        """
        try:
            # Generate unique report ID
            report_id = str(uuid4())

            # Create report
            report = Report(
                report_id=report_id,
                title=title,
                scan_target=scan_target,
                summary=summary,
                results_json=results_json or {},
                stats=stats or {},
                uploaded_by=uploaded_by,
                application_name=application_name,
                uploader_name=uploader_name,
                rule_version=rule_version,
                notes=notes
            )

            db.session.add(report)
            db.session.commit()

            logger.info(f"Created report {title} (ID: {report_id})")
            return report

        except Exception as e:
            logger.error(f"Error creating report {title}: {str(e)}")
            db.session.rollback()
            return None

    @staticmethod
    def get_report_by_id(report_id: str) -> Optional[Report]:
        """
        Get report by report ID.

        Args:
            report_id: The UUID of the report

        Returns:
            Report object if found, None otherwise
        """
        try:
            report = Report.query.filter_by(report_id=report_id).first()
            if report:
                logger.info(f"Retrieved report {report.title} (ID: {report_id})")
            else:
                logger.warning(f"Report with ID {report_id} not found")
            return report

        except Exception as e:
            logger.error(f"Error retrieving report {report_id}: {str(e)}")
            return None

    @staticmethod
    def get_report_by_pk(report_pk: int) -> Optional[Report]:
        """
        Get report by primary key.

        Args:
            report_pk: The primary key of the report

        Returns:
            Report object if found, None otherwise
        """
        try:
            report = Report.query.get(report_pk)
            if report:
                logger.info(f"Retrieved report {report.title} (PK: {report_pk})")
            else:
                logger.warning(f"Report with PK {report_pk} not found")
            return report

        except Exception as e:
            logger.error(f"Error retrieving report {report_pk}: {str(e)}")
            return None

    @staticmethod
    def get_user_reports(user_id: int, page: int = 1, per_page: int = 10,
                      start_date: str = None, end_date: str = None,
                      search_term: str = None) -> Dict[str, Any]:
        """
        Get reports for a specific user with pagination.

        Args:
            user_id: The user ID
            page: Page number (default 1)
            per_page: Items per page (default 10, max 50)
            start_date: Filter by start date (YYYY-MM-DD format)
            end_date: Filter by end date (YYYY-MM-DD format)
            search_term: Search in title and scan_target

        Returns:
            Dictionary with pagination info and reports
        """
        try:
            # Limit per_page to 50
            per_page = min(per_page, 50)

            # Build query
            query = Report.query.filter_by(uploaded_by=user_id)

            # Apply date filters
            if start_date:
                try:
                    start_dt = datetime.strptime(start_date, '%Y-%m-%d')
                    query = query.filter(Report.created_at >= start_dt)
                except ValueError:
                    logger.warning(f"Invalid start_date format: {start_date}")

            if end_date:
                try:
                    end_dt = datetime.strptime(end_date, '%Y-%m-%d')
                    # Set to end of day
                    end_dt = end_dt.replace(hour=23, minute=59, second=59)
                    query = query.filter(Report.created_at <= end_dt)
                except ValueError:
                    logger.warning(f"Invalid end_date format: {end_date}")

            # Apply search filter
            if search_term:
                search_pattern = f"%{search_term}%"
                query = query.filter(
                    or_(
                        Report.title.ilike(search_pattern),
                        Report.scan_target.ilike(search_pattern)
                    )
                )

            # Order by creation date (newest first)
            query = query.order_by(Report.created_at.desc())

            # Paginate
            pagination = query.paginate(
                page=page, per_page=per_page, error_out=False
            )

            reports = pagination.items

            # Convert to dict format
            reports_data = []
            for report in reports:
                report_data = {
                    "id": report.id,
                    "report_id": report.report_id,
                    "title": report.title,
                    "scan_target": report.scan_target,
                    "created_at": report.created_at.isoformat() if report.created_at else None,
                    "updated_at": report.updated_at.isoformat() if report.updated_at else None,
                    "critical_count": report.stats.get('critical', 0) if report.stats else 0,
                    "high_count": report.stats.get('high', 0) if report.stats else 0,
                    "medium_count": report.stats.get('medium', 0) if report.stats else 0,
                    "low_count": report.stats.get('low', 0) if report.stats else 0,
                }
                reports_data.append(report_data)

            logger.info(f"Retrieved {len(reports)} reports for user ID {user_id}")

            return {
                "reports": reports_data,
                "total": pagination.total,
                "pages": pagination.pages,
                "current_page": page,
                "per_page": per_page,
                "has_next": pagination.has_next,
                "has_prev": pagination.has_prev
            }

        except Exception as e:
            logger.error(f"Error getting user reports {user_id}: {str(e)}")
            return {
                "reports": [],
                "total": 0,
                "pages": 0,
                "current_page": page,
                "per_page": per_page,
                "has_next": False,
                "has_prev": False
            }

    @staticmethod
    def get_all_reports(page: int = 1, per_page: int = 10,
                       start_date: str = None, end_date: str = None,
                       search_term: str = None) -> Dict[str, Any]:
        """
        Get all reports with pagination (admin function).

        Args:
            page: Page number (default 1)
            per_page: Items per page (default 10, max 50)
            start_date: Filter by start date (YYYY-MM-DD format)
            end_date: Filter by end date (YYYY-MM-DD format)
            search_term: Search in title and scan_target

        Returns:
            Dictionary with pagination info and reports
        """
        try:
            # Limit per_page to 50
            per_page = min(per_page, 50)

            # Build query
            query = Report.query

            # Apply date filters
            if start_date:
                try:
                    start_dt = datetime.strptime(start_date, '%Y-%m-%d')
                    query = query.filter(Report.created_at >= start_dt)
                except ValueError:
                    logger.warning(f"Invalid start_date format: {start_date}")

            if end_date:
                try:
                    end_dt = datetime.strptime(end_date, '%Y-%m-%d')
                    end_dt = end_dt.replace(hour=23, minute=59, second=59)
                    query = query.filter(Report.created_at <= end_dt)
                except ValueError:
                    logger.warning(f"Invalid end_date format: {end_date}")

            # Apply search filter
            if search_term:
                search_pattern = f"%{search_term}%"
                query = query.filter(
                    or_(
                        Report.title.ilike(search_pattern),
                        Report.scan_target.ilike(search_pattern)
                    )
                )

            # Order by creation date (newest first)
            query = query.order_by(Report.created_at.desc())

            # Paginate
            pagination = query.paginate(
                page=page, per_page=per_page, error_out=False
            )

            reports = pagination.items

            # Convert to dict format
            reports_data = []
            for report in reports:
                report_data = {
                    "id": report.id,
                    "report_id": report.report_id,
                    "title": report.title,
                    "scan_target": report.scan_target,
                    "created_at": report.created_at.isoformat() if report.created_at else None,
                    "updated_at": report.updated_at.isoformat() if report.updated_at else None,
                    "critical_count": report.stats.get('critical', 0) if report.stats else 0,
                    "high_count": report.stats.get('high', 0) if report.stats else 0,
                    "medium_count": report.stats.get('medium', 0) if report.stats else 0,
                    "low_count": report.stats.get('low', 0) if report.stats else 0,
                    "uploader_name": report.uploader_name,
                    "application_name": report.application_name,
                }
                reports_data.append(report_data)

            logger.info(f"Retrieved {len(reports)} reports (admin)")

            return {
                "reports": reports_data,
                "total": pagination.total,
                "pages": pagination.pages,
                "current_page": page,
                "per_page": per_page,
                "has_next": pagination.has_next,
                "has_prev": pagination.has_prev
            }

        except Exception as e:
            logger.error(f"Error getting all reports: {str(e)}")
            return {
                "reports": [],
                "total": 0,
                "pages": 0,
                "current_page": page,
                "per_page": per_page,
                "has_next": False,
                "has_prev": False
            }

    @staticmethod
    def update_report(report_id: str, title: str = None, summary: str = None,
                      notes: str = None) -> Optional[Report]:
        """
        Update an existing report.

        Args:
            report_id: The UUID of the report
            title: New title (optional)
            summary: New summary (optional)
            notes: New notes (optional)

        Returns:
            Report object if updated successfully, None otherwise
        """
        try:
            report = Report.query.filter_by(report_id=report_id).first()
            if not report:
                logger.warning(f"Report with ID {report_id} not found for update")
                return None

            # Update fields if provided
            if title is not None:
                report.title = title
            if summary is not None:
                report.summary = summary
            if notes is not None:
                report.notes = notes

            db.session.commit()

            logger.info(f"Updated report {report.title} (ID: {report_id})")
            return report

        except Exception as e:
            logger.error(f"Error updating report {report_id}: {str(e)}")
            db.session.rollback()
            return None

    @staticmethod
    def delete_report(report_id: str) -> bool:
        """
        Delete a report.

        Args:
            report_id: The UUID of the report

        Returns:
            True if deleted successfully, False otherwise
        """
        try:
            report = Report.query.filter_by(report_id=report_id).first()
            if not report:
                logger.warning(f"Report with ID {report_id} not found for deletion")
                return False

            report_title = report.title
            db.session.delete(report)
            db.session.commit()

            logger.info(f"Deleted report {report_title} (ID: {report_id})")
            return True

        except Exception as e:
            logger.error(f"Error deleting report {report_id}: {str(e)}")
            db.session.rollback()
            return False

    @staticmethod
    def get_report_statistics(user_id: int = None) -> Dict[str, Any]:
        """
        Get report statistics.

        Args:
            user_id: If provided, get stats for specific user only

        Returns:
            Dictionary with statistics
        """
        try:
            query = Report.query
            if user_id:
                query = query.filter_by(uploaded_by=user_id)

            total_reports = query.count()

            # Get severity totals
            severity_stats = {"critical": 0, "high": 0, "medium": 0, "low": 0}
            for report in query.all():
                if report.stats:
                    for severity in severity_stats:
                        severity_stats[severity] += report.stats.get(severity, 0)

            # Get reports in last 30 days
            thirty_days_ago = datetime.utcnow() - timedelta(days=30)
            recent_reports = query.filter(Report.created_at >= thirty_days_ago).count()

            # Get top applications
            application_stats = {}
            for report in query.all():
                if report.application_name:
                    app_name = report.application_name
                    application_stats[app_name] = application_stats.get(app_name, 0) + 1

            # Sort applications by count and take top 5
            top_applications = sorted(
                application_stats.items(),
                key=lambda x: x[1],
                reverse=True
            )[:5]

            stats = {
                "total_reports": total_reports,
                "severity_distribution": severity_stats,
                "recent_reports_30_days": recent_reports,
                "top_applications": [
                    {"name": app, "count": count} for app, count in top_applications
                ]
            }

            logger.info(f"Generated report statistics for user {user_id if user_id else 'all'}")
            return stats

        except Exception as e:
            logger.error(f"Error getting report statistics: {str(e)}")
            return {
                "total_reports": 0,
                "severity_distribution": {"critical": 0, "high": 0, "medium": 0, "low": 0},
                "recent_reports_30_days": 0,
                "top_applications": []
            }

    @staticmethod
    def validate_report_data(data: Dict[str, Any]) -> Dict[str, str]:
        """
        Validate report data for creation/update.

        Args:
            data: Dictionary with report data

        Returns:
            Dictionary of validation errors (empty if valid)
        """
        errors = {}

        # Title validation
        if not data.get('title'):
            errors['title'] = 'Title is required'
        elif len(data.get('title', '')) < 1:
            errors['title'] = 'Title must be at least 1 character long'
        elif len(data.get('title', '')) > 200:
            errors['title'] = 'Title must be less than 200 characters long'

        # Scan target validation
        if not data.get('scan_target'):
            errors['scan_target'] = 'Scan target is required'
        elif len(data.get('scan_target', '')) < 1:
            errors['scan_target'] = 'Scan target must be at least 1 character long'

        # Summary validation
        if 'summary' in data and len(str(data.get('summary', ''))) > 10000:
            errors['summary'] = 'Summary must be less than 10000 characters long'

        # Notes validation
        if 'notes' in data and len(str(data.get('notes', ''))) > 5000:
            errors['notes'] = 'Notes must be less than 5000 characters long'

        # Results JSON validation
        if 'results_json' in data:
            try:
                json.dumps(data['results_json'])
            except (TypeError, ValueError):
                errors['results_json'] = 'Results JSON must be valid JSON'

        # Stats validation
        if 'stats' in data:
            try:
                json.dumps(data['stats'])
            except (TypeError, ValueError):
                errors['stats'] = 'Stats must be valid JSON'

            # Check if stats has expected structure
            stats = data.get('stats', {})
            valid_severities = ['critical', 'high', 'medium', 'low']
            for severity in stats:
                if severity not in valid_severities:
                    errors['stats'] = f'Invalid severity in stats: {severity}'

        return errors

    @staticmethod
    def export_report(report_id: str, format_type: str = 'json') -> Optional[str]:
        """
        Export a report in different formats.

        Args:
            report_id: The UUID of the report
            format_type: Export format ('json', 'html')

        Returns:
            String with exported data or None if not found
        """
        try:
            report = Report.query.filter_by(report_id=report_id).first()
            if not report:
                logger.warning(f"Report with ID {report_id} not found for export")
                return None

            if format_type.lower() == 'json':
                return ReportService._export_json(report)
            elif format_type.lower() == 'html':
                return ReportService._export_html(report)
            else:
                logger.warning(f"Unsupported export format: {format_type}")
                return None

        except Exception as e:
            logger.error(f"Error exporting report {report_id}: {str(e)}")
            return None

    @staticmethod
    def _export_json(report: Report) -> str:
        """Export report as JSON."""
        export_data = {
            "report_id": report.report_id,
            "title": report.title,
            "scan_target": report.scan_target,
            "summary": report.summary,
            "results": report.results_json,
            "stats": report.stats,
            "application_name": report.application_name,
            "uploader_name": report.uploader_name,
            "rule_version": report.rule_version,
            "notes": report.notes,
            "created_at": report.created_at.isoformat() if report.created_at else None,
            "updated_at": report.updated_at.isoformat() if report.updated_at else None,
        }

        return json.dumps(export_data, indent=2, ensure_ascii=False)

    @staticmethod
    def _export_html(report: Report) -> str:
        """Export report as HTML."""
        html_template = f"""
<!DOCTYPE html>
<html>
<head>
    <title>{report.title} - SDChat Security Report</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 40px; }}
        .header {{ background-color: #f5f5f5; padding: 20px; border-radius: 5px; }}
        .stats {{ margin: 20px 0; }}
        .stat-box {{ display: inline-block; margin: 10px; padding: 15px; border-radius: 5px; text-align: center; }}
        .critical {{ background-color: #d32f2f; color: white; }}
        .high {{ background-color: #f57c00; color: white; }}
        .medium {{ background-color: #fbc02d; color: black; }}
        .low {{ background-color: #388e3c; color: white; }}
        .results {{ margin: 20px 0; }}
        .result-item {{ margin: 10px 0; padding: 10px; border-left: 4px solid #ccc; }}
    </style>
</head>
<body>
    <div class="header">
        <h1>{report.title}</h1>
        <p><strong>Scan Target:</strong> {report.scan_target}</p>
        <p><strong>Generated:</strong> {report.created_at.strftime('%Y-%m-%d %H:%M:%S') if report.created_at else 'N/A'}</p>
        {f'<p><strong>Application:</strong> {report.application_name}</p>' if report.application_name else ''}
        {f'<p><strong>Rule Version:</strong> {report.rule_version}</p>' if report.rule_version else ''}
        {f'<p><strong>Notes:</strong> {report.notes}</p>' if report.notes else ''}
    </div>

    <div class="stats">
        <h2>Vulnerability Statistics</h2>
        <div class="stat-box critical">
            <h3>Critical</h3>
            <p>{report.stats.get('critical', 0) if report.stats else 0}</p>
        </div>
        <div class="stat-box high">
            <h3>High</h3>
            <p>{report.stats.get('high', 0) if report.stats else 0}</p>
        </div>
        <div class="stat-box medium">
            <h3>Medium</h3>
            <p>{report.stats.get('medium', 0) if report.stats else 0}</p>
        </div>
        <div class="stat-box low">
            <h3>Low</h3>
            <p>{report.stats.get('low', 0) if report.stats else 0}</p>
        </div>
    </div>

    {f'<div class="results"><h2>Summary</h2><p>{report.summary}</p></div>' if report.summary else ''}
</body>
</html>
        """

        return html_template