const { app, BrowserWindow, dialog, Menu } = require('electron')
const { autoUpdater } = require('electron-updater')
const { spawn } = require('child_process')
const fs = require('fs')
const path = require('path')

const PORT = 3080
const HOST = '127.0.0.1'
let coreProcess
let mainWindow

function logError(error) {
  const logPath = path.join(app.getPath('userData'), 'startup-error.txt')
  fs.mkdirSync(path.dirname(logPath), { recursive: true })
  fs.writeFileSync(logPath, String(error && error.stack ? error.stack : error))
}

function corePath() {
  const installed = path.join(process.resourcesPath, 'r', 'harness-core')
  if (fs.existsSync(path.join(installed, 'node.exe')) && fs.existsSync(path.join(installed, 'harness', 'lib', 'bin.js'))) return installed
  throw new Error(`Installed Harness runtime is missing: ${installed}`)
}

function waitForLaunchUrl(child, timeout = 180000) {
  return new Promise((resolve, reject) => {
    let stdout = ''
    let stderr = ''
    let settled = false

    const finish = (callback, value) => {
      if (settled) return
      settled = true
      clearTimeout(timer)
      callback(value)
    }

    const timer = setTimeout(() => {
      const detail = stderr.trim() || stdout.trim() || 'no output from embedded Harness'
      finish(reject, new Error(`Embedded Harness did not publish its authenticated URL within ${Math.round(timeout / 1000)} seconds.\n${detail}`))
    }, timeout)

    child.stdout.on('data', (chunk) => {
      stdout = (stdout + chunk.toString()).slice(-65536)
      const match = stdout.match(/dsh web:\s+(http:\/\/[^\s]+)/)
      if (!match) return

      try {
        const launchUrl = new URL(match[1])
        if (launchUrl.hostname !== HOST || launchUrl.port !== String(PORT) || !launchUrl.searchParams.has('token')) return
        finish(resolve, launchUrl.toString())
      } catch {
        // Keep reading until Harness prints a complete URL.
      }
    })

    child.stderr.on('data', (chunk) => {
      stderr = (stderr + chunk.toString()).slice(-65536)
    })

    child.once('error', (error) => finish(reject, error))
    child.once('exit', (code, signal) => {
      const detail = stderr.trim() || stdout.trim()
      finish(reject, new Error(`Embedded Harness exited before startup (code=${code}, signal=${signal}).${detail ? `\n${detail}` : ''}`))
    })
  })
}

function checkForUpdates() {
  return autoUpdater.checkForUpdates().then((result) => {
    if (!result || !result.updateInfo || result.updateInfo.version === app.getVersion()) {
      dialog.showMessageBox(mainWindow, { type: 'info', title: '检查更新', message: '当前已经是最新版本。' })
    }
  }).catch((error) => dialog.showErrorBox('检查更新失败', error.message || String(error)))
}

function configureUpdater() {
  autoUpdater.autoDownload = false
  autoUpdater.on('update-available', async (info) => {
    const result = await dialog.showMessageBox(mainWindow, { type: 'info', title: '发现新版本', message: `发现 DeepSeek Harness ${info.version}，是否下载？`, buttons: ['下载并安装', '稍后'] })
    if (result.response === 0) await autoUpdater.downloadUpdate()
  })
  autoUpdater.on('update-downloaded', async () => {
    const result = await dialog.showMessageBox(mainWindow, { type: 'info', title: '更新已下载', message: '更新已下载完成，重启后安装。', buttons: ['立即重启', '稍后'] })
    if (result.response === 0) autoUpdater.quitAndInstall()
  })
}

function createMenu() {
  Menu.setApplicationMenu(Menu.buildFromTemplate([{ label: 'DeepSeek Harness', submenu: [
    { label: '检查更新…', click: checkForUpdates },
    { type: 'separator' },
    { role: 'quit', label: '退出' }
  ] }]))
}

async function startCore() {
  const core = corePath()
  const node = path.join(core, 'node.exe')
  const cli = path.join(core, 'harness', 'lib', 'bin.js')
  coreProcess = spawn(node, [cli, 'web', '--host', HOST, '--port', String(PORT), '--no-open'], {
    cwd: path.join(core, 'harness'),
    windowsHide: true,
    stdio: ['ignore', 'pipe', 'pipe']
  })
  const launchUrl = await waitForLaunchUrl(coreProcess)
  await mainWindow.loadURL(launchUrl)
}

app.whenReady().then(async () => {
  mainWindow = new BrowserWindow({ width: 1440, height: 900, minWidth: 960, minHeight: 640, title: 'DeepSeek Harness' })
  await mainWindow.loadFile(path.join(__dirname, 'loader', 'index.html'))
  createMenu()
  configureUpdater()
  try {
    await startCore()
    setTimeout(() => { void checkForUpdates() }, 10000)
  } catch (error) {
    logError(error)
    dialog.showErrorBox('DeepSeek Harness 启动失败', String(error.message || error))
    app.quit()
  }
})

app.on('window-all-closed', () => { if (process.platform !== 'darwin') app.quit() })
app.on('before-quit', () => { if (coreProcess && !coreProcess.killed) coreProcess.kill() })
