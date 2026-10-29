import { createApp } from 'vue'

// 组件样式由 unplugin-vue-components 按需注入；ElMessage / ElMessageBox 是函数式调用，需手动引入样式
import 'element-plus/es/components/message/style/css'
import 'element-plus/es/components/message-box/style/css'
import './styles.css'
import App from './App.vue'

createApp(App).mount('#app')
