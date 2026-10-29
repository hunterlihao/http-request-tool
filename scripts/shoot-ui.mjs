// 抓 README 用的界面截图：连本机 Chrome，走 vite dev（mock 后端），
// 按真实交互路径推进（填 URL → Ctrl+Enter 发送 → 切集合/环境）。
import puppeteer from 'puppeteer-core'

const CHROME = 'C:/Program Files/Google/Chrome/Application/chrome.exe'
const URL0 = process.env.URL0 || 'http://localhost:1420/'
const OUT = 'docs/images'

const browser = await puppeteer.launch({
  executablePath: CHROME,
  headless: 'new',
  defaultViewport: { width: 1440, height: 900, deviceScaleFactor: 2 },
  args: ['--hide-scrollbars', '--force-device-scale-factor=2'],
})
const page = await browser.newPage()
page.on('console', (m) => console.log('  [browser]', m.type(), m.text().slice(0, 200)))
page.on('pageerror', (e) => console.log('  [pageerror]', String(e).slice(0, 200)))
await page.goto(URL0, { waitUntil: 'networkidle2' })
await new Promise((r) => setTimeout(r, 1200))

// 1) 填一个真实形状的请求
const input = await page.$('input[placeholder*="api.example.com"]')
await input.click({ clickCount: 3 })
await input.type('https://jsonplaceholder.typicode.com/todos/1', { delay: 12 })
await page.keyboard.down('Control')
await page.keyboard.press('Enter')
await page.keyboard.up('Control')
await new Promise((r) => setTimeout(r, 1500))
await page.screenshot({ path: `${OUT}/request.png` })

// 2) Headers 页签
const heads = await page.$$eval('.el-tabs__item', (els) => els.map((e) => e.textContent?.trim()))
const i = heads.indexOf('Headers')
if (i >= 0) {
  await (await page.$$('.el-tabs__item'))[i].click()
  await new Promise((r) => setTimeout(r, 500))
  await page.screenshot({ path: `${OUT}/headers.png` })
}

// 3) 左栏切到「集合」
const side = await page.$$eval('.el-tabs__item', (els) => els.map((e) => e.textContent?.trim()))
console.log('页签:', heads, '侧栏:', side)
await browser.close()