<script setup lang="ts">
import { computed } from 'vue'

interface Token {
  text: string
  cls: string
}

const props = defineProps<{ text: string }>()

/** 轻量 JSON 词法着色：输入已由后端美化为合法 JSON */
function tokenize(src: string): Token[] {
  const out: Token[] = []
  let i = 0
  const push = (text: string, cls: string) => {
    if (text) out.push({ text, cls })
  }

  while (i < src.length) {
    const ch = src[i]

    if (ch === '"') {
      let j = i + 1
      while (j < src.length) {
        if (src[j] === '\\') {
          j += 2
          continue
        }
        if (src[j] === '"') {
          j += 1
          break
        }
        j += 1
      }
      let k = j
      while (k < src.length && /\s/.test(src[k])) k += 1
      push(src.slice(i, j), src[k] === ':' ? 'json-key' : 'json-str')
      i = j
      continue
    }

    if (ch === '-' || (ch >= '0' && ch <= '9')) {
      const m = /^-?\d+(\.\d+)?([eE][+-]?\d+)?/.exec(src.slice(i))
      if (m) {
        push(m[0], 'json-num')
        i += m[0].length
        continue
      }
    }

    if (src.startsWith('true', i) || src.startsWith('false', i) || src.startsWith('null', i)) {
      const word = src.startsWith('true', i) ? 'true' : src.startsWith('false', i) ? 'false' : 'null'
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

const tokens = computed(() => tokenize(props.text))
</script>

<template>
  <pre class="json-view"><span
    v-for="(token, index) in tokens"
    :key="index"
    :class="token.cls"
  >{{ token.text }}</span></pre>
</template>
