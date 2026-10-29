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
    /** 提供后 Key 列渲染为可搜索、可自定义的下拉框 */
    keyOptions?: string[]
  }>(),
  {
    keyPlaceholder: 'key',
    valuePlaceholder: 'value',
    keyLabel: 'Key',
    valueLabel: 'Value',
    addText: '添加一行',
    keyOptions: () => [],
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
    <div class="kv-toolbar">
      <el-button text type="primary" size="small" :icon="Plus" @click="addRow">{{ addText }}</el-button>
    </div>

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
          <el-select
            v-if="keyOptions.length"
            v-model="row.key"
            size="small"
            style="width: 100%"
            filterable
            allow-create
            default-first-option
            :reserve-keyword="false"
            :placeholder="keyPlaceholder"
          >
            <el-option v-for="option in keyOptions" :key="option" :label="option" :value="option" />
          </el-select>
          <el-input v-else v-model="row.key" size="small" :placeholder="keyPlaceholder" />
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
  </div>
</template>
