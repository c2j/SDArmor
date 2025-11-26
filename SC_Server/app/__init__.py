import os
from flask import Flask
from flask_cors import CORS
from flask_jwt_extended import JWTManager
from flask_sqlalchemy import SQLAlchemy
from flask_login import LoginManager, current_user
from datetime import datetime, timedelta
from dotenv import load_dotenv

# Load environment variables from .env file
load_dotenv()

# Initialize extensions
db = SQLAlchemy()
jwt = JWTManager()
login_manager = LoginManager()

def create_app(test_config=None):
    """Create and configure the Flask application."""
    app = Flask(__name__, instance_relative_config=True)

    # Configure the app
    app.config.from_mapping(
        SECRET_KEY=os.environ.get('SECRET_KEY', 'dev-key-for-development-only'),
        SQLALCHEMY_DATABASE_URI=os.environ.get('DATABASE_URI', 'sqlite:///sdchat.db'),
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
    login_manager.init_app(app)
    login_manager.login_view = 'main.login'

    # Setup CORS
    CORS(app, resources={r"/api/*": {"origins": "*"}})

    # User loader for Flask-Login
    from app.models import User

    @login_manager.user_loader
    def load_user(user_id):
        return User.query.get(int(user_id))

    # Register blueprints
    from app.api.rules import rules_bp
    from app.api.reports import reports_bp
    from app.api.auth import auth_bp

    app.register_blueprint(rules_bp, url_prefix='/api/v1/rules')
    app.register_blueprint(reports_bp, url_prefix='/api/v1/reports')
    app.register_blueprint(auth_bp, url_prefix='/api/v1/auth')

    from flask import Blueprint, render_template, redirect, url_for, request, flash, jsonify
    from flask_login import login_user, logout_user, login_required
    from app.models import User, RuleSet, FileType, Pattern, Report
    from werkzeug.security import check_password_hash

    main_bp = Blueprint('main', __name__)

    @main_bp.route('/')
    @login_required
    def dashboard():
        # 获取统计数据
        total_reports = Report.query.count()
        total_users = User.query.count()

        # 获取活跃规则集
        active_ruleset = RuleSet.query.filter_by(is_active=True).first()
        active_ruleset_name = active_ruleset.name if active_ruleset else "无"

        # 计算规则总数
        total_patterns = 0
        if active_ruleset:
            for file_type in active_ruleset.file_types:
                total_patterns += len(file_type.patterns)

        # 获取最近的报告
        recent_reports = Report.query.order_by(Report.created_at.desc()).limit(5).all()

        # 计算漏洞严重性分布
        severity_stats = {"critical": 0, "high": 0, "medium": 0, "low": 0}
        for report in Report.query.all():
            if report.stats:
                for severity in severity_stats:
                    severity_stats[severity] += report.stats.get(severity, 0)

        # 准备图表数据 - 最近7天的报告数量
        from sqlalchemy import func
        from datetime import datetime, timedelta

        # 获取最近7天的日期
        end_date = datetime.now()
        start_date = end_date - timedelta(days=6)

        # 查询每天的报告数量
        daily_reports = db.session.query(
            func.date(Report.created_at).label('date'),
            func.count().label('count')
        ).filter(
            Report.created_at >= start_date,
            Report.created_at <= end_date
        ).group_by('date').all()

        # 准备图表数据
        report_dates = []
        report_counts = []

        # 填充所有7天的数据（包括没有报告的日期）
        current_date = start_date
        while current_date <= end_date:
            date_str = current_date.strftime('%Y-%m-%d')
            report_dates.append(date_str)

            # 查找当天的报告数量
            count = 0
            for date_obj, count_val in daily_reports:
                # 将date_obj转换为字符串进行比较
                date_str_from_db = date_obj
                if hasattr(date_obj, 'strftime'):
                    date_str_from_db = date_obj.strftime('%Y-%m-%d')

                if date_str_from_db == date_str:
                    count = count_val
                    break

            report_counts.append(count)
            current_date += timedelta(days=1)

        # 将数据传递给模板
        import json
        return render_template('dashboard.html',
                              total_reports=total_reports,
                              total_users=total_users,
                              active_ruleset_name=active_ruleset_name,
                              total_patterns=total_patterns,
                              recent_reports=recent_reports,
                              severity_stats=severity_stats,
                              report_dates=json.dumps(report_dates),
                              report_counts=json.dumps(report_counts))

    @main_bp.route('/login', methods=['GET', 'POST'])
    def login():
        if current_user.is_authenticated:
            return redirect(url_for('main.dashboard'))

        if request.method == 'POST':
            username = request.form.get('username')
            password = request.form.get('password')
            remember = True if request.form.get('remember') else False

            user = User.query.filter_by(username=username).first()

            if user and user.check_password(password):
                login_user(user, remember=remember)
                next_page = request.args.get('next')
                return redirect(next_page or url_for('main.dashboard'))
            else:
                flash('用户名或密码错误', 'danger')

        return render_template('auth/login.html')

    @main_bp.route('/logout')
    @login_required
    def logout():
        logout_user()
        return redirect(url_for('main.login'))

    @main_bp.route('/rules')
    @login_required
    def rules():
        # 获取所有规则集
        rulesets = RuleSet.query.all()
        # 获取活跃的规则集
        active_ruleset = RuleSet.query.filter_by(is_active=True).first()

        # 如果有活跃规则集，计算严重性统计
        active_stats = {"critical": 0, "high": 0, "medium": 0, "low": 0}
        if active_ruleset:
            for file_type in active_ruleset.file_types:
                for pattern in file_type.patterns:
                    if pattern.severity in active_stats:
                        active_stats[pattern.severity] += 1

        return render_template('rules/index.html',
                              rulesets=rulesets,
                              active_ruleset=active_ruleset,
                              active_stats=active_stats)

    @main_bp.route('/rules/upload', methods=['GET', 'POST'])
    @login_required
    def upload_rules():
        if request.method == 'POST':
            # 这里应该有处理上传规则集的逻辑
            # 但现在我们只是简单地返回模板
            flash('规则集上传功能尚未实现', 'warning')
            return redirect(url_for('main.rules'))
        return render_template('rules/upload.html')

    @main_bp.route('/rules/import', methods=['GET', 'POST'])
    @login_required
    def import_rules():
        if request.method == 'POST':
            # 处理规则导入
            if 'file' not in request.files:
                flash('没有选择文件', 'error')
                return redirect(request.url)

            file = request.files['file']
            if file.filename == '':
                flash('没有选择文件', 'error')
                return redirect(request.url)

            if not file.filename.lower().endswith('.json'):
                flash('只支持JSON格式的规则文件', 'error')
                return redirect(request.url)

            try:
                import json
                from jsonschema import validate, ValidationError
                from app.api.rules import RULE_SCHEMA

                # 读取并解析JSON文件
                content = file.read().decode('utf-8')
                rule_data = json.loads(content)

                # 验证规则格式
                validate(instance=rule_data, schema=RULE_SCHEMA)

                # 检查是否已存在相同版本的规则集
                existing_rule = RuleSet.query.filter_by(version=rule_data['version']).first()
                if existing_rule:
                    flash(f'版本 {rule_data["version"]} 的规则集已存在', 'error')
                    return redirect(request.url)

                # 创建新规则集
                rule_set = RuleSet(
                    name=rule_data.get('name', f'导入的规则集 {rule_data["version"]}'),
                    version=rule_data['version'],
                    description=rule_data.get('description', '通过文件导入的规则集'),
                    is_active=request.form.get('activate') == 'on'
                )

                # 如果要激活新规则集，先停用其他规则集
                if rule_set.is_active:
                    RuleSet.query.update({'is_active': False})

                # 添加文件类型和规则
                for ft_data in rule_data['file_types']:
                    file_type = FileType(
                        name=ft_data['name'],
                        identifiers=ft_data['identifiers']
                    )

                    # 添加规则模式
                    for pattern_data in ft_data['patterns']:
                        pattern = Pattern(
                            id_code=pattern_data['id'],
                            description=pattern_data['description'],
                            severity=pattern_data['severity'],
                            regex=pattern_data['regex']
                        )
                        file_type.patterns.append(pattern)

                    rule_set.file_types.append(file_type)

                db.session.add(rule_set)
                db.session.commit()

                flash(f'规则集 "{rule_set.name}" 导入成功', 'success')
                return redirect(url_for('main.rules'))

            except json.JSONDecodeError as e:
                flash(f'JSON格式错误: {str(e)}', 'error')
            except ValidationError as e:
                flash(f'规则格式验证失败: {str(e)}', 'error')
            except Exception as e:
                db.session.rollback()
                flash(f'导入失败: {str(e)}', 'error')

            return redirect(request.url)

        return render_template('rules/import.html')

    @main_bp.route('/rules/<int:ruleset_id>/edit', methods=['GET', 'POST'])
    @login_required
    def edit_ruleset(ruleset_id):
        ruleset = RuleSet.query.get_or_404(ruleset_id)

        if request.method == 'POST':
            try:
                # 更新基本信息
                ruleset.name = request.form.get('name', ruleset.name)
                ruleset.description = request.form.get('description', ruleset.description)

                # 处理文件类型和规则的更新
                # 这里可以添加更复杂的编辑逻辑

                db.session.commit()
                flash(f'规则集 "{ruleset.name}" 更新成功', 'success')
                return redirect(url_for('main.view_ruleset', ruleset_id=ruleset.id))

            except Exception as e:
                db.session.rollback()
                flash(f'更新失败: {str(e)}', 'error')

        # 计算统计信息
        total_patterns = 0
        for file_type in ruleset.file_types:
            total_patterns += len(file_type.patterns)

        return render_template('rules/edit.html',
                             ruleset=ruleset,
                             total_patterns=total_patterns)

    @main_bp.route('/rules/sample')
    @login_required
    def download_sample_rules():
        """下载示例规则文件"""
        import json
        import tempfile
        from flask import send_file

        sample_rules = {
            "name": "示例安全规则集",
            "version": "1.0.0",
            "description": "这是一个示例规则集，展示了规则文件的标准格式",
            "file_types": [
                {
                    "name": "JavaScript Files",
                    "identifiers": [
                        {
                            "type": "extension",
                            "pattern": "\\.(js|jsx|ts|tsx)$"
                        },
                        {
                            "type": "content",
                            "pattern": "^\\s*(import|export|require)"
                        }
                    ],
                    "patterns": [
                        {
                            "id": "JS-001",
                            "description": "不安全的eval函数使用",
                            "severity": "high",
                            "regex": "eval\\s*\\("
                        },
                        {
                            "id": "JS-002",
                            "description": "潜在的XSS漏洞 - innerHTML使用",
                            "severity": "medium",
                            "regex": "\\.innerHTML\\s*="
                        },
                        {
                            "id": "JS-003",
                            "description": "不安全的随机数生成",
                            "severity": "low",
                            "regex": "Math\\.random\\(\\)"
                        }
                    ]
                },
                {
                    "name": "Python Files",
                    "identifiers": [
                        {
                            "type": "extension",
                            "pattern": "\\.py$"
                        },
                        {
                            "type": "content",
                            "pattern": "^#!/usr/bin/env python"
                        }
                    ],
                    "patterns": [
                        {
                            "id": "PY-001",
                            "description": "不安全的exec函数使用",
                            "severity": "critical",
                            "regex": "exec\\s*\\("
                        },
                        {
                            "id": "PY-002",
                            "description": "SQL注入风险 - 字符串拼接",
                            "severity": "high",
                            "regex": "SELECT.*\\+.*%s"
                        },
                        {
                            "id": "PY-003",
                            "description": "不安全的pickle使用",
                            "severity": "medium",
                            "regex": "pickle\\.loads?\\("
                        }
                    ]
                }
            ]
        }

        # 创建临时文件
        temp_file = tempfile.NamedTemporaryFile(mode='w', delete=False, suffix='.json')
        json.dump(sample_rules, temp_file, indent=2, ensure_ascii=False)
        temp_file.close()

        return send_file(
            temp_file.name,
            mimetype='application/json',
            as_attachment=True,
            download_name='sample_rules.json'
        )

    @main_bp.route('/api/rules/file-types', methods=['POST'])
    @login_required
    def add_file_type_web():
        """Add a new file type to a rule set (Web interface)"""
        if not current_user.is_admin:
            return jsonify({'error': '需要管理员权限'}), 403

        try:
            data = request.get_json()

            if not data:
                return jsonify({'error': '没有提供数据'}), 400

            ruleset_id = data.get('ruleset_id')
            name = data.get('name')
            identifiers = data.get('identifiers')

            if not all([ruleset_id, name, identifiers]):
                return jsonify({'error': '缺少必需字段'}), 400

            # 验证规则集存在
            ruleset = RuleSet.query.get(ruleset_id)
            if not ruleset:
                return jsonify({'error': '规则集不存在'}), 404

            # 创建新文件类型
            file_type = FileType(
                name=name,
                identifiers=identifiers
            )

            ruleset.file_types.append(file_type)
            db.session.commit()

            return jsonify({
                'id': file_type.id,
                'message': f'文件类型 "{name}" 添加成功'
            }), 201

        except Exception as e:
            db.session.rollback()
            return jsonify({'error': f'添加失败: {str(e)}'}), 500

    @main_bp.route('/api/rules/file-types/<int:file_type_id>', methods=['DELETE'])
    @login_required
    def delete_file_type_web(file_type_id):
        """Delete a file type (Web interface)"""
        if not current_user.is_admin:
            return jsonify({'error': '需要管理员权限'}), 403

        try:
            file_type = FileType.query.get(file_type_id)
            if not file_type:
                return jsonify({'error': '文件类型不存在'}), 404

            name = file_type.name
            db.session.delete(file_type)
            db.session.commit()

            return jsonify({
                'message': f'文件类型 "{name}" 删除成功'
            })

        except Exception as e:
            db.session.rollback()
            return jsonify({'error': f'删除失败: {str(e)}'}), 500

    @main_bp.route('/api/rules/patterns', methods=['POST'])
    @login_required
    def add_pattern_web():
        """Add a new pattern to a file type (Web interface)"""
        if not current_user.is_admin:
            return jsonify({'error': '需要管理员权限'}), 403

        try:
            data = request.get_json()

            if not data:
                return jsonify({'error': '没有提供数据'}), 400

            file_type_id = data.get('file_type_id')
            id_code = data.get('id_code')
            description = data.get('description')
            severity = data.get('severity')
            regex = data.get('regex')

            if not all([file_type_id, id_code, description, severity, regex]):
                return jsonify({'error': '缺少必需字段'}), 400

            # 验证文件类型存在
            file_type = FileType.query.get(file_type_id)
            if not file_type:
                return jsonify({'error': '文件类型不存在'}), 404

            # 验证严重性级别
            if severity not in ['critical', 'high', 'medium', 'low']:
                return jsonify({'error': '无效的严重性级别'}), 400

            # 检查ID是否已存在
            existing_pattern = Pattern.query.filter_by(id_code=id_code).first()
            if existing_pattern:
                return jsonify({'error': f'规则ID "{id_code}" 已存在'}), 409

            # 创建新规则模式
            pattern = Pattern(
                id_code=id_code,
                description=description,
                severity=severity,
                regex=regex
            )

            file_type.patterns.append(pattern)
            db.session.commit()

            return jsonify({
                'id': pattern.id,
                'message': f'规则 "{id_code}" 添加成功'
            }), 201

        except Exception as e:
            db.session.rollback()
            return jsonify({'error': f'添加失败: {str(e)}'}), 500

    @main_bp.route('/api/rules/patterns/<int:pattern_id>', methods=['PUT'])
    @login_required
    def update_pattern_web(pattern_id):
        """Update a pattern (Web interface)"""
        if not current_user.is_admin:
            return jsonify({'error': '需要管理员权限'}), 403

        try:
            data = request.get_json()

            if not data:
                return jsonify({'error': '没有提供数据'}), 400

            pattern = Pattern.query.get(pattern_id)
            if not pattern:
                return jsonify({'error': '规则不存在'}), 404

            # 更新字段
            if 'id_code' in data:
                # 检查新ID是否已存在（排除当前规则）
                existing = Pattern.query.filter(
                    Pattern.id_code == data['id_code'],
                    Pattern.id != pattern_id
                ).first()
                if existing:
                    return jsonify({'error': f'规则ID "{data["id_code"]}" 已存在'}), 409
                pattern.id_code = data['id_code']

            if 'description' in data:
                pattern.description = data['description']

            if 'severity' in data:
                if data['severity'] not in ['critical', 'high', 'medium', 'low']:
                    return jsonify({'error': '无效的严重性级别'}), 400
                pattern.severity = data['severity']

            if 'regex' in data:
                pattern.regex = data['regex']

            db.session.commit()

            return jsonify({
                'id': pattern.id,
                'message': f'规则 "{pattern.id_code}" 更新成功'
            })

        except Exception as e:
            db.session.rollback()
            return jsonify({'error': f'更新失败: {str(e)}'}), 500

    @main_bp.route('/api/rules/patterns/<int:pattern_id>', methods=['DELETE'])
    @login_required
    def delete_pattern_web(pattern_id):
        """Delete a pattern (Web interface)"""
        if not current_user.is_admin:
            return jsonify({'error': '需要管理员权限'}), 403

        try:
            pattern = Pattern.query.get(pattern_id)
            if not pattern:
                return jsonify({'error': '规则不存在'}), 404

            id_code = pattern.id_code
            db.session.delete(pattern)
            db.session.commit()

            return jsonify({
                'message': f'规则 "{id_code}" 删除成功'
            })

        except Exception as e:
            db.session.rollback()
            return jsonify({'error': f'删除失败: {str(e)}'}), 500

    @main_bp.route('/rules/<int:ruleset_id>')
    @login_required
    def view_ruleset(ruleset_id):
        # 获取规则集
        ruleset = RuleSet.query.get_or_404(ruleset_id)

        # 计算统计信息
        total_patterns = 0
        severity_stats = {"critical": 0, "high": 0, "medium": 0, "low": 0}

        for file_type in ruleset.file_types:
            total_patterns += len(file_type.patterns)
            for pattern in file_type.patterns:
                if pattern.severity in severity_stats:
                    severity_stats[pattern.severity] += 1

        return render_template('rules/view.html',
                              ruleset=ruleset,
                              total_patterns=total_patterns,
                              severity_stats=severity_stats)

    @main_bp.route('/rules/<int:ruleset_id>/activate')
    @login_required
    def activate_ruleset(ruleset_id):
        # 获取规则集
        ruleset = RuleSet.query.get_or_404(ruleset_id)

        # 将所有规则集设置为非活跃
        RuleSet.query.update({RuleSet.is_active: False})

        # 将当前规则集设置为活跃
        ruleset.is_active = True
        db.session.commit()

        flash(f'规则集 "{ruleset.name}" 已激活', 'success')
        return redirect(url_for('main.rules'))

    @main_bp.route('/rules/<int:ruleset_id>/export')
    @login_required
    def export_ruleset(ruleset_id):
        # 这里应该有导出规则集的逻辑
        # 但现在我们只是简单地返回到规则集页面
        flash('规则集导出功能尚未实现', 'warning')
        return redirect(url_for('main.view_ruleset', ruleset_id=ruleset_id))

    @main_bp.route('/rules/<int:ruleset_id>/delete')
    @login_required
    def delete_ruleset(ruleset_id):
        # 获取规则集
        ruleset = RuleSet.query.get_or_404(ruleset_id)

        # 检查规则集是否为活跃状态
        if ruleset.is_active:
            flash('无法删除活跃的规则集', 'danger')
            return redirect(url_for('main.rules'))

        # 删除规则集
        db.session.delete(ruleset)
        db.session.commit()

        flash(f'规则集 "{ruleset.name}" 已删除', 'success')
        return redirect(url_for('main.rules'))

    @main_bp.route('/reports')
    @login_required
    def reports():
        # 获取分页参数
        page = request.args.get('page', 1, type=int)
        per_page = min(request.args.get('per_page', 10, type=int), 50)  # 限制最大每页数量为50

        # 构建查询
        query = Report.query

        # 应用过滤器
        if request.args.get('start_date'):
            try:
                start_date = datetime.strptime(request.args.get('start_date'), '%Y-%m-%d')
                query = query.filter(Report.created_at >= start_date)
            except ValueError:
                flash('开始日期格式无效', 'danger')

        if request.args.get('end_date'):
            try:
                end_date = datetime.strptime(request.args.get('end_date'), '%Y-%m-%d')
                # 设置为当天的结束时间
                end_date = end_date.replace(hour=23, minute=59, second=59)
                query = query.filter(Report.created_at <= end_date)
            except ValueError:
                flash('结束日期格式无效', 'danger')

        if request.args.get('severity'):
            severity = request.args.get('severity')
            # 这里需要根据实际数据结构调整过滤逻辑
            # 由于stats是JSON字段，可能需要特殊处理

        if request.args.get('search'):
            search_term = f"%{request.args.get('search')}%"
            query = query.filter(
                db.or_(
                    Report.title.ilike(search_term),
                    Report.scan_target.ilike(search_term)
                )
            )

        # 执行分页查询
        pagination = query.order_by(Report.created_at.desc()).paginate(
            page=page, per_page=per_page, error_out=False
        )

        # 获取当前页的报告
        reports = pagination.items

        return render_template('reports/index.html',
                              reports=reports,
                              pagination=pagination)

    @main_bp.route('/reports/<string:report_id>')
    @login_required
    def view_report(report_id):
        # 获取报告
        report = Report.query.filter_by(report_id=report_id).first_or_404()

        # 获取上传用户
        uploader = User.query.get(report.uploaded_by) if report.uploaded_by else None

        # 计算总漏洞数
        total_vulnerabilities = sum(report.stats.get(severity, 0)
                                   for severity in ['critical', 'high', 'medium', 'low'])

        # 处理报告结果数据，确保兼容不同的数据结构
        results_json = report.results_json

        # 如果结果中有 findings 字段但没有 hotspots 字段，确保模板能正确处理
        if isinstance(results_json, dict) and 'findings' in results_json and 'hotspots' not in results_json:
            # 保持原始数据结构不变，模板中已经添加了对 findings 的支持
            pass

        # 如果没有 findings 字段也没有 hotspots 字段，添加一个空的 findings 数组
        elif isinstance(results_json, dict) and 'findings' not in results_json and 'hotspots' not in results_json:
            results_json['findings'] = []

        return render_template('reports/view.html',
                              report=report,
                              uploader=uploader,
                              total_vulnerabilities=total_vulnerabilities,
                              results=results_json)

    @main_bp.route('/reports/<string:report_id>/export')
    @login_required
    def export_report(report_id):
        # 这里应该有导出报告的逻辑
        # 但现在我们只是简单地返回到报告页面
        flash('报告导出功能尚未实现', 'warning')
        return redirect(url_for('main.view_report', report_id=report_id))

    @main_bp.route('/reports/<string:report_id>/delete')
    @login_required
    def delete_report(report_id):
        # 获取报告
        report = Report.query.filter_by(report_id=report_id).first_or_404()

        # 删除报告
        db.session.delete(report)
        db.session.commit()

        flash(f'报告 "{report.title}" 已删除', 'success')
        return redirect(url_for('main.reports'))

    @main_bp.route('/users')
    @login_required
    def users():
        # 获取所有用户
        users_list = User.query.all()
        return render_template('users/index.html', users=users_list)

    @main_bp.route('/users/add', methods=['POST'])
    @login_required
    def add_user():
        # 检查当前用户是否为管理员
        if not current_user.is_admin:
            flash('只有管理员可以添加用户', 'danger')
            return redirect(url_for('main.users'))

        # 获取表单数据
        username = request.form.get('username')
        email = request.form.get('email')
        password = request.form.get('password')
        is_admin = True if request.form.get('is_admin') else False

        # 验证数据
        if not username or not email or not password:
            flash('所有字段都是必填的', 'danger')
            return redirect(url_for('main.users'))

        # 检查用户名和邮箱是否已存在
        if User.query.filter_by(username=username).first():
            flash(f'用户名 "{username}" 已存在', 'danger')
            return redirect(url_for('main.users'))

        if User.query.filter_by(email=email).first():
            flash(f'邮箱 "{email}" 已存在', 'danger')
            return redirect(url_for('main.users'))

        # 创建新用户
        new_user = User(username=username, email=email, is_admin=is_admin)
        new_user.set_password(password)

        # 保存到数据库
        db.session.add(new_user)
        db.session.commit()

        flash(f'用户 "{username}" 创建成功', 'success')
        return redirect(url_for('main.users'))

    @main_bp.route('/users/edit', methods=['POST'])
    @login_required
    def edit_user():
        # 检查当前用户是否为管理员
        if not current_user.is_admin:
            flash('只有管理员可以编辑用户', 'danger')
            return redirect(url_for('main.users'))

        # 获取表单数据
        user_id = request.form.get('user_id')
        username = request.form.get('username')
        email = request.form.get('email')
        is_admin = True if request.form.get('is_admin') else False
        is_active = True if request.form.get('is_active') else False

        # 验证数据
        if not user_id or not username or not email:
            flash('所有字段都是必填的', 'danger')
            return redirect(url_for('main.users'))

        # 获取用户
        user = User.query.get(user_id)
        if not user:
            flash('用户不存在', 'danger')
            return redirect(url_for('main.users'))

        # 检查用户名和邮箱是否已被其他用户使用
        username_exists = User.query.filter(User.username == username, User.id != user.id).first()
        if username_exists:
            flash(f'用户名 "{username}" 已存在', 'danger')
            return redirect(url_for('main.users'))

        email_exists = User.query.filter(User.email == email, User.id != user.id).first()
        if email_exists:
            flash(f'邮箱 "{email}" 已存在', 'danger')
            return redirect(url_for('main.users'))

        # 更新用户信息
        user.username = username
        user.email = email
        user.is_admin = is_admin
        user.is_active = is_active

        # 保存到数据库
        db.session.commit()

        flash(f'用户 "{username}" 更新成功', 'success')
        return redirect(url_for('main.users'))

    @main_bp.route('/users/<int:user_id>/delete')
    @login_required
    def delete_user(user_id):
        # 检查当前用户是否为管理员
        if not current_user.is_admin:
            flash('只有管理员可以删除用户', 'danger')
            return redirect(url_for('main.users'))

        # 不能删除自己
        if user_id == current_user.id:
            flash('不能删除当前登录的用户', 'danger')
            return redirect(url_for('main.users'))

        # 获取用户
        user = User.query.get_or_404(user_id)

        # 删除用户
        db.session.delete(user)
        db.session.commit()

        flash(f'用户 "{user.username}" 已删除', 'success')
        return redirect(url_for('main.users'))

    @main_bp.route('/users/reset-password', methods=['POST'])
    @login_required
    def reset_password():
        # 检查当前用户是否为管理员
        if not current_user.is_admin:
            flash('只有管理员可以重置密码', 'danger')
            return redirect(url_for('main.users'))

        # 获取表单数据
        user_id = request.form.get('user_id')
        new_password = request.form.get('new_password')
        confirm_password = request.form.get('confirm_password')

        # 验证数据
        if not user_id or not new_password or not confirm_password:
            flash('所有字段都是必填的', 'danger')
            return redirect(url_for('main.users'))

        if new_password != confirm_password:
            flash('两次输入的密码不匹配', 'danger')
            return redirect(url_for('main.users'))

        # 获取用户
        user = User.query.get(user_id)
        if not user:
            flash('用户不存在', 'danger')
            return redirect(url_for('main.users'))

        # 更新密码
        user.set_password(new_password)
        db.session.commit()

        flash(f'用户 "{user.username}" 的密码已重置', 'success')
        return redirect(url_for('main.users'))


    # Create database tables
    with app.app_context():
        db.create_all()

    app.register_blueprint(main_bp)

    return app