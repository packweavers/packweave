const agent = typeof navigator === 'undefined' ? '' : navigator.userAgent

export const isMac = /Mac|iPhone|iPad/.test(agent)
const isWindows = /Windows/.test(agent)

export const fileManager = isMac ? 'Finder' : isWindows ? 'File Explorer' : 'Files'
