<script setup lang="ts">
import { computed, ref, watch } from 'vue'

interface Segment {
  text: string
  cls: string
  hit: boolean
}

interface Line {
  index: number
  segments: Segment[]
  /** 该行以 { 或 [ 结尾，可以折叠 */
  foldEnd: number | null
  /** 折叠状态（仅 foldEnd 非空时有意义） */
  folded: boolean
  depth: number
}

const props = withDefaults(
  defineProps<{
    text: string
    /** 搜索关键词，命中的片段会高亮 */
    query?: string
  }>(),
  { query: '' },
)

/** 轻量 JSON 词法着色：输入已由后端美化为合法 JSON */
function tokenize(line: string): Segment[] {
  const out: Segment[] = []
  let i = 0
  const push = (text: string, cls: string) => {
    if (text) out.push({ text, cls, hit: false })
  }

  while (i < line.length) {
    const ch = line[i]

    if (ch === '"') {
      let j = i + 1
      while (j < line.length) {
        if (line[j] === '\\') {
          j += 2
          continue
        }
        if (line[j] === '"') {
          j += 1
          break
        }
        j += 1
      }
      let k = j
      while (k < line.length && /\s/.test(line[k])) k += 1
      push(line.slice(i, j), line[k] === ':' ? 'json-key' : 'json-str')
      i = j
      continue
    }

    if (ch === '-' || (ch >= '0' && ch <= '9')) {
      const m = /^-?\d+(\.\d+)?([eE][+-]?\d+)?/.exec(line.slice(i))
      if (m) {
        push(m[0], 'json-num')
        i += m[0].length
        continue
      }
    }

    if (line.startsWith('true', i) || line.startsWith('false', i) || line.startsWith('null', i)) {
      const word = line.startsWith('true', i) ? 'true' : line.startsWith('false', i) ? 'false' : 'null'
      push(word, 'json-lit')
      i += word.length
      continue
    }

    if ('{}[],:'.includes(ch)) {
      push(ch, 'json-punc')
      i += 1
      continue
    }

    push(ch, '')
    i += 1
  }
  return out
}

/** 在片段内按关键词切分，命中的部分单独标记 */
function markHits(segments: Segment[], query: string): Segment[] {
  const q = query.trim().toLowerCase()
  if (!q) return segments
  const out: Segment[] = []
  for (const seg of segments) {
    const lower = seg.text.toLowerCase()
    let from = 0
    while (from < seg.text.length) {
      const at = lower.indexOf(q, from)
      if (at < 0) break
      if (at > from) out.push({ text: seg.text.slice(from, at), cls: seg.cls, hit: false })
      out.push({ text: seg.text.slice(at, at + q.length), cls: seg.cls, hit: true })
      from = at + q.length
    }
    if (from < seg.text.length) out.push({ text: seg.text.slice(from), cls: seg.cls, hit: false })
  }
  return out
}

const collapsed = ref(new Set<number>())

// 换了一份响应后，旧的折叠位置不再有意义
watch(
  () => props.text,
  () => {
    collapsed.value = new Set()
  },
)

const rawLines = computed(() => props.text.split('\n'))

/** 预计算每行的缩进与折叠范围（按缩进配对 { 与 }） */
const lines = computed<Line[]>(() => {
  const src = rawLines.value
  const depthOf = (line: string) => line.length - line.trimStart().length
  const ends: (number | null)[] = src.map(() => null)

  for (let i = 0; i < src.length; i += 1) {
    const trimmed = src[i].trimEnd()
    if (!trimmed.endsWith('{') && !trimmed.endsWith('[')) continue
    const depth = depthOf(src[i])
    for (let j = i + 1; j < src.length; j += 1) {
      const t = src[j].trim()
      if (!t) continue
      if (depthOf(src[j]) <= depth && (t.startsWith('}') || t.startsWith(']'))) {
        ends[i] = j
        break
      }
    }
  }

  return src.map((line, index) => ({
    index,
    segments: markHits(tokenize(line), props.query),
    foldEnd: ends[index],
    folded: collapsed.value.has(index),
    depth: depthOf(line),
  }))
})

/** 折叠后实际渲染的行 */
const visibleLines = computed(() => {
  const out: Line[] = []
  let skipUntil = -1
  for (const line of lines.value) {
    if (line.index <= skipUntil) continue
    out.push(line)
    if (line.foldEnd != null && line.folded) skipUntil = line.foldEnd
  }
  return out
})

const hitCount = computed(() => {
  const q = props.query.trim().toLowerCase()
  if (!q) return 0
  return props.text.toLowerCase().split(q).length - 1
})

function toggle(line: Line) {
  if (line.foldEnd == null) return
  const next = new Set(collapsed.value)
  if (next.has(line.index)) next.delete(line.index)
  else next.add(line.index)
  collapsed.value = next
}

function foldHint(line: Line): string {
  if (line.foldEnd == null) return ''
  const count = line.folded ? line.foldEnd - line.index - 1 : 0
  return line.folded ? ` ⋯ ${count} 行` : ''
}

defineExpose({ hitCount })
</script>

<template>
  <pre class="json-view"><div
    v-for="line in visibleLines"
    :key="line.index"
    class="json-line"
    :class="{ 'json-foldable': line.foldEnd != null }"
  ><span
    class="json-caret"
    @click="toggle(line)"
  >{{ line.foldEnd == null ? ' ' : line.folded ? '▸' : '▾' }}</span><span
    v-for="(seg, i) in line.segments"
    :key="i"
    :class="[seg.cls, { 'json-hit': seg.hit }]"
  >{{ seg.text }}</span><span v-if="line.folded" class="json-fold-hint">{{ foldHint(line) }}</span></div></pre>
</template>
