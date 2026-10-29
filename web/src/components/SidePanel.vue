<script setup lang="ts">
import { CopyDocument, Delete, Edit, FolderAdd, Search } from '@element-plus/icons-vue'
import { computed, ref } from 'vue'

import { METHOD_COLOR, METHOD_LABEL, statusColor, type HistoryItem } from '@/api/types'
import {
  applyConfig,
  clearHistory,
  createCollection,
  createEnv,
  deleteCollection,
  deleteHistory,
  deleteSavedRequest,
  dropCurrentEnv,
  duplicateSavedRequest,
  exportCollection,
  importCollection,
  loadEnv,
  moveSavedRequest,
  openSaveDialog,
  openSavedRequest,
  persistEnvVar,
  removeEnvVar,
  renameSavedRequest,
  selectCollection,
  state,
  submitSaveDialog,
} from '@/store'

const newCollection = ref('')
const newEnv = ref('')

/** 已保存请求的行内改名 */
const editingId = ref<number | null>(null)
const editingName = ref('')
/** 拖拽排序的源行 */
const dragId = ref<number | null>(null)

function beginRename(id: number, name: string) {
  editingId.value = id
  editingName.value = name
}

async function commitRename(id: number) {
  if (editingId.value !== id) return
  const name = editingName.value
  editingId.value = null
  await renameSavedRequest(id, name)
}

function startDragSaved(id: number) {
  dragId.value = id
}

function dropSaved(targetId: number) {
  if (dragId.value == null || dragId.value === targetId) return
  const from = state.savedRequests.findIndex((r) => r.id === dragId.value)
  const to = state.savedRequests.findIndex((r) => r.id === targetId)
  dragId.value = null
  moveSavedRequest(from, to)
}

/** 历史按 方法 + URL 去重，保留最新一条 */
const filteredHistory = computed<HistoryItem[]>(() => {
  const keyword = state.historySearch.trim().toLowerCase()
  const seen = new Set<string>()
  const out: HistoryItem[] = []
  for (const item of state.history) {
    if (
      keyword &&
      !item.config.url.toLowerCase().includes(keyword) &&
      !METHOD_LABEL[item.config.method].toLowerCase().includes(keyword)
    ) {
      continue
    }
    const key = `${item.config.method}|${item.config.url}`
    if (seen.has(key)) continue
    seen.add(key)
    out.push(item)
  }
  return out
})

/** 集合内请求按名称/URL 过滤 */
const filteredSaved = computed(() => {
  const keyword = state.collectionSearch.trim().toLowerCase()
  if (!keyword) return state.savedRequests
  return state.savedRequests.filter(
    (req) =>
      req.name.toLowerCase().includes(keyword) || req.config.url.toLowerCase().includes(keyword),
  )
})

/** created_at 形如 2026-10-29 12:03:11，列表里只显示时间/日期 */
function shortTime(createdAt: string): string {
  if (!createdAt) return ''
  const [date, time = ''] = createdAt.split(' ')
  const now = new Date()
  const today = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}-${String(
    now.getDate(),
  ).padStart(2, '0')}`
  if (date === today) return time.slice(0, 5)
  return date.slice(5)
}

async function submitCollection() {
  await createCollection(newCollection.value)
  newCollection.value = ''
}

async function submitEnv() {
  await createEnv(newEnv.value)
  newEnv.value = ''
}

function addEnvVar() {
  state.envVars.push({ env_name: state.currentEnv, key: '', value: '' })
  state.envOriginals.push('')
}
</script>

<template>
  <div class="side-panel">
    <el-tabs v-model="state.sideTab" class="side-tabs" stretch>
      <!-- 历史 -->
      <el-tab-pane name="history">
        <template #label>历史</template>
        <div class="side-pane">
          <el-input
            v-model="state.historySearch"
            size="small"
            placeholder="搜索 URL 或方法"
            :prefix-icon="Search"
            clearable
          />
          <div class="side-row">
            <span class="hint">共 {{ filteredHistory.length }} 条</span>
            <div style="margin-left: auto">
              <el-button text size="small" @click="clearHistory">清空</el-button>
            </div>
          </div>
          <span class="hint">历史只保留最近 500 条，且不记录认证信息</span>
          <div class="list">
            <div
              v-for="item in filteredHistory"
              :key="item.id"
              class="list-row"
              :title="`${item.created_at}${item.duration_ms != null ? ` · ${item.duration_ms} ms` : ''}`"
              @click="applyConfig(item.config)"
            >
              <span class="method" :style="{ color: METHOD_COLOR[item.config.method] }">
                {{ METHOD_LABEL[item.config.method] }}
              </span>
              <span class="text" :title="item.config.url">
                {{ item.config.url || '（空 URL）' }}
              </span>
              <span
                v-if="item.status_code"
                class="mono"
                :style="{ color: statusColor(item.status_code), fontSize: '11px' }"
              >
                {{ item.status_code }}
              </span>
              <span v-if="item.created_at" class="row-time">{{ shortTime(item.created_at) }}</span>
              <span class="row-actions">
                <el-button
                  text
                  size="small"
                  :icon="Delete"
                  @click.stop="deleteHistory(item.id)"
                />
              </span>
            </div>
            <el-empty
              v-if="!filteredHistory.length"
              description="暂无历史，发送请求后自动记录"
              :image-size="72"
            />
          </div>
        </div>
      </el-tab-pane>

      <!-- 集合 -->
      <el-tab-pane name="collections">
        <template #label>集合</template>
        <div class="side-pane">
          <div class="side-row">
            <el-select
              :model-value="state.currentCollectionId ?? undefined"
              size="small"
              placeholder="选择集合"
              style="flex: 1"
              @change="selectCollection"
            >
              <el-option
                v-for="col in state.collections"
                :key="col.id"
                :label="col.name"
                :value="col.id"
              />
            </el-select>
            <el-button
              text
              size="small"
              type="danger"
              :icon="Delete"
              :disabled="state.currentCollectionId == null"
              title="删除当前集合"
              @click="state.currentCollectionId && deleteCollection(state.currentCollectionId)"
            />
          </div>

          <div class="side-row">
            <el-input
              v-model="newCollection"
              size="small"
              placeholder="新集合名称"
              @keyup.enter="submitCollection"
            />
            <el-button size="small" :icon="FolderAdd" @click="submitCollection">新建</el-button>
          </div>

          <el-button size="small" style="width: 100%" @click="openSaveDialog">保存当前请求</el-button>

          <div class="side-row">
            <el-button size="small" style="flex: 1 1 0; min-width: 0" @click="importCollection">
              导入
            </el-button>
            <el-button size="small" style="flex: 1 1 0; min-width: 0" @click="exportCollection">
              导出
            </el-button>
          </div>

          <span class="section-title">已保存请求（{{ filteredSaved.length }}）</span>
          <el-input
            v-if="state.savedRequests.length > 4"
            v-model="state.collectionSearch"
            size="small"
            placeholder="搜索已保存请求"
            :prefix-icon="Search"
            clearable
          />
          <div class="list">
            <div
              v-for="req in filteredSaved"
              :key="req.id"
              class="list-row"
              :class="{ active: state.currentSavedId === req.id }"
              :title="req.config.url"
              :draggable="!state.collectionSearch"
              @click="openSavedRequest(req)"
              @dragstart="startDragSaved(req.id)"
              @dragover.prevent
              @drop="dropSaved(req.id)"
              @dragend="dragId = null"
            >
              <span v-if="!state.collectionSearch" class="drag-handle" title="拖拽排序">⠿</span>
              <span class="method" :style="{ color: METHOD_COLOR[req.config.method] }">
                {{ METHOD_LABEL[req.config.method] }}
              </span>
              <el-input
                v-if="editingId === req.id"
                v-model="editingName"
                size="small"
                class="rename-input"
                @click.stop
                @keyup.enter="commitRename(req.id)"
                @blur="commitRename(req.id)"
              />
              <span v-else class="text" :title="req.name">{{ req.name }}</span>
              <span class="row-actions">
                <el-button
                  text
                  size="small"
                  :icon="CopyDocument"
                  title="创建副本"
                  @click.stop="duplicateSavedRequest(req.id)"
                />
                <el-button
                  text
                  size="small"
                  :icon="Edit"
                  title="重命名"
                  @click.stop="beginRename(req.id, req.name)"
                />
                <el-button
                  text
                  size="small"
                  :icon="Delete"
                  @click.stop="deleteSavedRequest(req.id)"
                />
              </span>
            </div>
            <el-empty
              v-if="!filteredSaved.length"
              :description="state.savedRequests.length ? '没有匹配的请求' : '集合为空，保存当前请求试试'"
              :image-size="72"
            />
          </div>
        </div>
      </el-tab-pane>

      <!-- 环境变量 -->
      <el-tab-pane name="environments">
        <template #label>环境</template>
        <div class="side-pane">
          <div class="side-row">
            <el-select
              :model-value="state.currentEnv"
              size="small"
              style="flex: 1"
              @change="loadEnv"
            >
              <el-option v-for="name in state.envNames" :key="name" :label="name" :value="name" />
            </el-select>
            <el-button
              text
              size="small"
              type="danger"
              :icon="Delete"
              :disabled="state.currentEnv === 'global'"
              title="删除当前环境"
              @click="dropCurrentEnv"
            />
          </div>

          <div class="side-row">
            <el-input
              v-model="newEnv"
              size="small"
              placeholder="新环境名"
              @keyup.enter="submitEnv"
            />
            <el-button size="small" @click="submitEnv">新建</el-button>
          </div>

          <span class="hint" v-text="'在 URL / Header / Body 中用 {{变量名}} 引用'"></span>
          <span class="hint" v-text="'内置：{{$timestamp}} {{$guid}} {{$randomInt}}'"></span>

          <div class="list">
            <div v-for="(item, index) in state.envVars" :key="`${item.key}-${index}`" class="env-grid-row">
              <el-input
                v-model="item.key"
                size="small"
                placeholder="key"
                class="mono"
                @change="persistEnvVar(index)"
              />
              <el-input
                v-model="item.value"
                size="small"
                placeholder="value"
                class="mono"
                @change="persistEnvVar(index)"
              />
              <el-button text size="small" :icon="Delete" @click="removeEnvVar(index)" />
            </div>
            <el-empty
              v-if="!state.envVars.length"
              description="当前环境还没有变量"
              :image-size="72"
            />
          </div>

          <el-button size="small" style="width: 100%" @click="addEnvVar">添加变量</el-button>
        </div>
      </el-tab-pane>
    </el-tabs>

    <el-dialog v-model="state.saveDialogOpen" title="保存请求到集合" width="420px" append-to-body>
      <span class="hint">为这个请求起个名字，便于以后在集合中找到它</span>
      <el-input
        v-model="state.saveName"
        placeholder="未命名请求"
        style="margin-top: 10px"
        @keyup.enter="submitSaveDialog(false)"
      />
      <template #footer>
        <el-button @click="state.saveDialogOpen = false">取消</el-button>
        <el-button v-if="state.currentSavedId != null" @click="submitSaveDialog(true)">
          覆盖原请求
        </el-button>
        <el-button type="primary" @click="submitSaveDialog(false)">另存为新请求</el-button>
      </template>
    </el-dialog>
  </div>
</template>
