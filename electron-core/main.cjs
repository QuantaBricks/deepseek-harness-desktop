const { app, BrowserWindow, dialog } = require('electron')
const { spawn } = require('child_process')
const { createConnection } = require('net')
const fs = require('fs')
const path = require('path')
const extractZip = require('extract-zip')

const PORT = 3080
const HOST = '127.0.0.1'
let coreProcess

function logError(error) {
  const logPath = path.join(app.getPath('userData'), 'startup-error.txt')
  fs.mkdirSync(path.dirname(logPath), { recursive: true })
  fs.writeFileSync(logPath, String(error && error.stack ? error.stack : error))
}

function createJunctions(core) {
  const linksPath = path.join(core, 'links.json')
  if (!fs.existsSync(linksPath)) return
  const links = JSON.parse(fs.readFileSync(linksPath, 'utf8').replace(/^\uFEFF/, ''))
  for (const link of links) {
    const linkPath = path.join(core, link.path)
    const target = path.resolve(path.dirname(linkPath), link.target)
    if (!fs.existsSync(target)) throw new Error(`Embedded dependency target is missing: ${target}`)
    fs.mkdirSync(path.dirname(linkPath), { recursive: true })
    try {
      fs.rmSync(linkPath, { recursive: true, force: true })
    } catch {}
    fs.symlinkSync(target, linkPath, 'junction')
  }
}

async function prepareCore() {
  const userData = app.getPath('userData')
  const core = path.join(userData, 'harness-core')
  const node = path.join(core, 'node.exe')
  const cli = path.join(core, 'harness', 'lib', 'bin.js')
  if (fs.existsSync(node) && fs.existsSync(cli)) return core

  const archive = path.join(process.resourcesPath, 'r', 'harness-core.zip')
  if (!fs.existsSync(archive)) throw new Error(`Embedded runtime archive is missing: ${archive}`)
  const staging = `${core}.next`
  fs.rmSync(staging, { recursive: true, force: true })
  fs.mkdirSync(staging, { recursive: true })
  await extractZip(archive, { dir: staging })
  if (!fs.existsSync(path.join(staging, 'node.exe')) || !fs.existsSync(path.join(staging, 'harness', 'lib', 'bin.js'))) {
    throw new Error('Embedded Harness runtime is incomplete')
  }
  fs.rmSync(core, { recursive: true, force: true })
  fs.renameSync(staging, core)
  createJunctions(core)
  return core
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

async function start() {
  const core = await prepareCore()
  const node = path.join(core, 'node.exe')
  const cli = path.join(core, 'harness', 'lib', 'bin.js')
  coreProcess = spawn(node, [cli, 'web', '--host', HOST, '--port', String(PORT), '--no-open'], {
    cwd: path.join(core, 'harness'),
    windowsHide: true,
    stdio: 'ignore'
  })
  coreProcess.once('error', (error) => logError(error))
  await waitForServer()
  const win = new BrowserWindow({
    width: 1440,
    height: 900,
    minWidth: 960,
    minHeight: 640,
    title: 'DeepSeek Harness',
    webPreferences: { contextIsolation: true, sandbox: true }
  })
  await win.loadURL(`http://${HOST}:${PORT}`)
}

app.whenReady().then(async () => {
  try { await start() } catch (error) {
    logError(error)
    dialog.showErrorBox('DeepSeek Harness 启动失败', String(error.message || error))
    app.quit()
  }
})

app.on('window-all-closed', () => { if (process.platform !== 'darwin') app.quit() })
app.on('before-quit', () => { if (coreProcess && !coreProcess.killed) coreProcess.kill() })
