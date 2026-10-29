<script setup lang="ts">
import { Delete, DocumentDelete, FolderOpened, MagicStick, Upload } from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus'
import { computed, ref } from 'vue'

import { api } from '@/api'
import {
  AUTH_TYPES,
  AUTH_TYPE_LABEL,
  BODY_TYPES,
  BODY_TYPE_LABEL,
  COMMON_HEADERS,
  METHODS,
  METHOD_COLOR,
  METHOD_LABEL,
} from '@/api/types'
import KeyValueTable from '@/components/KeyValueTable.vue'
import { cancelRequest, copyAsCode, importCurl, sendRequest, state } from '@/store'

const curlDialog = ref(false)
const curlText = ref('')

/** 输入 {{ 后提示可用的变量（环境变量 + 内置变量） */
function suggestVars(_query: string, cb: (items: { value: string }[]) => void) {
  const text = state.config.url
  const at = text.lastIndexOf('{{')
  if (at < 0) return cb([])
  const tail = text.slice(at + 2)
  // 已经闭合的不再提示
  if (tail.includes('}}')) return cb([])
  const partial = tail.trim().toLowerCase()
  const names = [
    ...Object.keys(state.envMap),
    '$timestamp',
    '$isoTimestamp',
    '$guid',
    '$randomInt',
  ]
  cb(
    names
      .filter((name) => name.toLowerCase().includes(partial))
      .map((name) => ({ value: name })),
  )
}

/** 选中变量后补全为 {{name}}，替换掉未闭合的部分 */
function onVarInsert(item: { value: string }) {
  const text = state.config.url
  const at = text.lastIndexOf('{{')
  if (at < 0) return
  state.config.url = `${text.slice(0, at)}{{${item.value}}}`
}

const paramsCount = () => state.config.params.filter((p) => p.key.trim()).length
const headersCount = () => state.config.headers.filter((h) => h.key.trim()).length

/** JSON 请求体的实时校验提示 */
const jsonError = computed(() => {
  if (state.config.body_type !== 'Json') return ''
  const text = state.config.body.trim()
  if (!text) return ''
  try {
    JSON.parse(text)
    return ''
  } catch (e) {
    return (e as Error).message
  }
})

async function submitCurl() {
  if (!curlText.value.trim()) {
    ElMessage.warning('请粘贴 cURL 命令')
    return
  }
  const ok = await importCurl(curlText.value)
  if (ok) {
    curlDialog.value = false
    curlText.value = ''
  }
}

/** multipart 已选文件（兼容旧的单文件字段） */
const files = computed(() =>
  state.config.file_paths.length
    ? state.config.file_paths
    : state.config.file_path
      ? [state.config.file_path]
      : [],
)

function baseName(path: string): string {
  return path.split(/[\\/]/).pop() || path
}

function removeFile(index: number) {
  if (state.config.file_paths.length) state.config.file_paths.splice(index, 1)
  else state.config.file_path = ''
}

async function chooseFiles() {
  const paths = await api.pickFiles()
  if (!paths.length) return
  // 首次追加时把旧字段迁移进列表
  if (!state.config.file_paths.length && state.config.file_path) state.config.file_path = ''
  for (const path of paths) {
    if (!state.config.file_paths.includes(path)) state.config.file_paths.push(path)
  }
}

async function clearCookies() {
  try {
    await api.cookiesClear()
    ElMessage.success('已清除本次运行的 Cookie')
  } catch (e) {
    ElMessage.error(`清除失败：${String(e)}`)
  }
}

/** 跳转到环境变量页补全变量 */
function goToEnv() {
  state.sideTab = 'environments'
  state.sidebarVisible = true
}

function formatBody() {
  try {
    const value = JSON.parse(state.config.body)
    state.config.body = JSON.stringify(value, null, 2)
    ElMessage.success('已格式化 JSON')
  } catch {
    ElMessage.error('当前内容不是合法 JSON')
  }
}

/** 把 URL 中的查询参数拆分到 Params 表格 */
function importParamsFromUrl() {
  try {
    const parsed = new URL(state.config.url.trim())
    const pairs = [...parsed.searchParams.entries()]
    if (!pairs.length) {
      ElMessage.info('URL 中没有查询参数')
      return
    }
    for (const [key, value] of pairs) {
      state.config.params.push({ key, value, enabled: true })
    }
    parsed.search = ''
    state.config.url = parsed.toString()
    ElMessage.success('已从 URL 导入查询参数')
  } catch {
    ElMessage.error('URL 无法解析')
  }
}
</script>

<template>
  <div class="request-panel">
    <div class="url-bar">
      <el-select v-model="state.config.method" class="method-select" size="default">
        <template #label="{ label }">
          <span :style="{ color: METHOD_COLOR[state.config.method], fontWeight: 600 }">
            {{ label }}
          </span>
        </template>
        <el-option v-for="m in METHODS" :key="m" :label="METHOD_LABEL[m]" :value="m">
          <span :style="{ color: METHOD_COLOR[m], fontWeight: 600 }">{{ METHOD_LABEL[m] }}</span>
        </el-option>
      </el-select>

      <el-autocomplete
        v-model="state.config.url"
        class="url-input"
        placeholder="https://api.example.com/endpoint"
        clearable
        :fetch-suggestions="suggestVars"
        :trigger-on-focus="false"
        @select="onVarInsert"
        @keyup.enter="sendRequest"
      />

      <el-button
        :type="state.sending ? 'danger' : 'primary'"
        @click="state.sending ? cancelRequest() : sendRequest()"
      >
        {{ state.sending ? '取消' : '发送' }}
      </el-button>
    </div>

    <div class="url-meta">
      <span class="hint">Ctrl + Enter 发送 · Ctrl + S 保存到集合</span>
      <el-divider direction="vertical" />
      <span class="hint">当前环境：{{ state.currentEnv }}</span>
      <el-tooltip
        v-if="state.undefinedVars.length"
        content="这些变量在当前环境没有值，发送时会原样保留；点击去环境变量里补上"
      >
        <el-tag
          type="warning"
          size="small"
          effect="plain"
          style="cursor: pointer"
          @click="goToEnv"
        >
          未定义变量：{{ state.undefinedVars.join('、') }}
        </el-tag>
      </el-tooltip>
      <div class="meta-right">
        <el-button text size="small" :icon="Upload" @click="curlDialog = true">导入 cURL</el-button>
        <el-dropdown trigger="click" @command="copyAsCode">
          <el-button text size="small" :icon="MagicStick">复制为代码</el-button>
          <template #dropdown>
            <el-dropdown-menu>
              <el-dropdown-item command="curl">cURL</el-dropdown-item>
              <el-dropdown-item command="fetch">fetch</el-dropdown-item>
              <el-dropdown-item command="axios">axios</el-dropdown-item>
            </el-dropdown-menu>
          </template>
        </el-dropdown>
      </div>
    </div>

    <el-tabs v-model="state.requestTab" class="req-tabs">
      <el-tab-pane name="params">
        <template #label>
          Params<span v-if="paramsCount()" class="tab-count">({{ paramsCount() }})</span>
        </template>
        <div class="tab-body">
          <div class="tab-tip">
            <span class="hint">查询参数将拼接到 URL 的 ? 之后</span>
            <el-button text type="primary" size="small" @click="importParamsFromUrl">
              从 URL 导入
            </el-button>
          </div>
          <KeyValueTable
            v-model="state.config.params"
            key-label="参数名"
            value-label="值"
            key-placeholder="参数名"
            value-placeholder="值"
          />
        </div>
      </el-tab-pane>

      <el-tab-pane name="headers">
        <template #label>
          Headers<span v-if="headersCount()" class="tab-count">({{ headersCount() }})</span>
        </template>
        <div class="tab-body">
          <div class="tab-tip">
            <span class="hint">自定义请求头，同名以最后一条为准</span>
          </div>
          <KeyValueTable
            v-model="state.config.headers"
            key-label="Header"
            value-label="值"
            key-placeholder="选择或输入 Header"
            value-placeholder="值"
            :key-options="COMMON_HEADERS"
          />
        </div>
      </el-tab-pane>

      <el-tab-pane label="Body" name="body">
        <template #label>
          Body<span v-if="jsonError" class="tab-dot" title="JSON 格式有误" />
        </template>
        <div class="tab-body">
          <div class="tab-tip">
            <el-radio-group v-model="state.config.body_type" size="small">
              <el-radio-button v-for="bt in BODY_TYPES" :key="bt" :value="bt">
                {{ BODY_TYPE_LABEL[bt] }}
              </el-radio-button>
            </el-radio-group>
            <el-button
              v-if="state.config.body_type === 'Json'"
              text
              size="small"
              :icon="MagicStick"
              @click="formatBody"
            >
              格式化
            </el-button>
          </div>

          <div v-if="state.config.body_type === 'None'" class="body-empty">
            <el-icon class="body-empty__icon" :size="22"><DocumentDelete /></el-icon>
            <span class="body-empty__title">本次请求不携带 Body</span>
            <span class="body-empty__hint">需要时在上方切换到 JSON、Text、Form 等类型</span>
          </div>

          <template
            v-else-if="state.config.body_type === 'Json' || state.config.body_type === 'Text'"
          >
            <el-input
              v-model="state.config.body"
              type="textarea"
              :rows="9"
              resize="vertical"
              class="mono"
              :placeholder="
                state.config.body_type === 'Json' ? '{\n  \'key\': \'value\'\n}' : '纯文本内容'
              "
            />
            <el-alert
              v-if="jsonError"
              type="error"
              :closable="false"
              show-icon
              :title="`JSON 格式有误：${jsonError}`"
              style="margin-top: 8px"
            />
          </template>

          <template v-else-if="state.config.body_type === 'Form'">
            <span class="hint">application/x-www-form-urlencoded 键值对</span>
            <KeyValueTable
              v-model="state.config.form_fields"
              key-label="字段"
              value-label="值"
              key-placeholder="字段"
              value-placeholder="值"
            />
          </template>

          <template v-else>
            <span class="hint">multipart/form-data：表单字段 + 文件（也可以直接把文件拖进窗口）</span>
            <KeyValueTable
              v-model="state.config.form_fields"
              key-label="字段"
              value-label="值"
              key-placeholder="字段"
              value-placeholder="值"
            />

            <div class="file-block">
              <div class="side-row">
                <span class="hint">文件字段名</span>
                <el-input
                  v-model="state.config.file_field"
                  size="small"
                  style="width: 150px"
                  placeholder="file"
                />
                <el-button size="small" :icon="FolderOpened" @click="chooseFiles">添加文件</el-button>
                <span v-if="files.length" class="hint">已选 {{ files.length }} 个</span>
              </div>
              <div
                v-for="(path, index) in files"
                :key="`${path}-${index}`"
                class="side-row file-row"
              >
                <span class="mono file-name" :title="path">{{ baseName(path) }}</span>
                <el-button text size="small" :icon="Delete" @click="removeFile(index)" />
              </div>
            </div>
          </template>
        </div>
      </el-tab-pane>

      <el-tab-pane label="Auth" name="auth">
        <div class="tab-body">
          <div class="tab-tip">
            <span class="hint">认证信息会自动附加到请求头</span>
          </div>
          <el-form label-width="92px" label-position="left" size="small">
            <el-form-item label="认证方式">
              <el-select v-model="state.config.auth.auth_type" style="width: 220px">
                <el-option
                  v-for="at in AUTH_TYPES"
                  :key="at"
                  :label="AUTH_TYPE_LABEL[at]"
                  :value="at"
                />
              </el-select>
            </el-form-item>
            <template v-if="state.config.auth.auth_type === 'Basic'">
              <el-form-item label="用户名">
                <el-input v-model="state.config.auth.username" style="width: 320px" />
              </el-form-item>
              <el-form-item label="密码">
                <el-input
                  v-model="state.config.auth.password"
                  type="password"
                  show-password
                  style="width: 320px"
                />
              </el-form-item>
            </template>
            <el-form-item v-else-if="state.config.auth.auth_type === 'Bearer'" label="Token">
              <el-input
                v-model="state.config.auth.token"
                placeholder="Bearer Token"
                style="width: 420px; max-width: 100%"
              />
            </el-form-item>
            <template v-else-if="state.config.auth.auth_type === 'CustomHeader'">
              <el-form-item label="Header 名">
                <el-input
                  v-model="state.config.auth.header_name"
                  placeholder="如 X-API-Key"
                  style="width: 320px"
                />
              </el-form-item>
              <el-form-item label="Header 值">
                <el-input v-model="state.config.auth.header_value" style="width: 320px" />
              </el-form-item>
            </template>
            <el-form-item v-else label=" ">
              <span class="hint">未启用认证，选择认证方式后将自动附加到请求头</span>
            </el-form-item>
          </el-form>
        </div>
      </el-tab-pane>

      <el-tab-pane label="高级" name="advanced">
        <div class="tab-body">
          <el-form label-width="96px" label-position="left" size="small">
            <el-form-item label="超时（秒）">
              <el-input-number v-model="state.config.timeout_secs" :min="0" :max="600" />
              <span class="hint" style="margin-left: 8px">0 表示默认 30 秒</span>
            </el-form-item>
            <el-form-item label="证书校验">
              <el-switch
                v-model="state.config.insecure_skip_verify"
                active-text="跳过 TLS 证书校验（自签名调试）"
              />
            </el-form-item>
            <el-form-item label="重定向">
              <el-switch v-model="state.config.follow_redirects" active-text="自动跟随 3xx 重定向" />
            </el-form-item>
            <el-form-item label="代理">
              <div style="width: 100%">
                <el-switch v-model="state.config.use_custom_proxy" active-text="使用自定义代理" />
                <el-input
                  v-if="state.config.use_custom_proxy"
                  v-model="state.config.proxy"
                  placeholder="http://127.0.0.1:7890"
                  style="max-width: 360px; margin-top: 6px"
                />
                <div v-else class="hint" style="margin-top: 4px">默认跟随系统代理设置</div>
              </div>
            </el-form-item>
            <el-form-item label="Cookie">
              <div style="width: 100%">
                <span class="hint">本次运行内自动保存并回带 Cookie（响应里的 Set-Cookie）</span>
                <el-button text type="danger" size="small" style="margin-left: 8px" @click="clearCookies">
                  清除 Cookie
                </el-button>
              </div>
            </el-form-item>
          </el-form>
        </div>
      </el-tab-pane>
    </el-tabs>

    <el-dialog v-model="curlDialog" title="导入 cURL" width="620px" append-to-body>
      <el-alert
        type="info"
        :closable="false"
        show-icon
        title="粘贴 curl 命令，自动解析为请求配置"
        style="margin-bottom: 10px"
      />
      <el-input
        v-model="curlText"
        type="textarea"
        :rows="8"
        class="mono"
        placeholder="curl -X POST 'https://api.example.com' -H 'Content-Type: application/json' -d '{&quot;a&quot;:1}'"
      />
      <template #footer>
        <el-button @click="curlDialog = false">取消</el-button>
        <el-button type="primary" @click="submitCurl">导入</el-button>
      </template>
    </el-dialog>
  </div>
</template>
