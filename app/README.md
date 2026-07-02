# Treant Arcade — native app (Capacitor)

Packages the Treant Arcade (the `/arcade` route of the docs site) as an
**offline** Android and iOS app. The web assets are bundled into the binary; the
app never makes a network request. Analytics and the PWA service worker are
stripped from this build, so the store "Data safety / privacy" answer is
genuinely **no data collected**.

- **appId:** `dev.mcts.arcade`
- **appName:** `Treant Arcade`
- **Engine:** Capacitor 7, wrapping the Docusaurus static build
- **Web root:** `www/` (generated — gitignored)

The app opens straight into the arcade: `index.tsx` redirects to `/arcade` when
it detects the Capacitor bridge, the Docusaurus navbar/footer are hidden
(`body.capacitor-app` CSS), the status bar is tinted to the arcade backdrop, and
the Android hardware back button walks the arcade's own history (exiting only at
the launcher). None of this affects the web build — every hook is feature-detected
off `window.Capacitor`, which is absent in a browser.

## Layout

```
app/
  package.json          # Capacitor deps + build scripts
  capacitor.config.ts   # appId/appName/webDir + SplashScreen config
  build-www.sh          # build docs in app-mode → www/ (with strip verification)
  resources/            # icon + splash source art (committed)
  android/              # generated Android Studio project (committed skeleton)
  ios/                  # generated Xcode project (committed skeleton)
  www/                  # built web assets (generated, gitignored)
  node_modules/         # (gitignored)
  README.md  SIGNING.md
```

## Build & run (developer workflow)

```bash
cd app
npm install                 # first time only

./build-www.sh              # ARCADE_APP_BUILD=1 docs build → www/, verified
                            # analytics-free and service-worker-free
npx cap sync                # copy www/ into android/ and ios/, update plugins

npx cap open android        # opens Android Studio  (needs Android SDK)
npx cap open ios            # opens Xcode           (needs macOS + Xcode)
```

`cap sync android` works on this box. `cap sync ios` runs but **skips
`pod install`** (CocoaPods/Xcode are macOS-only) — that step must run on a Mac.

Whenever the arcade changes, re-run `./build-www.sh && npx cap sync` to refresh
the bundled assets.

## App icons & splash

Source art lives in `resources/` (`icon.svg` is the master; the PNGs are
rasterized from it — neon treant-face `#ff3b5c` on the arcade backdrop `#1c1830`).
Regenerate every platform's icon/splash set with:

```bash
cd app
npx capacitor-assets generate \
  --iconBackgroundColor '#1c1830' --iconBackgroundColorDark '#1c1830' \
  --splashBackgroundColor '#1c1830' --splashBackgroundColorDark '#1c1830'
```

## Signing

See [`SIGNING.md`](SIGNING.md). The Android upload keystore is on Peter's box at
`~/.config/homenet/treant-arcade.jks` (credentials in the sibling `.env`, mode
`0600`) — **not** in the repo. Back both up; losing the upload key blocks future
Play updates.

---

## Android build status — DONE (only Play upload remains)

The Android SDK is installed on this box (`~/Android/Sdk`) and a **signed release
AAB has been built locally**. Artifacts live outside the repo in
`~/treant-arcade-artifacts/`:

| Artifact | Size | Signing | Purpose |
| --- | --- | --- | --- |
| `app-release.aab` | 4.3 MB | arcade upload key (`CN=Treant Arcade`, self-signed) | Play Console upload |
| `app-debug.apk` | 5.8 MB | Android debug key | sideload / test |

The only Android step left is uploading `app-release.aab` to the Play Console
(needs the $25 account below). Rebuild instructions are in [`SIGNING.md`](SIGNING.md).

### Still blocked on Peter (can't be done from this box)

| Blocker | Needed for | Cost / notes |
| --- | --- | --- |
| **Google Play Console account** | Publishing to Play | One-time **$25** |
| **Apple Developer Program** | Building/publishing iOS | **$99/yr** |
| **A Mac with Xcode** | iOS `pod install`, build, archive, upload | macOS-only; this box is Linux |

### How the Android SDK was installed (for reference / a fresh box)

```bash
# command-line tools only (no full Android Studio) is enough for CI-style builds
mkdir -p ~/Android/Sdk/cmdline-tools
# download commandlinetools-linux-*.zip from developer.android.com, unzip to latest/
export ANDROID_HOME=~/Android/Sdk
export PATH="$ANDROID_HOME/cmdline-tools/latest/bin:$ANDROID_HOME/platform-tools:$PATH"
yes | sdkmanager --licenses
# match android/variables.gradle (compileSdkVersion 35):
sdkmanager "platform-tools" "platforms;android-35" "build-tools;35.0.0"
# then write android/local.properties with: sdk.dir=/home/peter/Android/Sdk  (gitignored)
```

Then follow [`SIGNING.md`](SIGNING.md) to produce a signed AAB.

---

## Store-listing checklist

Content is identical for both stores unless noted.

**Metadata**
- Name: `Treant Arcade`
- Short description: family board-game arcade; every opponent is a Monte Carlo
  Tree Search AI (the `treant` library) thinking in real time.
- Category: Games → Board (Play) / Games → Board (App Store)
- Content rating: **Everyone (Play) / 4+ (App Store)** — no objectionable content
- **Data safety (Play) / Privacy (App Store): NO data collected** — true for this
  build (analytics stripped, offline, no accounts, no network). Verified by
  `build-www.sh`.
- Price: Free. No ads, no in-app purchases.

**Graphics assets to produce (not generated here — need real screenshots)**
- App icon: 512×512 (Play) — derive from `resources/icon-1024.png`
- Feature graphic (Play): **1024×500**
- Phone screenshots: at least 2; Play wants 1080×1920-ish (16:9/9:16), App Store
  wants 6.7" (1290×2796) and 6.5" (1242×2688) sets. Capture the launcher + a
  game mid-play + a win screen.
- (Optional) 7" / 10" tablet screenshots for Play tablet listing.

**Legal**
- Privacy policy URL: required by both stores even when nothing is collected.
  A one-line "this app collects no data and makes no network requests" page on
  mcts.dev suffices.
