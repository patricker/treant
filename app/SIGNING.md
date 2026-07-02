# Android signing — Treant Arcade

Google Play requires every AAB to be signed with an **upload key**. This repo
does **not** contain the keystore or its password (they must never be committed).
They live on Peter's box, mode `0600`:

| What | Path |
| --- | --- |
| Keystore (JKS) | `~/.config/homenet/treant-arcade.jks` |
| Credentials (env) | `~/.config/homenet/treant-arcade-keystore.env` |

The keystore was generated once with:

```bash
keytool -genkeypair -v \
  -keystore ~/.config/homenet/treant-arcade.jks \
  -keyalg RSA -keysize 2048 -validity 10000 \
  -alias arcade \
  -dname "CN=Treant Arcade, O=Peter Wicks, C=US"
```

`treant-arcade-keystore.env` holds:

```
TREANT_ARCADE_KEYSTORE=/home/peter/.config/homenet/treant-arcade.jks
TREANT_ARCADE_KEY_ALIAS=arcade
TREANT_ARCADE_STORE_PASSWORD=…
TREANT_ARCADE_KEY_PASSWORD=…
```

> **Back this up.** Losing the upload key means you can no longer push updates to
> the same Play listing without a Google key-reset (support ticket + days of
> delay). Copy the `.jks` and `.env` somewhere safe and offline.

---

## Building a signed release AAB

**A signed release AAB has been built locally on this box** — the Android SDK is
installed at `~/Android/Sdk` and `android/app/build.gradle` is already wired for
release signing (see step 1). The only remaining step is uploading to the Play
Console. To rebuild:

### 1. Point Gradle at the keystore — DONE

`android/app/build.gradle` now carries a `signingConfigs` block that reads the
credentials from the environment (so nothing secret is written into the repo).
It is guarded: if the env vars are absent the release config is simply not
wired, so debug builds still work with no keystore present.

```gradle
android {
    def keystorePath = System.getenv("TREANT_ARCADE_KEYSTORE")
    signingConfigs {
        if (keystorePath != null && !keystorePath.isEmpty() && file(keystorePath).exists()) {
            release {
                storeFile file(keystorePath)
                storePassword System.getenv("TREANT_ARCADE_STORE_PASSWORD")
                keyAlias System.getenv("TREANT_ARCADE_KEY_ALIAS")
                keyPassword System.getenv("TREANT_ARCADE_KEY_PASSWORD")
            }
        }
    }
    buildTypes {
        release {
            minifyEnabled false
            if (signingConfigs.findByName("release") != null) {
                signingConfig signingConfigs.release
            }
        }
    }
}
```

### 2. Refresh the web assets + sync

```bash
cd app
./build-www.sh          # ARCADE_APP_BUILD=1 build, verified analytics-free
npx cap sync android
```

### 3. Build the AAB with the credentials in the environment

```bash
cd app
export ANDROID_HOME=~/Android/Sdk
set -a; source ~/.config/homenet/treant-arcade-keystore.env; set +a
cd android
./gradlew bundleRelease
# → android/app/build/outputs/bundle/release/app-release.aab  (~4.3 MB, signed)
```

The last build produced a `4.3 MB` signed `app-release.aab` whose upload-key
cert is `CN=Treant Arcade, O=Peter Wicks, C=US` (self-signed, as an upload key
should be; `jarsigner -verify` reports **jar verified**). A copy lives outside
the repo at `~/treant-arcade-artifacts/app-release.aab`.

Upload that `.aab` to the Play Console. For a quick sideload/test build instead
of a store bundle, use `./gradlew assembleRelease` (produces an APK).

### 4. (First upload only) versioning

`android/app/build.gradle` sets `versionCode`/`versionName`. Bump `versionCode`
(integer, must strictly increase) on every Play upload; `versionName` is the
human string (e.g. `1.0.0`).

---

## Verifying a signed artifact

```bash
# Confirm the signed AAB carries the arcade upload key
keytool -printcert -jarfile app-release.aab      # → Owner: CN=Treant Arcade, …
jarsigner -verify app-release.aab                # → "jar verified."
# (apksigner verifies APKs, not AABs, e.g. the debug APK:)
$ANDROID_HOME/build-tools/35.0.0/apksigner verify --print-certs app-debug.apk
# or inspect the keystore entry
source ~/.config/homenet/treant-arcade-keystore.env
keytool -list -v -keystore "$TREANT_ARCADE_KEYSTORE" \
  -storepass "$TREANT_ARCADE_STORE_PASSWORD" -alias arcade
```
