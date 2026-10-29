/**
 * 浏览器预览用的本地 mock（仅在非 Tauri 环境下加载，生产构建不会包含）
 * 目的是让界面可以在纯前端环境独立调试，不改变真实业务逻辑。
 */

import type {
  Collection,
  EnvVar,
  HistoryItem,
  RequestConfig,
  ResponseView,
  SavedRequest,
} from './types'

interface MockDb {
  history: HistoryItem[]
  collections: Collection[]
  saved: SavedRequest[]
  envs: Record<string, EnvVar[]>
  lastName: string
}

const KEY = 'http-tool-mock-v1'

function load(): MockDb {
  const raw = localStorage.getItem(KEY)
  if (raw) {
    try {
      return JSON.parse(raw) as MockDb
    } catch {
      /* ignore */
    }
  }
  return {
    history: [],
    collections: [{ id: 1, parent_id: 0, name: '默认集合' }],
    saved: [],
    envs: { global: [{ env_name: 'global', key: 'baseUrl', value: 'https://api.example.com' }] },
    lastName: '',
  }
}

const db = load()

function persist() {
  localStorage.setItem(KEY, JSON.stringify(db))
}

let seq = Date.now()

function nextId() {
  seq += 1
  return seq
}

function replaceVars(text: string, env: Record<string, string>) {
  return text.replace(/\{\{\s*([\w.-]+)\s*\}\}/g, (all, name: string) =>
    Object.prototype.hasOwnProperty.call(env, name) ? env[name] : all,
  )
}


/**
 * 开发预览时**真的发一次请求**（浏览器受 CORS 限制，失败就退回本地 mock）。
 * 这样 `npm run dev` 里看到的响应是真数据，截图/调试都有意义；
 * 打包后的桌面版走 Rust 的 reqwest，不受 CORS 约束。
 */
async function realResponse(config: RequestConfig, env: Record<string, string>): Promise<ResponseView> {
  const url = new URL(replaceVars(config.url, env))
  for (const p of config.params) {
    if (p.enabled && p.key) url.searchParams.append(p.key, replaceVars(p.value, env))
  }
  const headers: Record<string, string> = {}
  for (const h of config.headers) {
    if (h.enabled && h.key) headers[h.key] = replaceVars(h.value, env)
  }
  const init: RequestInit = { method: config.method.toUpperCase(), headers }
  if (config.body && String(config.body_type) !== 'None') init.body = replaceVars(config.body, env)
  const t0 = performance.now()
  const r = await fetch(url.toString(), init)
  const text = await r.text()
  const dur = Math.round(performance.now() - t0)
  let pretty = text
  try {
    pretty = JSON.stringify(JSON.parse(text), null, 2)
  } catch {
    /* 不是 JSON 就原样 */
  }
  const hs: [string, string][] = []
  r.headers.forEach((v, k) => hs.push([k, v]))
  return {
    status_code: r.status,
    status_text: r.statusText,
    http_version: 'HTTP/1.1',
    headers: hs,
    request_url: url.toString(),
    body_text: text,
    body_pretty: pretty,
    body_len: text.length,
    truncated: false,
    is_binary: false,
    duration_ms: dur,
    content_type: r.headers.get('content-type') ?? '',
    error: null,
  }
}

function mockResponse(config: RequestConfig, env: Record<string, string>): ResponseView {
  const url = replaceVars(config.url, env)
  const body = {
    mock: true,
    method: config.method,
    url,
    headers: config.headers.filter((h) => h.enabled && h.key).map((h) => [h.key, h.value]),
    params: config.params.filter((p) => p.enabled && p.key),
    echo: config.body,
    note: '当前为浏览器预览模式，数据来自本地 mock；打包后的桌面应用会真实发起请求',
  }
  const text = JSON.stringify(body)
  const pretty = JSON.stringify(body, null, 2)
  return {
    status_code: 200,
    status_text: 'OK',
    http_version: 'HTTP/1.1',
    headers: [
      ['content-type', 'application/json; charset=utf-8'],
      ['x-mock', '1'],
    ],
    request_url: url,
    body_text: text,
    body_pretty: pretty,
    body_len: text.length,
    truncated: false,
    is_binary: false,
    duration_ms: 18,
    content_type: 'application/json; charset=utf-8',
    error: null,
  }
}

export async function handle(cmd: string, args: Record<string, unknown> = {}): Promise<unknown> {
  switch (cmd) {
    case 'send_request': {
      const config = args.config as RequestConfig
      const env = (args.envVars ?? {}) as Record<string, string>
      let resp: ResponseView
      try {
        resp = await realResponse(config, env)
      } catch (e) {
        console.warn('[mock] 真实请求失败，退回本地 mock：', e)
        resp = mockResponse(config, env)
      }
      const item: HistoryItem = {
        id: nextId(),
        config: JSON.parse(JSON.stringify(config)),
        created_at: new Date().toISOString().slice(0, 19).replace('T', ' '),
        status_code: resp.status_code,
        duration_ms: resp.duration_ms,
      }
      db.history.unshift(item)
      db.history = db.history.slice(0, 200)
      db.lastName = JSON.stringify(config)
      persist()
      return resp
    }
    case 'cancel_request':
      return null
    case 'undefined_vars': {
      const config = args.config as RequestConfig
      const env = (args.envVars ?? {}) as Record<string, string>
      const text = JSON.stringify(config)
      const names = new Set<string>()
      for (const m of text.matchAll(/\{\{\s*([\w.-]+)\s*\}\}/g)) {
        if (!m[1].startsWith('$') && !(m[1] in env)) names.add(m[1])
      }
      return [...names]
    }
    case 'save_response_file':
      return null
    case 'get_last_config':
      return db.lastName ? (JSON.parse(db.lastName) as RequestConfig) : null
    case 'set_last_config':
      db.lastName = JSON.stringify(args.config)
      persist()
      return null
    case 'history_list':
      return db.history
    case 'history_clear':
      db.history = []
      persist()
      return null
    case 'history_delete':
      db.history = db.history.filter((h) => h.id !== args.id)
      persist()
      return null
    case 'collections_list':
      return db.collections
    case 'collection_create': {
      const col: Collection = { id: nextId(), parent_id: 0, name: String(args.name) }
      db.collections.push(col)
      persist()
      return col.id
    }
    case 'collection_delete':
      db.collections = db.collections.filter((c) => c.id !== args.id)
      db.saved = db.saved.filter((s) => s.collection_id !== args.id)
      persist()
      return null
    case 'collection_export_file':
    case 'collection_import_file':
      return null
    case 'saved_list':
      return db.saved.filter((s) => s.collection_id === args.collectionId)
    case 'saved_save': {
      const item: SavedRequest = {
        id: nextId(),
        collection_id: Number(args.collectionId),
        name: String(args.name),
        config: args.config as RequestConfig,
      }
      db.saved.push(item)
      persist()
      return item.id
    }
    case 'saved_delete':
      db.saved = db.saved.filter((s) => s.id !== args.id)
      persist()
      return null
    case 'saved_update': {
      const item = db.saved.find((s) => s.id === args.id)
      if (item) {
        item.name = String(args.name)
        item.config = args.config as RequestConfig
        persist()
      }
      return null
    }
    case 'saved_reorder': {
      const ids = args.ids as number[]
      const rest = db.saved.filter((s) => !ids.includes(s.id))
      const ordered = ids
        .map((id) => db.saved.find((s) => s.id === id))
        .filter((s): s is SavedRequest => !!s)
      db.saved = [...ordered, ...rest]
      persist()
      return null
    }
    case 'env_names':
      return Object.keys(db.envs).length ? Object.keys(db.envs) : ['global']
    case 'env_vars':
      return db.envs[String(args.envName)] ?? []
    case 'env_set': {
      const name = String(args.envName)
      const key = String(args.key)
      const list = db.envs[name] ?? []
      const found = list.find((v) => v.key === key)
      if (found) found.value = String(args.value)
      else list.push({ env_name: name, key, value: String(args.value) })
      db.envs[name] = list
      persist()
      return null
    }
    case 'env_delete': {
      const name = String(args.envName)
      db.envs[name] = (db.envs[name] ?? []).filter((v) => v.key !== args.key)
      persist()
      return null
    }
    case 'env_drop':
      delete db.envs[String(args.envName)]
      persist()
      return null
    case 'curl_export': {
      const config = args.config as RequestConfig
      const env = (args.envVars ?? {}) as Record<string, string>
      const url = replaceVars(config.url, env)
      const parts = [`curl -X ${config.method.toUpperCase()} '${url}'`]
      for (const h of config.headers.filter((h) => h.enabled && h.key)) {
        parts.push(`  -H '${h.key}: ${h.value}'`)
      }
      if (config.body.trim()) parts.push(`  -d '${config.body.replace(/'/g, "'\\''")}'`)
      return parts.join(' \\\n')
    }
    case 'curl_import': {
      const text = String(args.text)
      const url = text.match(/['"]?(https?:\/\/[^\s'"]+)['"]?/)?.[1] ?? ''
      const method = text.match(/-X\s+([A-Za-z]+)/)?.[1]?.toUpperCase() ?? 'GET'
      const methodMap: Record<string, string> = {
        GET: 'Get',
        POST: 'Post',
        PUT: 'Put',
        PATCH: 'Patch',
        DELETE: 'Delete',
        HEAD: 'Head',
        OPTIONS: 'Options',
      }
      return {
        ...(args.config as RequestConfig),
        url,
        method: methodMap[method] ?? 'Get',
      }
    }
    case 'pick_file':
      return null
    default:
      throw new Error(`未知命令：${cmd}`)
  }
}
