/** 全局状态与业务流程编排（与原有 Rust 端逻辑一一对应） */

import { ElMessage } from 'element-plus'
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

export async function deleteCollection(id: number) {
  try {
    await api.collectionDelete(id)
    state.currentCollectionId = null
    await refreshCollections()
    toast('已删除集合')
  } catch (e) {
    toast(`删除失败：${String(e)}`, { type: 'error' })
  }
}

export async function saveCurrentRequest(name: string) {
  const target = state.config.url.trim() || '未命名请求'
  const finalName = name.trim() || target
  if (state.currentCollectionId == null) {
    toast('请先选择一个集合', { type: 'warning' })
    return false
  }
  try {
    await api.savedSave(state.currentCollectionId, finalName, clone(state.config))
    await refreshSaved()
    toast(`已保存「${finalName}」`)
    return true
  } catch (e) {
    toast(`保存失败：${String(e)}`, { type: 'error' })
    return false
  }
}

/** 打开「保存到集合」对话框（Ctrl+S 复用同一入口） */
export function openSaveDialog() {
  if (state.currentCollectionId == null) {
    state.sideTab = 'collections'
    toast('请先在左侧选择一个集合', { type: 'warning' })
    return
  }
  state.saveName = state.config.url || '未命名请求'
  state.saveDialogOpen = true
}

/** 提交保存对话框 */
export async function submitSaveDialog() {
  const ok = await saveCurrentRequest(state.saveName)
  if (ok) {
    state.saveDialogOpen = false
    state.saveName = ''
  }
}

export async function deleteSavedRequest(id: number) {
  try {
    await api.savedDelete(id)
    await refreshSaved()
  } catch (e) {
    toast(`删除失败：${String(e)}`, { type: 'error' })
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
  try {
    await api.historyClear()
    state.history = []
    toast('已清空历史')
  } catch (e) {
    toast(`清空失败：${String(e)}`, { type: 'error' })
  }
}

export async function deleteHistory(id: number) {
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
  try {
    const text = await api.curlExport(clone(state.config), state.envMap)
    await copyText(text)
    toast('已复制 cURL 命令')
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
