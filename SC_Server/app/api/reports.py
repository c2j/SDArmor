import os
import json
import uuid
from datetime import datetime
from flask import Blueprint, request, jsonify, current_app, send_file
from flask_jwt_extended import jwt_required, get_jwt_identity
from werkzeug.utils import secure_filename
from sqlalchemy.exc import SQLAlchemyError
from sqlalchemy.orm.exc import NoResultFound

from app import db
from app.models import Report, User
from app.utils.auth import admin_required

# Create blueprint
reports_bp = Blueprint('reports', __name__)

@reports_bp.route('', methods=['GET'])
@jwt_required()
def get_reports():
    """Get all reports with optional filtering"""
    try:
        page = request.args.get('page', 1, type=int)
        per_page = min(request.args.get('per_page', 10, type=int), 50)  # Limit to 50 max
        
        # Build query with filters
        query = Report.query
        
        # Filter by user if not admin
        current_user_id = get_jwt_identity()
        user = User.query.get(current_user_id)
        if not user.is_admin:
            query = query.filter_by(uploaded_by=current_user_id)
            
        # Apply additional filters
        if 'start_date' in request.args:
            start_date = datetime.fromisoformat(request.args['start_date'])
            query = query.filter(Report.created_at >= start_date)
            
        if 'end_date' in request.args:
            end_date = datetime.fromisoformat(request.args['end_date'])
            query = query.filter(Report.created_at <= end_date)
        
        # Execute paginated query
        reports_page = query.order_by(Report.created_at.desc()).paginate(page=page, per_page=per_page)
        
        # Format the results
        reports_data = []
        for report in reports_page.items:
            stats = report.stats or {}
            reports_data.append({
                'id': report.id,
                'report_id': str(report.report_id),
                'title': report.title,
                'scan_target': report.scan_target,
                'created_at': report.created_at.isoformat(),
                'critical_count': stats.get('critical', 0),
                'high_count': stats.get('high', 0),
                'medium_count': stats.get('medium', 0),
                'low_count': stats.get('low', 0),
            })
        
        return jsonify({
            'reports': reports_data,
            'total': reports_page.total,
            'pages': reports_page.pages,
            'current_page': reports_page.page
        })
        
    except ValueError as e:
        return jsonify({'error': f'Invalid parameter: {str(e)}'}), 400
    except SQLAlchemyError as e:
        return jsonify({'error': f'Database error: {str(e)}'}), 500

@reports_bp.route('/<string:report_id>', methods=['GET'])
@jwt_required()
def get_report(report_id):
    """Get a specific report by its UUID"""
    try:
        # Query the report by string UUID
        report = Report.query.filter_by(report_id=report_id).first_or_404()
        
        # Check access permissions
        current_user_id = get_jwt_identity()
        user = User.query.get(current_user_id)
        if not user.is_admin and report.uploaded_by != current_user_id:
            return jsonify({'error': 'Access denied'}), 403
        
        # Format the full report
        report_data = {
            'id': report.id,
            'report_id': str(report.report_id),
            'title': report.title,
            'scan_target': report.scan_target,
            'summary': report.summary,
            'created_at': report.created_at.isoformat(),
            'updated_at': report.updated_at.isoformat(),
            'stats': report.stats,
            'results': report.results_json
        }
        
        return jsonify(report_data)
        
    except ValueError:
        return jsonify({'error': 'Invalid report ID format'}), 400
    except NoResultFound:
        return jsonify({'error': 'Report not found'}), 404
    except SQLAlchemyError as e:
        return jsonify({'error': f'Database error: {str(e)}'}), 500

@reports_bp.route('', methods=['POST'])
@jwt_required()
def upload_report():
    """Upload a new scan report"""
    data = request.get_json()
    
    if not data:
        return jsonify({'error': 'No data provided'}), 400
    
    # Validate required fields
    required_fields = ['title', 'scan_target', 'results']
    for field in required_fields:
        if field not in data:
            return jsonify({'error': f'Missing required field: {field}'}), 400
    
    try:
        # Create the report record
        report = Report(
            title=data['title'],
            scan_target=data['scan_target'],
            summary=data.get('summary', ''),
            results_json=data['results'],
            stats=data.get('stats', {}),
            uploaded_by=get_jwt_identity()
        )
        
        db.session.add(report)
        db.session.commit()
        
        return jsonify({
            'message': 'Report uploaded successfully',
            'report_id': str(report.report_id)
        }), 201
        
    except SQLAlchemyError as e:
        db.session.rollback()
        return jsonify({'error': f'Database error: {str(e)}'}), 500

@reports_bp.route('/<string:report_id>', methods=['DELETE'])
@jwt_required()
def delete_report(report_id):
    """Delete a report"""
    try:
        # Query the report by string UUID
        report = Report.query.filter_by(report_id=report_id).first_or_404()
        
        # Check permissions
        current_user_id = get_jwt_identity()
        user = User.query.get(current_user_id)
        if not user.is_admin and report.uploaded_by != current_user_id:
            return jsonify({'error': 'Access denied'}), 403
        
        # Delete the report
        db.session.delete(report)
        db.session.commit()
        
        return jsonify({
            'message': 'Report deleted successfully'
        })
        
    except ValueError:
        return jsonify({'error': 'Invalid report ID format'}), 400
    except NoResultFound:
        return jsonify({'error': 'Report not found'}), 404
    except SQLAlchemyError as e:
        db.session.rollback()
        return jsonify({'error': f'Database error: {str(e)}'}), 500

@reports_bp.route('/<string:report_id>/export', methods=['GET'])
@jwt_required()
def export_report(report_id):
    """Export a report as JSON file"""
    try:
        # Query the report by string UUID
        report = Report.query.filter_by(report_id=report_id).first_or_404()
        
        # Check permissions
        current_user_id = get_jwt_identity()
        user = User.query.get(current_user_id)
        if not user.is_admin and report.uploaded_by != current_user_id:
            return jsonify({'error': 'Access denied'}), 403
        
        # Format the full report
        report_data = {
            'title': report.title,
            'report_id': str(report.report_id),
            'scan_target': report.scan_target,
            'summary': report.summary,
            'created_at': report.created_at.isoformat(),
            'stats': report.stats,
            'results': report.results_json
        }
        
        # Create the file
        filename = f"security_scan_{report.report_id}.json"
        file_path = os.path.join(current_app.config['REPORTS_DIR'], filename)
        
        with open(file_path, 'w') as f:
            json.dump(report_data, f, indent=2)
        
        return send_file(
            file_path,
            mimetype='application/json',
            as_attachment=True,
            download_name=filename
        )
        
    except ValueError:
        return jsonify({'error': 'Invalid report ID format'}), 400
    except NoResultFound:
        return jsonify({'error': 'Report not found'}), 404
    except SQLAlchemyError as e:
        return jsonify({'error': f'Database error: {str(e)}'}), 500

@reports_bp.route('/statistics', methods=['GET'])
@jwt_required()
@admin_required
def get_report_statistics():
    """Get aggregated report statistics"""
    try:
        # Get total report count
        total_reports = Report.query.count()
        
        # Get reports by date (for last 30 days)
        from sqlalchemy import func
        from datetime import timedelta
        
        thirty_days_ago = datetime.utcnow() - timedelta(days=30)
        daily_counts = db.session.query(
            func.date(Report.created_at).label('date'),
            func.count().label('count')
        ).filter(Report.created_at >= thirty_days_ago).group_by('date').all()
        
        # Convert to dictionary for JSON response
        daily_data = {str(date): count for date, count in daily_counts}
        
        # Get severity distribution
        severity_stats = {
            'critical': 0,
            'high': 0,
            'medium': 0,
            'low': 0
        }
        
        # For a real implementation, this would be more efficient with a database query
        # But for simplicity, we'll aggregate in Python
        reports = Report.query.all()
        for report in reports:
            stats = report.stats or {}
            severity_stats['critical'] += stats.get('critical', 0)
            severity_stats['high'] += stats.get('high', 0)
            severity_stats['medium'] += stats.get('medium', 0)
            severity_stats['low'] += stats.get('low', 0)
        
        return jsonify({
            'total_reports': total_reports,
            'daily_reports': daily_data,
            'severity_distribution': severity_stats
        })
        
    except SQLAlchemyError as e:
        return jsonify({'error': f'Database error: {str(e)}'}), 500