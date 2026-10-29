<script setup lang="ts">
import { Delete, Plus } from '@element-plus/icons-vue'

import type { KVPair } from '@/api/types'

withDefaults(
  defineProps<{
    keyPlaceholder?: string
    valuePlaceholder?: string
    keyLabel?: string
    valueLabel?: string
    addText?: string
  }>(),
  {
    keyPlaceholder: 'key',
    valuePlaceholder: 'value',
    keyLabel: 'Key',
    valueLabel: 'Value',
    addText: '添加一行',
  },
)

const rows = defineModel<KVPair[]>({ required: true })

function addRow() {
  rows.value.push({ key: '', value: '', enabled: true })
}

function removeRow(index: number) {
  rows.value.splice(index, 1)
}
</script>

<template>
  <div class="kv-editor">
    <el-table :data="rows" size="small" style="width: 100%" empty-text="暂无内容">
      <el-table-column width="48" align="center">
        <template #header>
          <span class="hint">启用</span>
        </template>
        <template #default="{ row }">
          <el-checkbox v-model="row.enabled" />
        </template>
      </el-table-column>
      <el-table-column :label="keyLabel" min-width="160">
        <template #default="{ row }">
          <el-input v-model="row.key" size="small" :placeholder="keyPlaceholder" />
        </template>
      </el-table-column>
      <el-table-column :label="valueLabel" min-width="200">
        <template #default="{ row }">
          <el-input v-model="row.value" size="small" :placeholder="valuePlaceholder" />
        </template>
      </el-table-column>
      <el-table-column width="52" align="center">
        <template #default="{ $index }">
          <el-button
            text
            size="small"
            :icon="Delete"
            @click="removeRow($index)"
          />
        </template>
      </el-table-column>
    </el-table>

    <div class="kv-actions">
      <el-button text type="primary" :icon="Plus" @click="addRow">{{ addText }}</el-button>
    </div>
  </div>
</template>
