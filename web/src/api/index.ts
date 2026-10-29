/** 后端调用层：Tauri 环境下走 IPC，纯浏览器环境走本地 mock（仅开发期） */

import { invoke } from '@tauri-apps/api/core'

import type {
  Collection,
  EnvVar,
  HistoryItem,
  RequestConfig,
  ResponseView,
  SavedRequest,
} from './types'

export const inTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (inTauri) {
    return invoke<T>(cmd, args)
  }
  if (!import.meta.env.DEV) {
    throw new Error('当前环境不支持请求后端')
  }
  const mock = await import('./mock')
  return mock.handle(cmd, args) as Promise<T>
}

export const api = {
  // 请求
  sendRequest: (config: RequestConfig, envVars: Record<string, string>) =>
    call<ResponseView>('send_request', { config, envVars }),
  cancelRequest: () => call<void>('cancel_request'),
  undefinedVars: (config: RequestConfig, envVars: Record<string, string>) =>
    call<string[]>('undefined_vars', { config, envVars }),
  saveResponseFile: () => call<string | null>('save_response_file'),

  // 设置
  getLastConfig: () => call<RequestConfig | null>('get_last_config'),
  setLastConfig: (config: RequestConfig) => call<void>('set_last_config', { config }),

  // 历史
  historyList: (limit = 500) => call<HistoryItem[]>('history_list', { limit }),
  historyClear: () => call<void>('history_clear'),
  historyDelete: (id: number) => call<void>('history_delete', { id }),

  // 集合
  collectionsList: () => call<Collection[]>('collections_list'),
  collectionCreate: (name: string, parentId = 0) =>
    call<number>('collection_create', { name, parentId }),
  collectionDelete: (id: number) => call<void>('collection_delete', { id }),
  collectionExportFile: (id: number, name: string) =>
    call<string | null>('collection_export_file', { id, name }),
  collectionImportFile: () => call<number | null>('collection_import_file'),

  // 已保存请求
  savedList: (collectionId: number) => call<SavedRequest[]>('saved_list', { collectionId }),
  savedSave: (collectionId: number, name: string, config: RequestConfig) =>
    call<number>('saved_save', { collectionId, name, config }),
  savedDelete: (id: number) => call<void>('saved_delete', { id }),

  // 环境
  envNames: () => call<string[]>('env_names'),
  envVars: (envName: string) => call<EnvVar[]>('env_vars', { envName }),
  envSet: (envName: string, key: string, value: string) =>
    call<void>('env_set', { envName, key, value }),
  envDelete: (envName: string, key: string) => call<void>('env_delete', { envName, key }),
  envDrop: (envName: string) => call<void>('env_drop', { envName }),

  // cURL
  curlExport: (config: RequestConfig, envVars: Record<string, string>) =>
    call<string>('curl_export', { config, envVars }),
  curlImport: (text: string) => call<RequestConfig>('curl_import', { text }),

  // 文件
  pickFile: () => call<string | null>('pick_file'),
}
