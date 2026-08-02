<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'

// ---- State ----
const version = ref('')
const connected = ref(false)
const running = ref(false)
const logs = ref<Array<{ msg: string; detail: string; time: string }>>([])
const logsEl = ref<HTMLElement | null>(null)

// ---- Connection form ----
const adbPath = ref('adb')
const address = ref('127.0.0.1:5555')
const config = ref('General')
const connecting = ref(false)

// ---- Task form ----
const taskType = ref('Fight')
const taskParams = ref(JSON.stringify({ stage: '1-7', times: 1 }, null, 2))
const taskParamsError = ref('')
const taskIds = ref<number[]>([])

let ws: WebSocket | null = null
let statusTimer: number | null = null
let reconnectTimer: number | null = null

// ---- Helpers ----
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

// ---- Status polling ----
async function refreshStatus() {
  try {
    const status = await api('/api/status')
    connected.value = !!status.connected
    running.value = !!status.running
  } catch {
    /* server not reachable; keep last state */
  }
}

// ---- WebSocket ----
function connectWs() {
  const proto = location.protocol === 'https:' ? 'wss:' : 'ws:'
  ws = new WebSocket(`${proto}//${location.host}/api/ws`)

  ws.onopen = () => {
    appendLog('system', 'WebSocket 已连接')
  }

  ws.onmessage = (e) => {
    try {
      const data = JSON.parse(e.data)
      if (data.type === 'hello') {
        version.value = data.version || ''
      } else if (data.type === 'event') {
        const detail = typeof data.details === 'string'
          ? data.details
          : JSON.stringify(data.details)
        appendLog(data.msg, detail)
        if (data.msg === 'AllTasksCompleted') running.value = false
        if (data.msg === 'TaskChainStart') running.value = true
        refreshStatus()
      }
    } catch {
      /* ignore malformed */
    }
  }

  ws.onclose = () => {
    appendLog('system', 'WebSocket 已断开，重连中…')
    reconnectTimer = window.setTimeout(connectWs, 3000)
  }
}

// ---- Actions ----
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

function parseTaskParams(): Record<string, unknown> {
  try {
    const parsed = JSON.parse(taskParams.value)
    taskParamsError.value = ''
    return parsed
  } catch (e) {
    taskParamsError.value = `参数不是合法 JSON: ${e}`
    return {}
  }
}

async function addTask() {
  if (taskParamsError.value) return
  const params = parseTaskParams()
  if (taskParamsError.value) return
  try {
    const data = await api('/api/task', 'POST', {
      task_type: taskType.value,
      params,
    })
    taskIds.value.push(data.task_id)
    appendLog('add-task', `已添加 ${taskType.value} (id=${data.task_id})`)
  } catch (e) {
    appendLog('error', String(e))
  }
}

async function doStart() {
  try {
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

// ---- Lifecycle ----
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
  <header>
    <h1 style="font-size: 22px; margin-bottom: 4px">MaaWeb</h1>
    <p style="color: var(--text-secondary); font-size: 13px">
      明日方舟小助手 · Web 控制台
      <span v-if="version">(MaaCore {{ version }})</span>
    </p>
  </header>

  <!-- Status bar -->
  <div class="card" style="display: flex; gap: 24px; align-items: center; padding: 12px 20px">
    <span>
      <span class="status-dot" :class="connected ? 'connected' : 'disconnected'"></span>
      {{ connected ? '设备已连接' : '未连接设备' }}
    </span>
    <span>
      <span class="status-dot" :class="running ? 'running' : 'disconnected'"></span>
      {{ running ? '任务运行中' : '任务空闲' }}
    </span>
    <button class="btn" style="margin-left: auto" @click="refreshStatus">刷新</button>
  </div>

  <!-- Device connection -->
  <div class="card">
    <div class="card-title">设备连接</div>
    <div class="field">
      <label>ADB 路径</label>
      <input v-model="adbPath" class="input" placeholder="adb" />
    </div>
    <div class="field">
      <label>设备地址（局域网 ADB）</label>
      <input v-model="address" class="input" placeholder="192.168.1.100:5555" />
    </div>
    <div class="field">
      <label>连接配置</label>
      <select v-model="config" class="input">
        <option value="General">General</option>
        <option value="BlueStacks">BlueStacks</option>
        <option value="MuMuEmulator">MuMuEmulator</option>
        <option value="LDPlayer">LDPlayer</option>
        <option value="NoxPlayer">NoxPlayer</option>
        <option value="XYAZ">XYAZ</option>
        <option value="WSA">WSA</option>
        <option value="Waydroid">Waydroid</option>
      </select>
    </div>
    <button class="btn" :disabled="connecting" @click="doConnect">
      {{ connecting ? '连接中…' : '连接设备' }}
    </button>
  </div>

  <!-- Task control -->
  <div class="card">
    <div class="card-title">任务控制</div>
    <div class="field">
      <label>任务类型</label>
      <select v-model="taskType" class="input">
        <option value="Fight">Fight - 刷图</option>
        <option value="Infrast">Infrast - 基建</option>
        <option value="Recruit">Recruit - 公招</option>
        <option value="Mall">Mall - 商店</option>
        <option value="Award">Award - 领取奖励</option>
        <option value="Roguelike">Roguelike - 肉鸽</option>
        <option value="Copilot">Copilot - 自动作战</option>
      </select>
    </div>
    <div class="field">
      <label>任务参数 (JSON)</label>
      <textarea v-model="taskParams" class="input" rows="6" style="font-family: monospace; font-size: 12px" />
      <p v-if="taskParamsError" style="color: var(--danger); font-size: 12px; margin-top: 4px">{{ taskParamsError }}</p>
    </div>
    <div style="display: flex; gap: 10px">
      <button class="btn" @click="addTask">添加任务</button>
      <button class="btn btn-success" @click="doStart" :disabled="running">开始任务</button>
      <button class="btn btn-danger" @click="doStop" :disabled="!running">停止任务</button>
    </div>
    <div v-if="taskIds.length" style="margin-top: 12px; font-size: 13px; color: var(--text-secondary)">
      已添加任务 ID: {{ taskIds.join(', ') }}
    </div>
  </div>

  <!-- Logs -->
  <div class="card">
    <div class="card-title">实时日志</div>
    <div class="log-panel" ref="logsEl">
      <div v-for="(line, i) in logs" :key="i" class="log-line">
        <span style="color: #64748b">{{ line.time }}</span>
        <span class="log-msg">{{ line.msg }}</span>
        <span class="log-detail">{{ line.detail }}</span>
      </div>
      <div v-if="!logs.length" style="color: #64748b">等待事件…</div>
    </div>
  </div>
</template>
