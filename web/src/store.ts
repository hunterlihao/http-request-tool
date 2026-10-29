/** 全局状态与业务流程编排（与原有 Rust 端逻辑一一对应） */

import { ElMessage, ElMessageBox } from 'element-plus'
import { reactive, watch } from 'vue'

import { api } from '@/api'
import {
  emptyConfig,
  type Collection,
  type EnvVar,
  type HistoryItem,
  type RequestConfig,
  type ResponseView,
  type SavedRequest,
} from '@/api/types'

export interface ToastOptions {
  type?: 'success' | 'error' | 'info' | 'warning'
  duration?: number
}

export function toast(message: string, options: ToastOptions = {}) {
  ElMessage({
    message,
    type: options.type ?? 'success',
    duration: options.duration ?? 2000,
    showClose: false,
  })
}

export const state = reactive({
  config: emptyConfig() as RequestConfig,
  response: null as ResponseView | null,
  sending: false,

  envNames: ['global'] as string[],
  currentEnv: 'global',
  envVars: [] as EnvVar[],
  /** 与 envVars 一一对应的已落库键名，用于改键时删除旧记录 */
  envOriginals: [] as string[],
  envMap: {} as Record<string, string>,

  collections: [] as Collection[],
  currentCollectionId: null as number | null,
  savedRequests: [] as SavedRequest[],
  /** 当前编辑内容来自哪个已保存请求（用于覆盖保存 / 改名） */
  currentSavedId: null as number | null,
  collectionSearch: '',

  history: [] as HistoryItem[],
  historySearch: '',

  undefinedVars: [] as string[],

  requestTab: 'params',
  sideTab: 'history',
  responseTab: 'body',
  rawView: false,

  saveDialogOpen: false,
  saveName: '',

  sidebarVisible: true,
})

function clone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T
}

// ---------- 初始化 ----------

export async function init() {
  try {
    const [names, lastConfig] = await Promise.all([api.envNames(), api.getLastConfig()])
    state.envNames = names.length ? names : ['global']
    state.currentEnv = state.envNames.includes('global') ? 'global' : state.envNames[0]
    await loadEnv(state.currentEnv)
    if (lastConfig) {
      state.config = normalizeConfig(lastConfig)
    }
  } catch (e) {
    toast(`初始化失败：${String(e)}`, { type: 'error', duration: 3500 })
  }
  await Promise.all([refreshCollections(), refreshHistory()])
  // 初始化完成后才开始跟踪草稿变更
  draftReady = true
}

/** 兼容旧数据：补齐缺失字段 */
export function normalizeConfig(cfg: Partial<RequestConfig>): RequestConfig {
  const base = emptyConfig()
  return {
    ...base,
    ...cfg,
    params: cfg.params ?? [],
    headers: cfg.headers ?? [],
    form_fields: cfg.form_fields ?? [],
    file_paths: cfg.file_paths ?? [],
    file_field: cfg.file_field || base.file_field,
    auth: { ...base.auth, ...(cfg.auth ?? {}) },
  }
}

// ---------- 环境变量 ----------

export async function loadEnv(name: string) {
  try {
    state.currentEnv = name
    const list = await api.envVars(name)
    state.envVars = list
    state.envOriginals = list.map((v) => v.key)
    rebuildEnvMap()
  } catch (e) {
    toast(`加载环境变量失败：${String(e)}`, { type: 'error' })
  }
}

export function rebuildEnvMap() {
  const map: Record<string, string> = {}
  for (const v of state.envVars) {
    const key = v.key.trim()
    if (key) map[key] = v.value
  }
  state.envMap = map
}

/** 编辑环境变量：写入后在变更处调用持久化 */
export async function persistEnvVar(index: number) {
  const item = state.envVars[index]
  if (!item) return
  const name = item.key.trim()
  if (!name) return
  const previous = state.envOriginals[index] ?? ''
  try {
    // 改名时先删掉旧键，否则库里会同时留下新旧两条
    if (previous && previous !== name) {
      await api.envDelete(state.currentEnv, previous)
    }
    await api.envSet(state.currentEnv, name, item.value)
    item.key = name
    state.envOriginals[index] = name
    rebuildEnvMap()
  } catch (e) {
    toast(`保存变量失败：${String(e)}`, { type: 'error' })
  }
}

export async function removeEnvVar(index: number) {
  const item = state.envVars[index]
  if (!item) return
  state.envVars.splice(index, 1)
  state.envOriginals.splice(index, 1)
  rebuildEnvMap()
  if (item.key.trim()) {
    try {
      await api.envDelete(state.currentEnv, item.key.trim())
    } catch (e) {
      toast(`删除变量失败：${String(e)}`, { type: 'error' })
    }
  }
}

export async function createEnv(name: string) {
  const trimmed = name.trim()
  if (!trimmed) {
    toast('环境名不能为空', { type: 'warning' })
    return
  }
  if (state.envNames.includes(trimmed)) {
    toast('环境已存在', { type: 'warning' })
    return
  }
  state.envNames = [...state.envNames, trimmed].sort()
  await loadEnv(trimmed)
  toast(`已创建环境「${trimmed}」`)
}

export async function dropCurrentEnv() {
  if (state.currentEnv === 'global') {
    toast('global 环境不可删除', { type: 'warning' })
    return
  }
  const name = state.currentEnv
  try {
    await api.envDrop(name)
    state.envNames = state.envNames.filter((n) => n !== name)
    await loadEnv('global')
    toast(`已删除环境「${name}」`)
  } catch (e) {
    toast(`删除失败：${String(e)}`, { type: 'error' })
  }
}

// ---------- 集合 ----------

export async function refreshCollections(preferId?: number) {
  try {
    state.collections = await api.collectionsList()
    const target =
      preferId ?? state.currentCollectionId ?? state.collections[0]?.id ?? null
    state.currentCollectionId = state.collections.some((c) => c.id === target)
      ? target
      : (state.collections[0]?.id ?? null)
    await refreshSaved()
  } catch (e) {
    toast(`加载集合失败：${String(e)}`, { type: 'error' })
  }
}

export async function refreshSaved() {
  if (state.currentCollectionId == null) {
    state.savedRequests = []
    return
  }
  try {
    state.savedRequests = await api.savedList(state.currentCollectionId)
  } catch (e) {
    toast(`加载请求失败：${String(e)}`, { type: 'error' })
  }
}

export async function selectCollection(id: number) {
  state.currentCollectionId = id
  await refreshSaved()
}

export async function createCollection(name: string) {
  const trimmed = name.trim()
  if (!trimmed) {
    toast('集合名称不能为空', { type: 'warning' })
    return
  }
  try {
    const id = await api.collectionCreate(trimmed)
    await refreshCollections(id)
    toast(`已创建集合「${trimmed}」`)
  } catch (e) {
    toast(`创建失败：${String(e)}`, { type: 'error' })
  }
}

/** 统一的二次确认：避免误点删除不可恢复的数据 */
async function confirmDanger(message: string, title = '确认删除'): Promise<boolean> {
  try {
    await ElMessageBox.confirm(message, title, {
      confirmButtonText: '删除',
      cancelButtonText: '取消',
      type: 'warning',
      confirmButtonClass: 'el-button--danger',
    })
    return true
  } catch {
    return false
  }
}

export async function deleteCollection(id: number) {
  const col = state.collections.find((c) => c.id === id)
  if (!(await confirmDanger(`删除集合「${col?.name ?? id}」及其中的所有请求？`))) return
  try {
    await api.collectionDelete(id)
    state.currentCollectionId = null
    state.currentSavedId = null
    await refreshCollections()
    toast('已删除集合')
  } catch (e) {
    toast(`删除失败：${String(e)}`, { type: 'error' })
  }
}

export async function saveCurrentRequest(name: string, overwrite = false) {
  const target = state.config.url.trim() || '未命名请求'
  const finalName = name.trim() || target
  if (state.currentCollectionId == null) {
    toast('请先选择一个集合', { type: 'warning' })
    return false
  }
  try {
    if (overwrite && state.currentSavedId != null) {
      await api.savedUpdate(state.currentSavedId, finalName, clone(state.config))
      await refreshSaved()
      toast(`已覆盖「${finalName}」`)
      return true
    }
    const id = await api.savedSave(state.currentCollectionId, finalName, clone(state.config))
    state.currentSavedId = id
    await refreshSaved()
    toast(`已保存「${finalName}」`)
    return true
  } catch (e) {
    toast(`保存失败：${String(e)}`, { type: 'error' })
    return false
  }
}

/** 打开集合里的已保存请求（记住来源，便于覆盖保存） */
export function openSavedRequest(req: SavedRequest) {
  applyConfig(req.config)
  state.currentSavedId = req.id
}

/** 给已保存请求改名 */
export async function renameSavedRequest(id: number, name: string) {
  const finalName = name.trim()
  const item = state.savedRequests.find((r) => r.id === id)
  if (!item || !finalName) return
  try {
    await api.savedUpdate(id, finalName, item.config)
    await refreshSaved()
    toast('已重命名')
  } catch (e) {
    toast(`重命名失败：${String(e)}`, { type: 'error' })
  }
}

/** 拖拽排序：先改本地顺序，再落库 */
export async function moveSavedRequest(from: number, to: number) {
  if (from === to || from < 0 || to < 0) return
  const list = [...state.savedRequests]
  const [moved] = list.splice(from, 1)
  if (!moved) return
  list.splice(to, 0, moved)
  state.savedRequests = list
  try {
    await api.savedReorder(list.map((r) => r.id))
  } catch (e) {
    toast(`保存排序失败：${String(e)}`, { type: 'error' })
  }
}

/** 打开「保存到集合」对话框（Ctrl+S 复用同一入口） */
export function openSaveDialog() {
  if (state.currentCollectionId == null) {
    state.sideTab = 'collections'
    toast('请先在左侧选择一个集合', { type: 'warning' })
    return
  }
  // 已在编辑某条已保存请求时，默认沿用它的名字（覆盖保存）
  const saved = state.savedRequests.find((r) => r.id === state.currentSavedId)
  state.saveName = saved?.name ?? (state.config.url || '未命名请求')
  state.saveDialogOpen = true
}

/** 提交保存对话框 */
export async function submitSaveDialog(overwrite = false) {
  const ok = await saveCurrentRequest(state.saveName, overwrite)
  if (ok) {
    state.saveDialogOpen = false
    state.saveName = ''
  }
}

export async function deleteSavedRequest(id: number) {
  const item = state.savedRequests.find((r) => r.id === id)
  if (!(await confirmDanger(`删除已保存的请求「${item?.name ?? id}」？`))) return
  try {
    await api.savedDelete(id)
    if (state.currentSavedId === id) state.currentSavedId = null
    await refreshSaved()
  } catch (e) {
    toast(`删除失败：${String(e)}`, { type: 'error' })
  }
}

/** 复制一份已保存请求（便于派生不同环境的版本） */
export async function duplicateSavedRequest(id: number) {
  const item = state.savedRequests.find((r) => r.id === id)
  if (!item || state.currentCollectionId == null) return
  try {
    await api.savedSave(state.currentCollectionId, `${item.name} 副本`, clone(item.config))
    await refreshSaved()
    toast('已创建副本')
  } catch (e) {
    toast(`复制失败：${String(e)}`, { type: 'error' })
  }
}

export async function exportCollection() {
  const id = state.currentCollectionId
  const col = state.collections.find((c) => c.id === id)
  if (id == null || !col) {
    toast('请先选择集合', { type: 'warning' })
    return
  }
  try {
    const path = await api.collectionExportFile(id, col.name)
    if (path) toast(`已导出到 ${path}`)
  } catch (e) {
    toast(`导出失败：${String(e)}`, { type: 'error' })
  }
}

export async function importCollection() {
  try {
    const id = await api.collectionImportFile()
    if (id != null) {
      await refreshCollections(id)
      toast('导入成功')
    }
  } catch (e) {
    toast(`导入失败：${String(e)}`, { type: 'error' })
  }
}

// ---------- 历史 ----------

export async function refreshHistory() {
  try {
    state.history = await api.historyList(500)
  } catch (e) {
    toast(`加载历史失败：${String(e)}`, { type: 'error' })
  }
}

export async function clearHistory() {
  if (!(await confirmDanger('清空全部历史记录？此操作不可恢复', '清空历史'))) return
  try {
    await api.historyClear()
    state.history = []
    toast('已清空历史')
  } catch (e) {
    toast(`清空失败：${String(e)}`, { type: 'error' })
  }
}

export async function deleteHistory(id: number) {
  if (!(await confirmDanger('删除这条历史记录？'))) return
  try {
    await api.historyDelete(id)
    await refreshHistory()
  } catch (e) {
    toast(`删除失败：${String(e)}`, { type: 'error' })
  }
}

// ---------- 请求 ----------

export function applyConfig(cfg: RequestConfig) {
  state.config = normalizeConfig(clone(cfg))
  state.response = null
  // 从历史 / cURL 来的内容不再对应某条已保存请求
  state.currentSavedId = null
}

export async function sendRequest() {
  if (state.sending) return
  state.sending = true
  try {
    const resp = await api.sendRequest(clone(state.config), state.envMap)
    if (resp.error === '请求已取消') {
      toast('已取消本次请求', { type: 'info' })
      return
    }
    state.response = resp
    state.responseTab = 'body'
    await refreshHistory()
  } catch (e) {
    toast(`请求失败：${String(e)}`, { type: 'error', duration: 3500 })
  } finally {
    state.sending = false
  }
}

/** 中止正在进行的请求 */
export async function cancelRequest() {
  try {
    await api.cancelRequest()
  } catch (e) {
    toast(`取消失败：${String(e)}`, { type: 'error' })
  }
}

export async function copyCurl() {
  await copyAsCode('curl')
}

// ---------- 复制为代码 ----------

export type CodeTarget = 'curl' | 'fetch' | 'axios'

/** 用当前环境变量替换文本中的 {{var}}（内置 $ 变量交给后端处理） */
function resolveVars(text: string): string {
  return text.replace(/\{\{\s*([\w.$-]+)\s*\}\}/g, (all, name: string) =>
    Object.prototype.hasOwnProperty.call(state.envMap, name) ? state.envMap[name] : all,
  )
}

/** 拼出最终请求 URL（含启用中的查询参数） */
function resolvedUrl(): string {
  const url = resolveVars(state.config.url.trim())
  try {
    const parsed = new URL(url)
    for (const p of state.config.params.filter((p) => p.enabled && p.key.trim())) {
      parsed.searchParams.append(resolveVars(p.key), resolveVars(p.value))
    }
    return parsed.toString()
  } catch {
    return url
  }
}

/** 最终请求头（含认证注入与自动 Content-Type） */
function resolvedHeaders(): Record<string, string> {
  const headers: Record<string, string> = {}
  for (const h of state.config.headers.filter((h) => h.enabled && h.key.trim())) {
    headers[resolveVars(h.key.trim())] = resolveVars(h.value)
  }
  const auth = state.config.auth
  if (auth.auth_type === 'Basic') {
    headers['Authorization'] = `Basic ${btoa(`${resolveVars(auth.username)}:${resolveVars(auth.password)}`)}`
  } else if (auth.auth_type === 'Bearer' && auth.token) {
    headers['Authorization'] = `Bearer ${resolveVars(auth.token)}`
  } else if (auth.auth_type === 'CustomHeader' && auth.header_name.trim()) {
    headers[resolveVars(auth.header_name.trim())] = resolveVars(auth.header_value)
  }
  const hasContentType = Object.keys(headers).some((k) => k.toLowerCase() === 'content-type')
  if (!hasContentType) {
    const byType: Record<string, string> = {
      Json: 'application/json',
      Text: 'text/plain',
      Form: 'application/x-www-form-urlencoded',
    }
    const ct = byType[state.config.body_type]
    if (ct) headers['Content-Type'] = ct
  }
  return headers
}

/** 最终请求体（multipart 只提示，不展开） */
function resolvedBody(): string | null {
  const cfg = state.config
  if (cfg.body_type === 'Json' || cfg.body_type === 'Text') {
    return cfg.body.trim() ? resolveVars(cfg.body) : null
  }
  if (cfg.body_type === 'Form') {
    const fields = cfg.form_fields.filter((f) => f.enabled && f.key.trim())
    if (!fields.length) return null
    const params = new URLSearchParams()
    for (const f of fields) params.append(resolveVars(f.key), resolveVars(f.value))
    return params.toString()
  }
  return null
}

function jsString(value: string): string {
  return JSON.stringify(value)
}

/** 按目标语言生成代码片段 */
function buildCode(target: Exclude<CodeTarget, 'curl'>): string {
  const method = state.config.method.toUpperCase()
  const url = resolvedUrl()
  const headers = resolvedHeaders()
  const body = resolvedBody()

  if (target === 'fetch') {
    const lines = [`const res = await fetch(${jsString(url)}, {`, `  method: ${jsString(method)},`]
    if (Object.keys(headers).length) {
      lines.push('  headers: {')
      for (const [k, v] of Object.entries(headers)) lines.push(`    ${jsString(k)}: ${jsString(v)},`)
      lines.push('  },')
    }
    if (body != null) lines.push(`  body: ${jsString(body)},`)
    lines.push('})', 'if (!res.ok) throw new Error(`HTTP ${res.status}`)', 'const data = await res.json()')
    return lines.join('\n')
  }

  const lines = [
    "import axios from 'axios'",
    '',
    'const { data } = await axios.request({',
    `  url: ${jsString(url)},`,
    `  method: ${jsString(method.toLowerCase())},`,
  ]
  if (Object.keys(headers).length) {
    lines.push('  headers: {')
    for (const [k, v] of Object.entries(headers)) lines.push(`    ${jsString(k)}: ${jsString(v)},`)
    lines.push('  },')
  }
  if (body != null) lines.push(`  data: ${jsString(body)},`)
  lines.push('})')
  return lines.join('\n')
}

/** 复制当前请求为可直接粘贴的代码 */
export async function copyAsCode(target: CodeTarget) {
  try {
    let code: string
    if (target === 'curl') {
      code = await api.curlExport(clone(state.config), state.envMap)
    } else {
      code = buildCode(target)
    }
    await copyText(code)
    toast(`已复制 ${target === 'curl' ? 'cURL' : target} 代码`)
  } catch (e) {
    toast(`复制失败：${String(e)}`, { type: 'error' })
  }
}

export async function importCurl(text: string) {
  try {
    const cfg = await api.curlImport(text)
    applyConfig(cfg)
    toast('已导入 cURL')
    return true
  } catch (e) {
    toast(`解析失败：${String(e)}`, { type: 'error', duration: 3500 })
    return false
  }
}

export async function copyResponseBody() {
  const resp = state.response
  if (!resp) return
  const text = state.rawView ? resp.body_text : resp.body_pretty
  await copyText(text)
  toast('已复制响应体')
}

export async function saveResponseBody() {
  try {
    const path = await api.saveResponseFile()
    if (path) toast(`已保存到 ${path}`)
  } catch (e) {
    toast(`保存失败：${String(e)}`, { type: 'error' })
  }
}

// ---------- 工具 ----------

/** 复制文本：优先 Clipboard API，回退到临时 textarea（WebView2 下更可靠） */
export async function copyText(text: string) {
  try {
    if (navigator.clipboard && window.isSecureContext) {
      await navigator.clipboard.writeText(text)
      return
    }
  } catch {
    /* 继续走回退方案 */
  }
  const el = document.createElement('textarea')
  el.value = text
  el.style.position = 'fixed'
  el.style.opacity = '0'
  document.body.appendChild(el)
  el.select()
  try {
    document.execCommand('copy')
  } finally {
    document.body.removeChild(el)
  }
}

// 未定义变量：配置或环境变化后重新计算
let varTimer: number | undefined
watch(
  () => [state.config, state.envMap],
  () => {
    window.clearTimeout(varTimer)
    varTimer = window.setTimeout(async () => {
      try {
        state.undefinedVars = await api.undefinedVars(clone(state.config), state.envMap)
      } catch {
        state.undefinedVars = []
      }
    }, 250)
  },
  { deep: true },
)

// 草稿自动保存：编辑后防抖落库，关窗也不会丢掉没发送的内容
let draftTimer: number | undefined
let draftReady = false
watch(
  () => state.config,
  () => {
    if (!draftReady) return
    window.clearTimeout(draftTimer)
    draftTimer = window.setTimeout(() => {
      api.setLastConfig(clone(state.config)).catch(() => {})
    }, 800)
  },
  { deep: true },
)
