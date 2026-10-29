<script setup lang="ts">
import { Menu, Setting } from '@element-plus/icons-vue'
import zhCn from 'element-plus/es/locale/lang/zh-cn'
import { onBeforeUnmount, onMounted, ref } from 'vue'

import RequestPanel from '@/components/RequestPanel.vue'
import ResponsePanel from '@/components/ResponsePanel.vue'
import SidePanel from '@/components/SidePanel.vue'
import { init, loadEnv, openSaveDialog, sendRequest, state } from '@/store'

const drawerOpen = ref(false)
const narrow = ref(false)
/** 请求区占比（可拖拽调整，持久化到本地） */
const ratio = ref(Number(localStorage.getItem('pane-ratio') ?? 0.47))
const dragging = ref(false)

function syncNarrow() {
  narrow.value = window.innerWidth < 900
  if (!narrow.value) drawerOpen.value = false
}

function onPointerMove(event: PointerEvent) {
  if (!dragging.value) return
  const main = document.querySelector('.app-main')
  if (!(main instanceof HTMLElement)) return
  const rect = main.getBoundingClientRect()
  const next = (event.clientY - rect.top) / rect.height
  ratio.value = Math.min(0.74, Math.max(0.24, next))
}

function stopDrag() {
  if (!dragging.value) return
  dragging.value = false
  localStorage.setItem('pane-ratio', ratio.value.toFixed(3))
  window.removeEventListener('pointermove', onPointerMove)
  window.removeEventListener('pointerup', stopDrag)
}

function startDrag(event: PointerEvent) {
  event.preventDefault()
  dragging.value = true
  window.addEventListener('pointermove', onPointerMove)
  window.addEventListener('pointerup', stopDrag)
}

function handleKeydown(e: KeyboardEvent) {
  if (!(e.ctrlKey || e.metaKey)) return
  if (e.key === 'Enter') {
    e.preventDefault()
    sendRequest()
  } else if (e.key.toLowerCase() === 's') {
    e.preventDefault()
    openSaveDialog()
  }
}

onMounted(async () => {
  syncNarrow()
  window.addEventListener('resize', syncNarrow)
  window.addEventListener('keydown', handleKeydown)
  await init()
})

onBeforeUnmount(() => {
  window.removeEventListener('resize', syncNarrow)
  window.removeEventListener('keydown', handleKeydown)
  window.removeEventListener('pointermove', onPointerMove)
  window.removeEventListener('pointerup', stopDrag)
})
</script>

<template>
  <el-config-provider :locale="zhCn">
    <div class="app-shell">
      <header class="app-header">
        <div class="header-actions">
          <el-select
            :model-value="state.currentEnv"
            class="env-select"
            size="default"
            @change="loadEnv"
          >
            <template #prefix>
              <el-icon><Setting /></el-icon>
            </template>
            <el-option v-for="name in state.envNames" :key="name" :label="name" :value="name" />
          </el-select>
          <el-button
            text
            :icon="Menu"
            @click="narrow ? (drawerOpen = true) : (state.sidebarVisible = !state.sidebarVisible)"
          >
            {{ state.sidebarVisible && !narrow ? '收起侧栏' : '侧栏' }}
          </el-button>
        </div>
      </header>

      <div class="app-body">
        <aside v-if="state.sidebarVisible && !narrow" class="app-side">
          <SidePanel />
        </aside>

        <main class="app-main">
          <section
            class="pane pane-request"
            :style="!narrow ? { flex: `0 0 calc(${(ratio * 100).toFixed(2)}% - 10px)` } : undefined"
          >
            <RequestPanel />
          </section>

          <div
            v-if="!narrow"
            class="splitter"
            :class="{ dragging }"
            title="拖拽调整请求区与响应区高度"
            @pointerdown="startDrag"
          />

          <section class="pane pane-response">
            <ResponsePanel />
          </section>
        </main>
      </div>

      <el-drawer v-model="drawerOpen" direction="ltr" size="300px" :with-header="false">
        <SidePanel />
      </el-drawer>
    </div>
  </el-config-provider>
</template>
