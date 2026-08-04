<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import TaskForm from './components/TaskForm.vue'
import { TASK_TYPES, CONNECT_CONFIGS, getTaskType } from './config/tasks'
import type { TaskType } from './config/tasks'

// ==================== State ====================
const version = ref('')
const healthy = ref(true)
const connected = ref(false)
const running = ref(false)
const logs = ref<Array<{ msg: string; detail: string; time: string }>>([])
const logsEl = ref<HTMLElement | null>(null)

// ---- Device connection ----
const adbPath = ref('adb')
const address = ref('127.0.0.1:5555')
const config = ref('General')
const connecting = ref(false)

// ---- Task queue (MAA 风格：任务列表) ----
interface TaskItem {
  id: number
  type: string
  label: string
  params: Record<string, unknown>
  enabled: boolean
}
const tasks = ref<TaskItem[]>([])
let nextTaskId = 1

// 当前编辑的任务
const selectedTaskId = ref<number | null>(null)

// ---- UI state ----
const showTaskList = ref(true) // 移动端面板切换
const activePanel = ref<'tasks' | 'device' | 'logs'>('tasks')

// 当前选中的任务类型（添加新任务时）
const newTaskType = ref('Fight')

// ==================== Computed ====================
const selectedTask = computed(() => {
  return tasks.value.find((t) => t.id === selectedTaskId.value) || null
})

const selectedTaskTypeDef = computed<TaskType | undefined>(() => {
  if (!selectedTask.value) return undefined
  return getTaskType(selectedTask.value.type)
})

const selectedParams = computed({
  get: () => selectedTask.value?.params || {},
  set: (v: Record<string, unknown>) => {
    if (selectedTask.value) selectedTask.value.params = v
  },
})

// ==================== Helpers ====================
function ts() {
  return new Date().toLocaleTimeString()
}

function appendLog(msg: string, detail: string) {
  logs.value.push({ msg, detail, time: ts() })
  if (logs.value.length > 500) logs.value.shift()
  setTimeout(() => {
    if (logsEl.value) logsEl.value.scrollTop = logsEl.value.scrollHeight
  }, 50)
}

async function api(path: string, method = 'GET', body?: unknown) {
  const res = await fetch(path, {
    method,
    headers: { 'Content-Type': 'application/json' },
    body: body ? JSON.stringify(body) : undefined,
  })
  const data = await res.json()
  if (!data.ok) throw new Error(data.error || '请求失败')
  return data.data
}

// ==================== Status ====================
async function refreshStatus() {
  try {
    const status = await api('/api/status')
    healthy.value = status.healthy !== false
    connected.value = !!status.connected
    running.value = !!status.running
  } catch {
    /* server not reachable */
  }
}

// ==================== WebSocket ====================
let ws: WebSocket | null = null
let statusTimer: number | null = null
let reconnectTimer: number | null = null

function connectWs() {
  const proto = location.protocol === 'https:' ? 'wss:' : 'ws:'
  ws = new WebSocket(`${proto}//${location.host}/api/ws`)
  ws.onopen = () => appendLog('system', 'WebSocket 已连接')
  ws.onmessage = (e) => {
    try {
      const data = JSON.parse(e.data)
      if (data.type === 'hello') {
        version.value = data.version || ''
      } else if (data.type === 'event') {
        const detail = typeof data.details === 'string' ? data.details : JSON.stringify(data.details)
        appendLog(data.msg, detail)
        if (data.msg === 'AllTasksCompleted') running.value = false
        if (data.msg === 'TaskChainStart') running.value = true
        refreshStatus()
      }
    } catch {
      /* ignore */
    }
  }
  ws.onclose = () => {
    appendLog('system', 'WebSocket 已断开，重连中…')
    reconnectTimer = window.setTimeout(connectWs, 3000)
  }
}

// ==================== Device Actions ====================
async function doConnect() {
  connecting.value = true
  try {
    await api('/api/connect', 'POST', {
      adb_path: adbPath.value,
      address: address.value,
      config: config.value,
    })
    appendLog('connect', `正在连接 ${address.value}…`)
    setTimeout(refreshStatus, 1500)
  } catch (e) {
    appendLog('error', String(e))
  } finally {
    connecting.value = false
  }
}

// ==================== Task Actions ====================
function addTaskByType(type: string) {
  const def = getTaskType(type)
  if (!def) return
  const item: TaskItem = {
    id: nextTaskId++,
    type,
    label: def.label,
    params: JSON.parse(JSON.stringify(def.defaultParams)),
    enabled: true,
  }
  tasks.value.push(item)
  selectedTaskId.value = item.id
  appendLog('add-task', `已添加 ${def.label} (${type})`)
}

// 添加任务到 MAA（提交到后端）
async function submitTasks() {
  if (!tasks.value.length) {
    appendLog('error', '任务列表为空')
    return
  }
  try {
    // 清除旧的（后端每次 append 前需 reset；这里简单实现：逐个添加）
    for (const task of tasks.value) {
      if (!task.enabled) continue
      await api('/api/task', 'POST', {
        task_type: task.type,
        params: task.params,
      })
    }
    appendLog('system', `已提交 ${tasks.value.filter((t) => t.enabled).length} 个任务`)
  } catch (e) {
    appendLog('error', String(e))
  }
}

async function doStart() {
  try {
    // 防御性：先清空旧任务记录，避免重复叠加
    await api('/api/tasks/clear', 'POST')
    await submitTasks()
    await api('/api/start', 'POST')
    running.value = true
    appendLog('start', '任务已启动')
  } catch (e) {
    appendLog('error', String(e))
  }
}

async function doStop() {
  try {
    await api('/api/stop', 'POST')
    running.value = false
    appendLog('stop', '任务已停止')
  } catch (e) {
    appendLog('error', String(e))
  }
}

function toggleTaskEnabled(task: TaskItem) {
  task.enabled = !task.enabled
}

function removeTask(id: number) {
  tasks.value = tasks.value.filter((t) => t.id !== id)
  if (selectedTaskId.value === id) selectedTaskId.value = null
}

function clearTasks() {
  tasks.value = []
  selectedTaskId.value = null
}

// 一键长草（快速添加默认任务组合）
function quickAddAll() {
  tasks.value = []
  nextTaskId = 1
  const combos = [
    'StartUp', 'Fight', 'Infrast', 'Recruit', 'Mall', 'Award',
  ]
  for (const type of combos) {
    const def = getTaskType(type)
    if (def) {
      tasks.value.push({
        id: nextTaskId++,
        type,
        label: def.label,
        params: JSON.parse(JSON.stringify(def.defaultParams)),
        enabled: true,
      })
    }
  }
  appendLog('system', '已添加一键长草组合')
}

// ==================== Lifecycle ====================
onMounted(() => {
  connectWs()
  refreshStatus()
  statusTimer = window.setInterval(refreshStatus, 5000)
})

onUnmounted(() => {
  ws?.close()
  if (statusTimer) clearInterval(statusTimer)
  if (reconnectTimer) clearTimeout(reconnectTimer)
})
</script>

<template>
  <div class="app-shell">
    <!-- ===== 顶部导航栏 ===== -->
    <header class="topbar">
      <div class="logo">
        <span class="logo-icon">⚡</span>
        <h1>MaaWeb</h1>
        <span v-if="version" class="version-badge">MaaCore {{ version }}</span>
      </div>
      <div class="topbar-status">
        <span class="status-chip" :class="healthy ? 'ok' : 'bad'">
          <span class="dot"></span>{{ healthy ? '核心' : '未加载' }}
        </span>
        <span class="status-chip" :class="connected ? 'ok' : 'bad'">
          <span class="dot"></span>{{ connected ? '已连接' : '未连接' }}
        </span>
        <span class="status-chip" :class="running ? 'running' : 'idle'">
          <span class="dot"></span>{{ running ? '运行中' : '空闲' }}
        </span>
        <button class="btn btn-small" @click="refreshStatus">刷新</button>
      </div>
    </header>

    <!-- ===== 主体 ===== -->
    <main class="main-area">
      <!-- 左侧栏 -->
      <aside class="sidebar">
        <div class="sidebar-section">
          <div class="sidebar-title">任务</div>
          <button class="nav-item" :class="{ active: activePanel === 'tasks' }" @click="activePanel = 'tasks'">
            📋 任务列表 <span class="badge-count">{{ tasks.length }}</span>
          </button>
          <button class="nav-item" :class="{ active: activePanel === 'device' }" @click="activePanel = 'device'">
            📱 设备连接
          </button>
          <button class="nav-item" :class="{ active: activePanel === 'logs' }" @click="activePanel = 'logs'">
            📜 实时日志
          </button>
        </div>
        <div class="sidebar-section">
          <div class="sidebar-title">快速组合</div>
          <button class="nav-item quick-add" @click="quickAddAll">⚡ 一键长草</button>
          <button class="nav-item quick-add" @click="addTaskByType('Fight')">⚔️ 刷理智</button>
          <button class="nav-item quick-add" @click="addTaskByType('Roguelike')">🏆 刷肉鸽</button>
        </div>
      </aside>

      <!-- 主内容区 -->
      <section class="content">
        <!-- ===== 设备连接面板 ===== -->
        <div v-show="activePanel === 'device'" class="panel">
          <div class="panel-header">
            <h2>设备连接</h2>
          </div>
          <div class="card">
            <div class="field">
              <label>ADB 路径</label>
              <input v-model="adbPath" class="input" placeholder="adb 或 /usr/bin/adb" />
            </div>
            <div class="field">
              <label>设备地址</label>
              <input v-model="address" class="input" placeholder="192.168.1.100:5555" />
            </div>
            <div class="field">
              <label>连接配置</label>
              <select v-model="config" class="input">
                <option v-for="c in CONNECT_CONFIGS" :key="c.value" :value="c.value">{{ c.label }}</option>
              </select>
            </div>
            <button class="btn" :disabled="connecting" @click="doConnect">
              {{ connecting ? '连接中…' : '连接设备' }}
            </button>
          </div>
        </div>

        <!-- ===== 任务面板 ===== -->
        <div v-show="activePanel === 'tasks'" class="panel">
          <div class="panel-header">
            <h2>任务配置</h2>
            <div class="panel-actions">
              <button class="btn btn-small" @click="clearTasks">清空</button>
              <button class="btn btn-success btn-small" @click="doStart" :disabled="running">开始</button>
              <button class="btn btn-danger btn-small" @click="doStop" :disabled="!running">停止</button>
            </div>
          </div>

          <!-- 添加任务行 -->
          <div class="add-task-row card">
            <select v-model="newTaskType" class="input">
              <option v-for="t in TASK_TYPES" :key="t.type" :value="t.type">
                {{ t.icon }} {{ t.label }}
              </option>
            </select>
            <button class="btn" @click="addTaskByType(newTaskType)">添加任务</button>
          </div>

          <div class="task-layout">
            <!-- 任务列表（左） -->
            <div class="task-list">
              <div
                v-for="task in tasks"
                :key="task.id"
                class="task-item"
                :class="{ selected: task.id === selectedTaskId }"
                @click="selectedTaskId = task.id"
              >
                <label class="task-toggle" @click.stop>
                  <input type="checkbox" :checked="task.enabled" @change="toggleTaskEnabled(task)" />
                  <span>{{ task.enabled ? '✅' : '⬜' }}</span>
                </label>
                <span class="task-type-icon">{{ getTaskType(task.type)?.icon }}</span>
                <span class="task-name">{{ task.label }}</span>
                <button class="task-delete" @click.stop="removeTask(task.id)">✕</button>
              </div>
              <div v-if="!tasks.length" class="task-empty">
                任务列表为空，请添加任务
              </div>
            </div>

            <!-- 任务参数编辑（右） -->
            <div class="task-editor">
              <template v-if="selectedTask && selectedTaskTypeDef">
                <div class="editor-title">
                  {{ selectedTaskTypeDef.icon }} {{ selectedTask.label }}
                  <span class="editor-type">{{ selectedTask.type }}</span>
                </div>
                <TaskForm
                  :fields="selectedTaskTypeDef.fields"
                  v-model="selectedParams"
                />
              </template>
              <div v-else class="editor-empty">
                选择左侧任务以编辑参数
              </div>
            </div>
          </div>
        </div>

        <!-- ===== 日志面板 ===== -->
        <div v-show="activePanel === 'logs'" class="panel">
          <div class="panel-header">
            <h2>实时日志</h2>
          </div>
          <div class="log-panel" ref="logsEl">
            <div v-for="(line, i) in logs" :key="i" class="log-line">
              <span class="log-time">{{ line.time }}</span>
              <span class="log-msg">{{ line.msg }}</span>
              <span class="log-detail">{{ line.detail }}</span>
            </div>
            <div v-if="!logs.length" class="log-empty">等待事件…</div>
          </div>
        </div>
      </section>
    </main>
  </div>
</template>

<style scoped>
/* ===== 布局 ===== */
.app-shell {
  min-height: 100vh;
  display: flex;
  flex-direction: column;
}

.topbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 20px;
  background: var(--card);
  border-bottom: 1px solid var(--border);
  position: sticky;
  top: 0;
  z-index: 100;
}

.logo {
  display: flex;
  align-items: center;
  gap: 8px;
}

.logo h1 {
  font-size: 18px;
}

.logo-icon {
  font-size: 22px;
}

.version-badge {
  font-size: 11px;
  background: var(--border);
  padding: 2px 8px;
  border-radius: 10px;
  color: var(--text-secondary);
}

.topbar-status {
  display: flex;
  align-items: center;
  gap: 10px;
}

.status-chip {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  padding: 4px 10px;
  border-radius: 12px;
  background: var(--bg);
}

.status-chip .dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
}

.status-chip.ok .dot { background: var(--success); }
.status-chip.bad .dot { background: var(--danger); }
.status-chip.running .dot {
  background: var(--warning);
  animation: pulse 1s infinite;
}
.status-chip.idle .dot { background: var(--border); }

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}

.main-area {
  display: flex;
  flex: 1;
}

/* ===== 侧边栏 ===== */
.sidebar {
  width: 200px;
  background: var(--card);
  border-right: 1px solid var(--border);
  padding: 16px 10px;
  flex-shrink: 0;
}

.sidebar-section {
  margin-bottom: 20px;
}

.sidebar-title {
  font-size: 11px;
  text-transform: uppercase;
  color: var(--text-secondary);
  margin-bottom: 8px;
  padding: 0 8px;
  letter-spacing: 0.5px;
}

.nav-item {
  display: flex;
  align-items: center;
  width: 100%;
  padding: 8px 10px;
  margin-bottom: 2px;
  border: none;
  background: transparent;
  border-radius: 6px;
  cursor: pointer;
  font-size: 13px;
  color: var(--text);
  text-align: left;
  gap: 6px;
  transition: background 0.15s;
}

.nav-item:hover {
  background: var(--bg);
}

.nav-item.active {
  background: var(--primary);
  color: #fff;
}

.nav-item.quick-add {
  color: var(--primary);
  font-weight: 500;
}

.nav-item.quick-add:hover {
  background: var(--bg);
}

.badge-count {
  margin-left: auto;
  background: var(--primary);
  color: #fff;
  border-radius: 10px;
  padding: 1px 7px;
  font-size: 11px;
}

/* ===== 内容区 ===== */
.content {
  flex: 1;
  padding: 20px;
  min-width: 0;
}

.panel {
  animation: fadeIn 0.2s;
}

@keyframes fadeIn {
  from { opacity: 0; transform: translateY(4px); }
  to { opacity: 1; transform: translateY(0); }
}

.panel-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}

.panel-header h2 {
  font-size: 16px;
}

.panel-actions {
  display: flex;
  gap: 8px;
}

/* ===== 任务区 ===== */
.add-task-row {
  display: flex;
  gap: 10px;
  margin-bottom: 16px;
  align-items: center;
}

.task-layout {
  display: grid;
  grid-template-columns: 280px 1fr;
  gap: 16px;
}

.task-list {
  background: var(--card);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 8px;
  max-height: 500px;
  overflow-y: auto;
}

.task-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  border-radius: 6px;
  cursor: pointer;
  transition: background 0.15s;
}

.task-item:hover {
  background: var(--bg);
}

.task-item.selected {
  background: var(--bg);
  border: 1px solid var(--primary);
}

.task-toggle {
  cursor: pointer;
  display: flex;
  align-items: center;
}

.task-toggle input {
  display: none;
}

.task-name {
  flex: 1;
  font-size: 13px;
}

.task-type-icon {
  font-size: 16px;
}

.task-delete {
  border: none;
  background: transparent;
  color: var(--text-secondary);
  cursor: pointer;
  font-size: 12px;
  padding: 2px 4px;
}

.task-delete:hover {
  color: var(--danger);
}

.task-empty {
  padding: 30px;
  text-align: center;
  color: var(--text-secondary);
  font-size: 13px;
}

.task-editor {
  background: var(--card);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 20px;
  max-height: 500px;
  overflow-y: auto;
}

.editor-title {
  font-size: 15px;
  font-weight: 600;
  margin-bottom: 16px;
  display: flex;
  align-items: center;
  gap: 8px;
}

.editor-type {
  font-size: 11px;
  background: var(--border);
  padding: 2px 8px;
  border-radius: 8px;
  color: var(--text-secondary);
}

.editor-empty {
  padding: 40px;
  text-align: center;
  color: var(--text-secondary);
  font-size: 13px;
}

/* ===== 日志 ===== */
.log-panel {
  background: #1e293b;
  color: #e2e8f0;
  border-radius: 8px;
  padding: 12px;
  font-family: 'JetBrains Mono', 'Fira Code', Consolas, monospace;
  font-size: 12px;
  line-height: 1.7;
  max-height: 500px;
  overflow-y: auto;
  white-space: pre-wrap;
  word-break: break-all;
}

.log-line {
  display: flex;
  gap: 8px;
}

.log-time {
  color: #64748b;
  flex-shrink: 0;
}

.log-msg {
  color: #94a3b8;
  min-width: 130px;
  flex-shrink: 0;
}

.log-detail {
  color: #cbd5e1;
}

.log-empty {
  color: #64748b;
}

/* ===== 通用按钮 ===== */
.btn-small {
  padding: 5px 12px;
  font-size: 12px;
}

/* ===== 自适应 ===== */
@media (max-width: 768px) {
  .main-area {
    flex-direction: column;
  }

  .sidebar {
    width: 100%;
    border-right: none;
    border-bottom: 1px solid var(--border);
    padding: 10px;
    display: flex;
    gap: 8px;
    overflow-x: auto;
  }

  .sidebar-section {
    margin-bottom: 0;
    display: flex;
    gap: 6px;
    align-items: center;
  }

  .sidebar-title {
    display: none;
  }

  .nav-item {
    width: auto;
    white-space: nowrap;
    padding: 6px 10px;
    font-size: 12px;
  }

  .badge-count {
    display: none;
  }

  .content {
    padding: 12px;
  }

  .task-layout {
    grid-template-columns: 1fr;
  }

  .task-list {
    max-height: 200px;
  }

  .topbar-status .status-chip {
    display: none;
  }

  .topbar-status .status-chip:first-child {
    display: inline-flex;
  }

  .panel-header {
    flex-wrap: wrap;
    gap: 8px;
  }
}

@media (max-width: 480px) {
  .topbar {
    padding: 10px 12px;
  }

  .version-badge {
    display: none;
  }

  .panel-actions {
    width: 100%;
  }

  .panel-actions .btn {
    flex: 1;
  }

  .add-task-row {
    flex-direction: column;
  }

  .add-task-row .input,
  .add-task-row .btn {
    width: 100%;
  }
}
</style>
