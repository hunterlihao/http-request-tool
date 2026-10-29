/** 与 Rust 端模型一致的数据结构（serde 默认枚举名） */

export type Method = 'Get' | 'Post' | 'Put' | 'Patch' | 'Delete' | 'Head' | 'Options'
export type BodyType = 'None' | 'Json' | 'Form' | 'Text' | 'Multipart'
export type AuthType = 'None' | 'Basic' | 'Bearer' | 'CustomHeader'

export interface KVPair {
  key: string
  value: string
  enabled: boolean
}

export interface Auth {
  auth_type: AuthType
  username: string
  password: string
  token: string
  header_name: string
  header_value: string
}

export interface RequestConfig {
  method: Method
  url: string
  params: KVPair[]
  headers: KVPair[]
  body_type: BodyType
  body: string
  form_fields: KVPair[]
  auth: Auth
  insecure_skip_verify: boolean
  follow_redirects: boolean
  timeout_secs: number
  proxy: string
  /** 旧版单文件字段（保留兼容） */
  file_path: string
  /** multipart 文件字段名 */
  file_field: string
  /** multipart 上传文件列表 */
  file_paths: string[]
  use_custom_proxy: boolean
}

export interface ResponseView {
  status_code: number
  status_text: string
  http_version: string
  headers: [string, string][]
  request_url: string
  body_text: string
  body_pretty: string
  body_len: number
  truncated: boolean
  /** 二进制响应（图片 / 压缩包等），body_text 为空，只能下载 */
  is_binary: boolean
  duration_ms: number
  content_type: string
  error: string | null
}

export interface Collection {
  id: number
  parent_id: number
  name: string
}

export interface SavedRequest {
  id: number
  collection_id: number
  name: string
  config: RequestConfig
}

export interface HistoryItem {
  id: number
  config: RequestConfig
  created_at: string
  status_code: number | null
  duration_ms: number | null
}

export interface EnvVar {
  env_name: string
  key: string
  value: string
}

export const METHODS: Method[] = ['Get', 'Post', 'Put', 'Patch', 'Delete', 'Head', 'Options']

export const METHOD_LABEL: Record<Method, string> = {
  Get: 'GET',
  Post: 'POST',
  Put: 'PUT',
  Patch: 'PATCH',
  Delete: 'DELETE',
  Head: 'HEAD',
  Options: 'OPTIONS',
}

/** 方法配色：与 Element Plus 语义色协调的自定义色板 */
export const METHOD_COLOR: Record<Method, string> = {
  Get: '#0f9d58',
  Post: '#e8590c',
  Put: '#2563eb',
  Patch: '#7c3aed',
  Delete: '#dc2626',
  Head: '#64748b',
  Options: '#0891b2',
}

export const BODY_TYPES: BodyType[] = ['None', 'Json', 'Form', 'Text', 'Multipart']

export const BODY_TYPE_LABEL: Record<BodyType, string> = {
  None: 'none',
  Json: 'JSON',
  Form: 'Form URL Encoded',
  Text: 'Text',
  Multipart: 'Multipart Form',
}

export const AUTH_TYPES: AuthType[] = ['None', 'Basic', 'Bearer', 'CustomHeader']

export const AUTH_TYPE_LABEL: Record<AuthType, string> = {
  None: '无',
  Basic: 'Basic Auth',
  Bearer: 'Bearer Token',
  CustomHeader: '自定义 Header',
}

/** 常用请求头：Header 名称支持下拉选择，也可直接输入 */
export const COMMON_HEADERS: string[] = [
  'Accept',
  'Accept-Charset',
  'Accept-Encoding',
  'Accept-Language',
  'Authorization',
  'Cache-Control',
  'Connection',
  'Content-Length',
  'Content-Type',
  'Cookie',
  'Host',
  'If-Modified-Since',
  'If-None-Match',
  'Origin',
  'Pragma',
  'Range',
  'Referer',
  'User-Agent',
  'X-API-Key',
  'X-Auth-Token',
  'X-Request-Id',
  'X-Requested-With',
]

export function emptyConfig(): RequestConfig {
  return {
    method: 'Get',
    url: '',
    params: [],
    headers: [],
    body_type: 'None',
    body: '',
    form_fields: [],
    auth: {
      auth_type: 'None',
      username: '',
      password: '',
      token: '',
      header_name: '',
      header_value: '',
    },
    insecure_skip_verify: false,
    follow_redirects: true,
    timeout_secs: 30,
    proxy: '',
    file_path: '',
    file_field: 'file',
    file_paths: [],
    use_custom_proxy: false,
  }
}

export function statusColor(code: number): string {
  if (code >= 200 && code < 300) return '#0f9d58'
  if (code >= 300 && code < 400) return '#2563eb'
  if (code >= 400 && code < 500) return '#b45309'
  return '#dc2626'
}

export function humanSize(bytes: number): string {
  const units = ['B', 'KB', 'MB', 'GB']
  let size = bytes
  let unit = 0
  while (size >= 1024 && unit < units.length - 1) {
    size /= 1024
    unit += 1
  }
  return unit === 0 ? `${bytes} B` : `${size.toFixed(1)} ${units[unit]}`
}
