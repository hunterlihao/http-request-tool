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
      const resp = mockResponse(config, env)
      const item: HistoryItem = {
        id: nextId(),
        config: JSON.parse(JSON.stringify(config)),
        created_at: new Date().toISOString().slice(0, 19).replace('T', ' '),
        status_code: 200,
        duration_ms: 18,
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
