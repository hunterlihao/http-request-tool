// 宣传片素材：真实前端 + 真实请求，按真实交互路径截 4 张 2x 图
import puppeteer from 'puppeteer-core'
const CHROME = 'C:/Program Files/Google/Chrome/Application/chrome.exe'
const URL0 = process.env.URL0 || 'http://localhost:5199/'
const OUT = 'docs/promo/shots'
const wait = (ms) => new Promise((r) => setTimeout(r, ms))

const browser = await puppeteer.launch({
  executablePath: CHROME, headless: 'new',
  defaultViewport: { width: 1440, height: 900, deviceScaleFactor: 2 },
  args: ['--hide-scrollbars', '--force-device-scale-factor=2'],
})
const page = await browser.newPage()
await page.goto(URL0, { waitUntil: 'networkidle2' })
await wait(1000)

const input = await page.$('input[placeholder*="api.example.com"]')
await input.click({ clickCount: 3 })
await input.type('https://jsonplaceholder.typicode.com/todos/1', { delay: 10 })
await page.keyboard.down('Control'); await page.keyboard.press('Enter'); await page.keyboard.up('Control')
await wait(1500)
await page.screenshot({ path: `${OUT}/response.png` })

// Headers 页签
const tabs = await page.$$('.el-tabs__item')
const texts = await page.$$eval('.el-tabs__item', (e) => e.map((x) => x.textContent?.trim()))
const hi = texts.indexOf('Headers')
if (hi >= 0) { await tabs[hi].click(); await wait(400); await page.screenshot({ path: `${OUT}/headers.png` }) }

// 保存到集合（Ctrl+S）→ 弹窗
await page.keyboard.down('Control'); await page.keyboard.press('KeyS'); await page.keyboard.up('Control')
await wait(900)
await page.screenshot({ path: `${OUT}/save.png` })
const title = await page.title()
console.log('tabs:', texts.join('|'), 'title:', title)
await browser.close()