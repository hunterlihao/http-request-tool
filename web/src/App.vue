<script setup lang="ts">
import { Menu, Setting } from '@element-plus/icons-vue'
import { onBeforeUnmount, onMounted, ref } from 'vue'

import RequestPanel from '@/components/RequestPanel.vue'
import ResponsePanel from '@/components/ResponsePanel.vue'
import SidePanel from '@/components/SidePanel.vue'
import { init, loadEnv, sendRequest, state } from '@/store'

const drawerOpen = ref(false)
const narrow = ref(false)

function syncNarrow() {
  narrow.value = window.innerWidth < 900
  if (!narrow.value) drawerOpen.value = false
}

function handleKeydown(e: KeyboardEvent) {
  if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
    e.preventDefault()
    sendRequest()
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
})
</script>

<template>
  <div class="app-shell">
    <header class="app-header">
      <div class="brand">
        <svg width="22" height="22" viewBox="0 0 64 64" aria-hidden="true">
          <defs>
            <linearGradient id="brandGrad" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stop-color="#FB923C" />
              <stop offset="100%" stop-color="#C23708" />
            </linearGradient>
          </defs>
          <rect x="1" y="1" width="62" height="62" rx="16" fill="url(#brandGrad)" />
          <polygon
            points="37.4,8.3 17.9,35.8 29.8,35.8 25.9,56 46.1,27.8 33.9,27.8"
            fill="#ffffff"
          />
        </svg>
        <span class="brand-name">HTTP 请求工具</span>
        <el-tag size="small" type="info" effect="plain">便携版</el-tag>
      </div>

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
        <el-button text :icon="Menu" @click="narrow ? (drawerOpen = true) : (state.sidebarVisible = !state.sidebarVisible)">
          {{ state.sidebarVisible && !narrow ? '收起侧栏' : '侧栏' }}
        </el-button>
      </div>
    </header>

    <div class="app-body">
      <aside v-if="state.sidebarVisible && !narrow" class="app-side">
        <SidePanel />
      </aside>

      <main class="app-main">
        <section class="pane pane-request">
          <RequestPanel />
        </section>
        <section class="pane pane-response">
          <ResponsePanel />
        </section>
      </main>
    </div>

    <el-drawer v-model="drawerOpen" direction="ltr" size="300px" :with-header="false">
      <SidePanel />
    </el-drawer>
  </div>
</template>
