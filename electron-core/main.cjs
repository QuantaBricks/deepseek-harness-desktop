const { app, BrowserWindow, dialog, Menu } = require('electron')
const { autoUpdater } = require('electron-updater')
const { spawn } = require('child_process')
const { createConnection } = require('net')
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

function waitForServer(timeout = 30000) {
  return new Promise((resolve, reject) => {
    const started = Date.now()
    const probe = () => {
      const socket = createConnection({ host: HOST, port: PORT })
      socket.once('connect', () => { socket.destroy(); resolve() })
      socket.once('error', () => {
        socket.destroy()
        if (Date.now() - started >= timeout) reject(new Error('Embedded Harness did not become ready within 30 seconds'))
        else setTimeout(probe, 250)
      })
    }
    probe()
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
  coreProcess = spawn(node, [cli, 'web', '--host', HOST, '--port', String(PORT), '--no-open'], { cwd: path.join(core, 'harness'), windowsHide: true, stdio: 'ignore' })
  coreProcess.once('error', (error) => logError(error))
  await waitForServer()
  await mainWindow.loadURL(`http://${HOST}:${PORT}`)
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
