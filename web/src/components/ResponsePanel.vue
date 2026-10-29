<script setup lang="ts">
import { Download, DocumentCopy, Picture, Search } from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus'
import { computed, ref } from 'vue'

import { api } from '@/api'
import { humanSize, statusColor } from '@/api/types'
import JsonView from '@/components/JsonView.vue'
import { copyResponseBody, loadEnv, saveResponseBody, state } from '@/store'

const resp = computed(() => state.response)

/** 响应体搜索（只在 Body 页生效） */
const query = ref('')

const isJson = computed(() => {
  const r = resp.value
  if (!r || r.truncated || r.error || r.is_binary) return false
  const ct = (r.content_type || '').toLowerCase()
  if (ct.includes('json')) return true
  try {
    JSON.parse(r.body_text)
    return true
  } catch {
    return false
  }
})

const viewMode = computed({
  get: () => (state.rawView ? 'raw' : 'pretty'),
  set: (value: string) => {
    state.rawView = value === 'raw'
  },
})

const displayText = computed(() => {
  const r = resp.value
  if (!r) return ''
  return state.rawView || !isJson.value ? r.body_text : r.body_pretty
})

const hitCount = computed(() => {
  const q = query.value.trim().toLowerCase()
  if (!q) return 0
  return displayText.value.toLowerCase().split(q).length - 1
})

/** 原始文本视图按行高亮，避免整段字符串重建 */
const rawLines = computed(() => {
  const q = query.value.trim()
  const text = displayText.value
  if (!q) return [{ text, hit: false }]
  const out: { text: string; hit: boolean }[] = []
  let from = 0
  const lower = text.toLowerCase()
  const needle = q.toLowerCase()
  while (from < text.length) {
    const at = lower.indexOf(needle, from)
    if (at < 0) break
    if (at > from) out.push({ text: text.slice(from, at), hit: false })
    out.push({ text: text.slice(at, at + q.length), hit: true })
    from = at + q.length
  }
  if (from < text.length) out.push({ text: text.slice(from), hit: false })
  return out
})

// ---------- 从响应提取变量 ----------

const jsonValue = computed(() => {
  const r = resp.value
  if (!r || r.is_binary || r.truncated) return null
  try {
    return JSON.parse(r.body_text) as unknown
  } catch {
    return null
  }
})

const extractOpen = ref(false)
const extractPath = ref('')
const extractName = ref('')
const extractEnv = ref('global')

/** 极简路径取值：支持 a.b[0].c 这种写法 */
function pickByPath(value: unknown, path: string): unknown {
  if (!path.trim()) return undefined
  const parts = path
    .replace(/\[(\d+)\]/g, '.$1')
    .split('.')
    .map((p) => p.trim())
    .filter(Boolean)
  let current: unknown = value
  for (const part of parts) {
    if (current == null) return undefined
    if (Array.isArray(current)) {
      const index = Number(part)
      if (!Number.isInteger(index)) return undefined
      current = current[index]
      continue
    }
    if (typeof current !== 'object') return undefined
    current = (current as Record<string, unknown>)[part]
  }
  return current
}

const extractPreview = computed(() => {
  const found = pickByPath(jsonValue.value, extractPath.value)
  if (found === undefined) return ''
  return typeof found === 'string' ? found : JSON.stringify(found)
})

function openExtract() {
  extractPath.value = ''
  extractName.value = ''
  extractEnv.value = state.currentEnv
  extractOpen.value = true
}

async function submitExtract() {
  const name = extractName.value.trim()
  if (!name) {
    ElMessage.warning('请填写变量名')
    return
  }
  const value = extractPreview.value
  if (value === '') {
    ElMessage.warning('路径没有匹配到内容')
    return
  }
  try {
    await api.envSet(extractEnv.value, name, value)
    if (extractEnv.value === state.currentEnv) {
      await loadEnv(state.currentEnv)
      ElMessage.success(`已写入 {{${name}}}`)
    } else {
      ElMessage.success(`已写入环境「${extractEnv.value}」的 {{${name}}}`)
    }
    extractOpen.value = false
  } catch (e) {
    ElMessage.error(`写入失败：${String(e)}`)
  }
}
</script>

<template>
  <div
    class="response-panel"
    :style="{
      '--status-color':
        resp && !resp.error ? statusColor(resp.status_code) : resp?.error ? '#dc2626' : 'transparent',
    }"
  >
    <el-empty
      v-if="!resp"
      description="尚无响应"
      :image-size="72"
    >
      <span class="hint">填写 URL 后点击「发送」，响应结果将显示在这里</span>
    </el-empty>

    <template v-else-if="resp.error">
      <div class="resp-head">
        <el-tag type="danger" effect="dark" size="small">请求失败</el-tag>
        <span class="meta-chip">{{ resp.duration_ms }} ms</span>
      </div>
      <div style="padding: 0 14px 14px">
        <el-alert type="error" :closable="false" :title="resp.error" show-icon />
        <div class="hint" style="margin-top: 8px">
          可检查：URL 是否可访问、是否需要代理（高级 → 自定义代理）、或服务端证书是否受信任。
        </div>
      </div>
    </template>

    <template v-else>
      <div class="resp-head">
        <el-tooltip :content="`实际请求：${resp.request_url}`" placement="top" :show-after="150">
          <span
            class="status-chip"
            :style="{ backgroundColor: statusColor(resp.status_code) }"
          >
            {{ resp.status_code }} {{ resp.status_text }}
          </span>
        </el-tooltip>
        <span class="meta-chip">{{ resp.duration_ms }} ms</span>
        <span class="meta-chip">{{ humanSize(resp.body_len) }}</span>
        <span v-if="resp.http_version" class="meta-chip">{{ resp.http_version }}</span>
        <div class="meta-right">
          <el-button
            v-if="!resp.is_binary"
            text
            size="small"
            :icon="DocumentCopy"
            @click="copyResponseBody"
          >
            复制
          </el-button>
          <el-button text size="small" :icon="Download" @click="saveResponseBody">下载</el-button>
        </div>
      </div>

      <el-tabs v-model="state.responseTab" class="resp-tabs">
        <el-tab-pane label="Body" name="body">
          <div class="resp-toolbar">
            <el-radio-group v-if="isJson" v-model="viewMode" size="small">
              <el-radio-button value="pretty">格式化</el-radio-button>
              <el-radio-button value="raw">原始</el-radio-button>
            </el-radio-group>
            <el-input
              v-if="!resp.is_binary"
              v-model="query"
              class="resp-search"
              size="small"
              placeholder="在响应中搜索"
              :prefix-icon="Search"
              clearable
            />
            <span v-if="query.trim()" class="hint">命中 {{ hitCount }} 处</span>
            <el-button
              v-if="jsonValue"
              text
              size="small"
              type="primary"
              @click="openExtract"
            >
              提取为变量
            </el-button>
            <el-tag v-if="resp.truncated" type="warning" size="small" effect="plain">
              内容过大，仅显示前 {{ humanSize(1024 * 1024) }}，完整内容请点「下载」
            </el-tag>
          </div>

          <div class="scroll-y" style="height: 100%">
            <div v-if="resp.is_binary" class="body-empty">
              <el-icon class="body-empty__icon" :size="22"><Picture /></el-icon>
              <span class="body-empty__title">二进制响应，不做文本预览</span>
              <span class="body-empty__hint">
                {{ resp.content_type || '未知类型' }} · {{ humanSize(resp.body_len) }}，可点右上角「下载」保存
              </span>
            </div>
            <span v-else-if="!displayText.trim()" class="hint">（空响应体）</span>
            <JsonView v-else-if="isJson && !state.rawView" :text="displayText" :query="query" />
            <pre v-else class="raw-view"><span
              v-for="(seg, i) in rawLines"
              :key="i"
              :class="{ 'json-hit': seg.hit }"
            >{{ seg.text }}</span></pre>
          </div>
        </el-tab-pane>

        <el-tab-pane :label="`Headers (${resp.headers.length})`" name="headers">
          <el-table :data="resp.headers" size="small" style="width: 100%">
            <el-table-column label="Header" width="240">
              <template #default="{ row }">
                <span class="mono" style="color: #ea580c">{{ row[0] }}</span>
              </template>
            </el-table-column>
            <el-table-column label="值">
              <template #default="{ row }">
                <span class="mono" style="word-break: break-all">{{ row[1] }}</span>
              </template>
            </el-table-column>
          </el-table>
        </el-tab-pane>
      </el-tabs>
    </template>

    <el-dialog v-model="extractOpen" title="从响应提取变量" width="480px" append-to-body>
      <el-form label-width="76px" label-position="left" size="small">
        <el-form-item label="取值路径">
          <el-input v-model="extractPath" placeholder="如 data.token 或 items[0].id" class="mono" />
        </el-form-item>
        <el-form-item label="取到的值">
          <span class="mono" style="word-break: break-all">{{ extractPreview || '（未匹配到内容）' }}</span>
        </el-form-item>
        <el-form-item label="变量名">
          <el-input v-model="extractName" placeholder="如 token" class="mono" />
        </el-form-item>
        <el-form-item label="写入环境">
          <el-select v-model="extractEnv" style="width: 200px">
            <el-option v-for="name in state.envNames" :key="name" :label="name" :value="name" />
          </el-select>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="extractOpen = false">取消</el-button>
        <el-button type="primary" @click="submitExtract">保存</el-button>
      </template>
    </el-dialog>
  </div>
</template>
