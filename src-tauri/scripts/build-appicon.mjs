import { execFileSync } from 'node:child_process'
import { mkdirSync, rmSync, writeFileSync, statSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import process from 'node:process'

const root = join(dirname(fileURLToPath(import.meta.url)), '..')
const out = join(root, 'target', 'appicon')

const EMPTY_PLIST = `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict/>
</plist>
`

const ICON_PLIST = `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
\t<key>CFBundleIconName</key>
\t<string>AppIcon</string>
</dict>
</plist>
`

rmSync(out, { recursive: true, force: true })
mkdirSync(out, { recursive: true })
writeFileSync(join(out, 'Assets.car'), '')
writeFileSync(join(out, 'Info.plist'), EMPTY_PLIST)

if (process.platform !== 'darwin') {
	process.exit(0)
}

try {
	execFileSync('xcrun', ['--find', 'actool'], { stdio: 'ignore' })
} catch {
	console.log('packweave: actool not found (needs full Xcode) - using static app icon')
	process.exit(0)
}

try {
	execFileSync(
		'xcrun',
		[
			'actool',
			'AppIcon.xcassets',
			'--compile',
			out,
			'--app-icon',
			'AppIcon',
			'--minimum-deployment-target',
			'11.0',
			'--platform',
			'macosx',
			'--output-partial-info-plist',
			join(out, 'actool.plist'),
		],
		{ cwd: root, stdio: 'ignore' },
	)
} catch {}

let size = 0
try {
	size = statSync(join(out, 'Assets.car')).size
} catch {}

if (size > 0) {
	writeFileSync(join(out, 'Info.plist'), ICON_PLIST)
	console.log('packweave: compiled light/dark app icon')
} else {
	console.log('packweave: actool produced no icon data - using static app icon')
}
