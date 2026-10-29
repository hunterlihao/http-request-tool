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
