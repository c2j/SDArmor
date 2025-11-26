// 侧边栏折叠功能
document.addEventListener('DOMContentLoaded', function() {
    const sidebarCollapse = document.getElementById('sidebarCollapse');
    const sidebar = document.getElementById('sidebar');
    const content = document.getElementById('content');
    
    if (sidebarCollapse) {
        sidebarCollapse.addEventListener('click', function() {
            sidebar.classList.toggle('collapsed');
            content.classList.toggle('expanded');
        });
    }
    
    // 自动关闭警告消息
    const alerts = document.querySelectorAll('.alert');
    alerts.forEach(alert => {
        setTimeout(() => {
            const bsAlert = new bootstrap.Alert(alert);
            bsAlert.close();
        }, 5000);
    });
    
    // 表格排序和搜索功能
    const tables = document.querySelectorAll('.table');
    tables.forEach(table => {
        if (table.classList.contains('sortable')) {
            makeSortable(table);
        }
    });
    
    // 密码强度检查
    const passwordInputs = document.querySelectorAll('input[type="password"]');
    passwordInputs.forEach(input => {
        if (input.classList.contains('check-strength')) {
            input.addEventListener('input', checkPasswordStrength);
        }
    });
    
    // 表单验证
    const forms = document.querySelectorAll('form.needs-validation');
    forms.forEach(form => {
        form.addEventListener('submit', function(event) {
            if (!form.checkValidity()) {
                event.preventDefault();
                event.stopPropagation();
            }
            form.classList.add('was-validated');
        });
    });
});

// 表格排序功能
function makeSortable(table) {
    const headers = table.querySelectorAll('th');
    headers.forEach((header, index) => {
        if (!header.classList.contains('no-sort')) {
            header.style.cursor = 'pointer';
            header.addEventListener('click', () => {
                sortTable(table, index);
            });
        }
    });
}

function sortTable(table, column) {
    const tbody = table.querySelector('tbody');
    const rows = Array.from(tbody.querySelectorAll('tr'));
    const headers = table.querySelectorAll('th');
    
    // 确定排序方向
    const currentDir = headers[column].getAttribute('data-sort') || 'asc';
    const newDir = currentDir === 'asc' ? 'desc' : 'asc';
    
    // 更新所有表头的排序状态
    headers.forEach(header => {
        header.removeAttribute('data-sort');
        header.querySelector('i')?.remove();
    });
    
    // 设置当前表头的排序状态
    headers[column].setAttribute('data-sort', newDir);
    const icon = document.createElement('i');
    icon.className = `bi bi-sort-${newDir === 'asc' ? 'up' : 'down'} ms-1`;
    headers[column].appendChild(icon);
    
    // 排序行
    rows.sort((a, b) => {
        const cellA = a.querySelectorAll('td')[column].textContent.trim();
        const cellB = b.querySelectorAll('td')[column].textContent.trim();
        
        // 尝试数字排序
        const numA = parseFloat(cellA);
        const numB = parseFloat(cellB);
        
        if (!isNaN(numA) && !isNaN(numB)) {
            return newDir === 'asc' ? numA - numB : numB - numA;
        }
        
        // 字符串排序
        return newDir === 'asc' 
            ? cellA.localeCompare(cellB, 'zh-CN') 
            : cellB.localeCompare(cellA, 'zh-CN');
    });
    
    // 重新添加排序后的行
    rows.forEach(row => tbody.appendChild(row));
}

// 密码强度检查
function checkPasswordStrength(event) {
    const password = event.target.value;
    const strengthMeter = document.getElementById('password-strength');
    
    if (!strengthMeter) return;
    
    // 密码强度评分
    let score = 0;
    
    // 长度检查
    if (password.length >= 8) score += 1;
    if (password.length >= 12) score += 1;
    
    // 复杂性检查
    if (/[A-Z]/.test(password)) score += 1;
    if (/[a-z]/.test(password)) score += 1;
    if (/[0-9]/.test(password)) score += 1;
    if (/[^A-Za-z0-9]/.test(password)) score += 1;
    
    // 更新强度指示器
    let strengthClass = '';
    let strengthText = '';
    
    if (score < 3) {
        strengthClass = 'bg-danger';
        strengthText = '弱';
    } else if (score < 5) {
        strengthClass = 'bg-warning';
        strengthText = '中';
    } else {
        strengthClass = 'bg-success';
        strengthText = '强';
    }
    
    strengthMeter.className = `progress-bar ${strengthClass}`;
    strengthMeter.style.width = `${(score / 6) * 100}%`;
    strengthMeter.textContent = strengthText;
}

// 确认对话框
function confirmAction(message, callback) {
    if (confirm(message)) {
        callback();
    }
}

// 格式化日期时间
function formatDateTime(dateString) {
    const date = new Date(dateString);
    return date.toLocaleString('zh-CN', {
        year: 'numeric',
        month: '2-digit',
        day: '2-digit',
        hour: '2-digit',
        minute: '2-digit'
    });
}

// 格式化文件大小
function formatFileSize(bytes) {
    if (bytes === 0) return '0 Bytes';
    
    const k = 1024;
    const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

// AJAX请求辅助函数
async function fetchData(url, options = {}) {
    try {
        const response = await fetch(url, options);
        
        if (!response.ok) {
            throw new Error(`HTTP error! Status: ${response.status}`);
        }
        
        return await response.json();
    } catch (error) {
        console.error('Fetch error:', error);
        throw error;
    }
}
