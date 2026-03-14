// Meriadoc Web UI

// Store task data for modal
let tasksData = {};
let currentTaskName = null;
let infoTaskName = null;

// Track collapsed state of project groups
let collapsedProjects = new Set();

async function loadProjects() {
    try {
        const response = await fetch('/api/projects');
        const data = await response.json();
        const list = document.getElementById('project-list');
        list.innerHTML = '';

        if (data.projects.length === 0) {
            list.innerHTML = '<li class="empty">No projects loaded</li>';
            return;
        }

        data.projects.forEach(project => {
            const li = document.createElement('li');
            li.className = 'project-item';
            li.innerHTML = `
                <span class="project-name">${project.name}</span>
                <span class="project-counts">${project.task_count}/${project.job_count}/${project.shell_count}</span>
            `;
            li.title = `${project.root}\nTasks: ${project.task_count}, Jobs: ${project.job_count}, Shells: ${project.shell_count}`;
            list.appendChild(li);
        });
    } catch (error) {
        console.error('Failed to load projects:', error);
        document.getElementById('project-list').innerHTML =
            '<li class="error">Failed to load projects</li>';
    }
}

async function loadTasks() {
    try {
        const response = await fetch('/api/tasks');
        const data = await response.json();

        // Store task data
        tasksData = {};
        data.tasks.forEach(task => {
            const qualifiedName = `${task.project}:${task.name}`;
            tasksData[qualifiedName] = task;
        });

        // Group tasks by project
        const groups = {};
        data.tasks.forEach(task => {
            if (!groups[task.project]) groups[task.project] = [];
            groups[task.project].push(task);
        });

        const container = document.getElementById('task-groups');
        container.innerHTML = '';

        if (Object.keys(groups).length === 0) {
            container.innerHTML = '<div class="empty">No tasks found</div>';
            return;
        }

        for (const [project, projectTasks] of Object.entries(groups)) {
            const group = createProjectGroup(project, projectTasks);
            container.appendChild(group);
        }
    } catch (error) {
        console.error('Failed to load tasks:', error);
        document.getElementById('task-groups').innerHTML =
            '<div class="error">Failed to load tasks</div>';
    }
}

function createProjectGroup(projectName, tasks) {
    const group = document.createElement('div');
    group.className = 'project-group';
    group.dataset.project = projectName;

    const isCollapsed = collapsedProjects.has(projectName);

    const header = document.createElement('div');
    header.className = `project-header${isCollapsed ? ' collapsed' : ''}`;
    header.innerHTML = `
        <span class="arrow">&#9660;</span>
        <span class="project-group-name">${projectName}</span>
        <span class="task-count">${tasks.length}</span>
    `;
    header.onclick = () => toggleProjectGroup(projectName);

    const taskList = document.createElement('div');
    taskList.className = 'project-tasks';
    taskList.id = `tasks-${projectName}`;
    if (isCollapsed) taskList.style.display = 'none';

    tasks.forEach(task => {
        const qualifiedName = `${task.project}:${task.name}`;
        const taskItem = document.createElement('div');
        taskItem.className = 'task-item';

        const hasEnv = task.env_vars && task.env_vars.length > 0;

        taskItem.innerHTML = `
            <div class="task-info">
                <span class="task-name">${task.name}</span>
                <span class="task-desc">${task.description || ''}</span>
            </div>
            <div class="task-badges">
                <span class="risk-badge risk-${task.risk_level}">${task.risk_level}</span>
                ${task.requires_approval ? '<span class="approval-badge">approval</span>' : ''}
                ${hasEnv ? '<span class="env-badge">env</span>' : ''}
            </div>
            <div class="task-actions">
                <button class="btn-info" onclick="showTaskInfo('${qualifiedName}')" title="View task info">i</button>
                <button class="btn-run" onclick="openRunModal('${qualifiedName}')" title="Run task">&#9654;</button>
            </div>
        `;
        taskList.appendChild(taskItem);
    });

    group.appendChild(header);
    group.appendChild(taskList);
    return group;
}

function toggleProjectGroup(projectName) {
    const taskList = document.getElementById(`tasks-${projectName}`);
    const header = taskList.previousElementSibling;

    if (collapsedProjects.has(projectName)) {
        collapsedProjects.delete(projectName);
        taskList.style.display = 'block';
        header.classList.remove('collapsed');
    } else {
        collapsedProjects.add(projectName);
        taskList.style.display = 'none';
        header.classList.add('collapsed');
    }
}

async function showTaskInfo(taskName) {
    try {
        const response = await fetch(`/api/tasks/${encodeURIComponent(taskName)}/info`);
        if (!response.ok) {
            throw new Error(`Failed to fetch task info: ${response.statusText}`);
        }
        const info = await response.json();
        renderInfoModal(info, taskName);
    } catch (error) {
        console.error('Failed to load task info:', error);
        alert(`Failed to load task info: ${error.message}`);
    }
}

function renderInfoModal(info, taskName) {
    infoTaskName = taskName;

    const modal = document.getElementById('info-modal');
    const title = document.getElementById('info-modal-title');
    const body = document.getElementById('info-modal-body');
    const runBtn = document.getElementById('info-run-btn');

    title.textContent = taskName;

    let html = '';

    // Description
    if (info.description) {
        html += `<div class="info-section">
            <h4>Description</h4>
            <p>${info.description}</p>
        </div>`;
    }

    // Project & Risk
    html += `<div class="info-section info-meta">
        <div class="meta-item">
            <span class="meta-label">Project:</span>
            <span class="meta-value">${info.project}</span>
        </div>
        <div class="meta-item">
            <span class="meta-label">Risk Level:</span>
            <span class="risk-badge risk-${info.risk_level}">${info.risk_level}</span>
        </div>
        ${info.requires_approval ? `<div class="meta-item">
            <span class="meta-label">Approval:</span>
            <span class="approval-badge">required</span>
        </div>` : ''}
    </div>`;

    // Working directory
    if (info.workdir) {
        html += `<div class="info-section">
            <h4>Working Directory</h4>
            <code>${info.workdir}</code>
        </div>`;
    }

    // Commands
    if (info.cmds && info.cmds.length > 0) {
        html += `<div class="info-section">
            <h4>Commands (${info.cmds.length})</h4>
            <div class="cmd-list">`;
        info.cmds.forEach((cmd, i) => {
            html += `<div class="cmd-item"><span class="cmd-num">${i + 1}.</span> ${escapeHtml(cmd)}</div>`;
        });
        html += `</div></div>`;
    }

    // Environment variables
    if (info.env_vars && info.env_vars.length > 0) {
        html += `<div class="info-section">
            <h4>Environment Variables (${info.env_vars.length})</h4>
            <table class="env-table">
                <thead>
                    <tr><th>Name</th><th>Type</th><th>Default</th><th>Required</th></tr>
                </thead>
                <tbody>`;
        info.env_vars.forEach(v => {
            const defaultVal = v.default || '-';
            const typeClass = `type-${v.type}`;
            html += `<tr>
                <td class="env-name">${v.name}</td>
                <td><span class="type-hint ${typeClass}">${v.type}</span></td>
                <td class="env-default">${escapeHtml(defaultVal)}</td>
                <td>${v.required ? '<span class="required-yes">yes</span>' : 'no'}</td>
            </tr>`;
            if (v.options && v.options.length > 0) {
                html += `<tr class="env-options-row">
                    <td colspan="4">Options: ${v.options.map(o => `<code>${o}</code>`).join(', ')}</td>
                </tr>`;
            }
        });
        html += `</tbody></table></div>`;
    }

    // Env files
    if (info.env_files && info.env_files.length > 0) {
        html += `<div class="info-section">
            <h4>Environment Files</h4>
            <ul class="env-files-list">`;
        info.env_files.forEach(f => {
            html += `<li><code>${escapeHtml(f)}</code></li>`;
        });
        html += `</ul></div>`;
    }

    // Preconditions & On failure
    if (info.has_preconditions || info.has_on_failure) {
        html += `<div class="info-section info-flags">`;
        if (info.has_preconditions) {
            html += `<span class="flag-badge">Has preconditions</span>`;
        }
        if (info.has_on_failure) {
            html += `<span class="flag-badge">Has failure handler</span>`;
        }
        html += `</div>`;
    }

    body.innerHTML = html;

    // Set up run button
    runBtn.onclick = () => {
        closeInfoModal();
        openRunModal(taskName);
    };

    modal.classList.remove('hidden');
}

function closeInfoModal() {
    const modal = document.getElementById('info-modal');
    modal.classList.add('hidden');
    infoTaskName = null;
}

function escapeHtml(str) {
    const div = document.createElement('div');
    div.textContent = str;
    return div.innerHTML;
}

/**
 * Create an input element based on variable type.
 */
function createInputForType(envVar) {
    const varType = envVar.type || 'string';
    let input;
    let wrapper = null;

    switch (varType) {
        case 'choice':
            // Dropdown for choice type
            input = document.createElement('select');
            if (!envVar.required) {
                const emptyOpt = document.createElement('option');
                emptyOpt.value = '';
                emptyOpt.textContent = '-- Select --';
                input.appendChild(emptyOpt);
            }
            (envVar.options || []).forEach(opt => {
                const option = document.createElement('option');
                option.value = opt;
                option.textContent = opt;
                if (envVar.default === opt) {
                    option.selected = true;
                }
                input.appendChild(option);
            });
            break;

        case 'boolean':
            // Checkbox for boolean
            wrapper = document.createElement('div');
            wrapper.className = 'checkbox-wrapper';
            input = document.createElement('input');
            input.type = 'checkbox';
            input.checked = envVar.default === 'true';
            const checkLabel = document.createElement('span');
            checkLabel.className = 'checkbox-label';
            checkLabel.textContent = envVar.default === 'true' ? 'true' : 'false';
            input.addEventListener('change', () => {
                checkLabel.textContent = input.checked ? 'true' : 'false';
            });
            wrapper.appendChild(input);
            wrapper.appendChild(checkLabel);
            break;

        case 'number':
            // Number input (allows decimals)
            input = document.createElement('input');
            input.type = 'number';
            input.step = 'any';
            input.placeholder = envVar.default || '0';
            if (envVar.default) {
                input.value = envVar.default;
            }
            break;

        case 'integer':
            // Integer input (whole numbers only)
            input = document.createElement('input');
            input.type = 'number';
            input.step = '1';
            input.placeholder = envVar.default || '0';
            if (envVar.default) {
                input.value = envVar.default;
            }
            break;

        case 'secret':
            // Password input for secrets
            input = document.createElement('input');
            input.type = 'password';
            input.placeholder = envVar.default ? '********' : 'Enter secret...';
            input.autocomplete = 'off';
            // Don't pre-fill secrets for security
            break;

        case 'filepath':
            // Text input with filepath styling (future: add browse button)
            wrapper = document.createElement('div');
            wrapper.className = 'filepath-wrapper';
            input = document.createElement('input');
            input.type = 'text';
            input.placeholder = envVar.default || '/path/to/file';
            input.className = 'filepath-input';
            if (envVar.default) {
                input.value = envVar.default;
            }
            const pathIcon = document.createElement('span');
            pathIcon.className = 'filepath-icon';
            pathIcon.textContent = '\u{1F4C1}';
            wrapper.appendChild(input);
            wrapper.appendChild(pathIcon);
            break;

        case 'string':
        default:
            // Default text input
            input = document.createElement('input');
            input.type = 'text';
            input.placeholder = envVar.default || '';
            if (envVar.default) {
                input.value = envVar.default;
            }
            break;
    }

    input.id = `env-${envVar.name}`;
    input.name = envVar.name;

    if (envVar.required && varType !== 'boolean') {
        input.required = true;
    }

    return wrapper || input;
}

/**
 * Get the value from an input element based on its type.
 */
function getInputValue(envVar) {
    const varType = envVar.type || 'string';
    const input = document.getElementById(`env-${envVar.name}`);

    if (!input) return null;

    if (varType === 'boolean') {
        return input.checked ? 'true' : 'false';
    }

    return input.value;
}

function openRunModal(taskName) {
    const task = tasksData[taskName];
    if (!task) return;

    currentTaskName = taskName;
    const modal = document.getElementById('env-modal');
    const title = document.getElementById('modal-title');
    const fields = document.getElementById('env-fields');

    title.textContent = `Run: ${taskName}`;
    fields.innerHTML = '';

    // Add run options section
    const optionsSection = document.createElement('div');
    optionsSection.className = 'run-options';
    optionsSection.innerHTML = `
        <div class="option-row">
            <label class="checkbox-option">
                <input type="checkbox" id="opt-dry-run" name="_dry_run">
                <span>Dry Run</span>
                <span class="option-hint">Preview commands without executing</span>
            </label>
        </div>
    `;
    fields.appendChild(optionsSection);

    // Add env vars section
    if (task.env_vars && task.env_vars.length > 0) {
        const envSection = document.createElement('div');
        envSection.className = 'env-section';

        const envHeader = document.createElement('div');
        envHeader.className = 'env-header';
        envHeader.textContent = 'Environment Variables';
        envSection.appendChild(envHeader);

        task.env_vars.forEach(envVar => {
            const fieldDiv = document.createElement('div');
            fieldDiv.className = 'form-field';

            const label = document.createElement('label');
            label.setAttribute('for', `env-${envVar.name}`);

            const varType = envVar.type || 'string';
            const typeClass = `type-${varType}`;

            label.innerHTML = `
                ${envVar.name}
                ${envVar.required ? '<span class="required">*</span>' : ''}
                <span class="type-hint ${typeClass}">${varType}</span>
            `;

            const inputEl = createInputForType(envVar);

            fieldDiv.appendChild(label);
            fieldDiv.appendChild(inputEl);
            envSection.appendChild(fieldDiv);
        });

        fields.appendChild(envSection);
    } else {
        const noEnv = document.createElement('p');
        noEnv.className = 'no-env';
        noEnv.textContent = 'No environment variables required.';
        fields.appendChild(noEnv);
    }

    modal.classList.remove('hidden');
}

function closeModal() {
    const modal = document.getElementById('env-modal');
    modal.classList.add('hidden');
    currentTaskName = null;
}

function submitEnvForm(event) {
    event.preventDefault();

    if (!currentTaskName) return;

    const taskName = currentTaskName; // Capture before closeModal resets it
    const task = tasksData[taskName];

    // Get dry run option
    const dryRun = document.getElementById('opt-dry-run')?.checked || false;

    // Collect env vars using type-aware value extraction
    const env = [];
    if (task && task.env_vars) {
        task.env_vars.forEach(envVar => {
            const value = getInputValue(envVar);
            if (value !== null && value !== '') {
                env.push([envVar.name, value]);
            }
        });
    }

    closeModal();
    runTask(taskName, env, dryRun);
}

async function runTask(name, env = [], dryRun = false) {
    const consoleEl = document.getElementById('console');
    const currentTaskEl = document.getElementById('current-task');

    const mode = dryRun ? 'Dry run' : 'Running';
    currentTaskEl.textContent = `${mode}: ${name}`;
    consoleEl.textContent = '';

    try {
        // Use POST with env vars
        const response = await fetch(`/api/tasks/${encodeURIComponent(name)}/run`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({ env, dry_run: dryRun }),
        });

        const result = await response.json();

        if (result.success) {
            consoleEl.textContent = result.output || '[No output]';
            currentTaskEl.textContent = `Completed: ${name}`;
        } else {
            consoleEl.textContent = result.output;
            currentTaskEl.textContent = `Failed: ${name}`;
        }
    } catch (error) {
        consoleEl.textContent = `Error: ${error.message}\n`;
        currentTaskEl.textContent = `Error: ${name}`;
    }
}

function clearOutput() {
    document.getElementById('console').textContent = '';
    document.getElementById('current-task').textContent = '';
}

// Close modals on escape key
document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') {
        closeModal();
        closeInfoModal();
    }
});

// Close modals on backdrop click
document.getElementById('env-modal').addEventListener('click', (e) => {
    if (e.target.id === 'env-modal') {
        closeModal();
    }
});

document.getElementById('info-modal').addEventListener('click', (e) => {
    if (e.target.id === 'info-modal') {
        closeInfoModal();
    }
});

// Initial load
document.addEventListener('DOMContentLoaded', () => {
    loadProjects();
    loadTasks();
});
