<script setup lang="ts">
import { Download, DocumentCopy, Picture } from '@element-plus/icons-vue'
import { computed } from 'vue'

import { humanSize, statusColor } from '@/api/types'
import JsonView from '@/components/JsonView.vue'
import { copyResponseBody, saveResponseBody, state } from '@/store'

const resp = computed(() => state.response)

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
            <JsonView v-else-if="isJson && !state.rawView" :text="displayText" />
            <pre v-else class="raw-view">{{ displayText }}</pre>
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
  </div>
</template>
