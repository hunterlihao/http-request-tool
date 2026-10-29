<script setup lang="ts">
import { Delete, FolderAdd, Search } from '@element-plus/icons-vue'
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
  exportCollection,
  importCollection,
  loadEnv,
  persistEnvVar,
  removeEnvVar,
  saveCurrentRequest,
  selectCollection,
  state,
} from '@/store'

const newCollection = ref('')
const newEnv = ref('')
const saveDialog = ref(false)
const saveName = ref('')

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

const currentCollectionName = computed(
  () => state.collections.find((c) => c.id === state.currentCollectionId)?.name ?? '（无集合）',
)

async function submitCollection() {
  await createCollection(newCollection.value)
  newCollection.value = ''
}

async function submitEnv() {
  await createEnv(newEnv.value)
  newEnv.value = ''
}

async function submitSave() {
  const ok = await saveCurrentRequest(saveName.value)
  if (ok) {
    saveDialog.value = false
    saveName.value = ''
  }
}

function openSaveDialog() {
  saveName.value = state.config.url || '未命名请求'
  saveDialog.value = true
}

function addEnvVar() {
  state.envVars.push({ env_name: state.currentEnv, key: '', value: '' })
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
          <div class="list">
            <div
              v-for="item in filteredHistory"
              :key="item.id"
              class="list-row"
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

          <span class="section-title">已保存请求（{{ state.savedRequests.length }}）</span>
          <div class="list">
            <div
              v-for="req in state.savedRequests"
              :key="req.id"
              class="list-row"
              @click="applyConfig(req.config)"
            >
              <span class="method" :style="{ color: METHOD_COLOR[req.config.method] }">
                {{ METHOD_LABEL[req.config.method] }}
              </span>
              <span class="text" :title="req.name">{{ req.name }}</span>
              <span class="row-actions">
                <el-button
                  text
                  size="small"
                  :icon="Delete"
                  @click.stop="deleteSavedRequest(req.id)"
                />
              </span>
            </div>
            <el-empty
              v-if="!state.savedRequests.length"
              description="集合为空，保存当前请求试试"
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

          <div class="list">
            <div v-for="(item, index) in state.envVars" :key="`${item.key}-${index}`" class="env-grid-row">
              <el-input
                v-model="item.key"
                size="small"
                placeholder="key"
                class="mono"
                @change="persistEnvVar(item.key, item.value)"
              />
              <el-input
                v-model="item.value"
                size="small"
                placeholder="value"
                class="mono"
                @change="persistEnvVar(item.key, item.value)"
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

    <el-dialog v-model="saveDialog" title="保存请求到集合" width="420px" append-to-body>
      <span class="hint">为这个请求起个名字，便于以后在集合中找到它</span>
      <el-input
        v-model="saveName"
        placeholder="未命名请求"
        style="margin-top: 10px"
        @keyup.enter="submitSave"
      />
      <template #footer>
        <el-button @click="saveDialog = false">取消</el-button>
        <el-button type="primary" @click="submitSave">保存</el-button>
      </template>
    </el-dialog>
  </div>
</template>
