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

**Blocked until the Android SDK is installed** (this box has JDK 21 but no SDK —
see [`README.md`](README.md)). Once it is:

### 1. Point Gradle at the keystore

The generated `android/app/build.gradle` has no `signingConfigs` block yet. Add
one that reads the credentials from the environment (so nothing secret is
written into the repo):

```gradle
android {
    signingConfigs {
        release {
            storeFile file(System.getenv("TREANT_ARCADE_KEYSTORE"))
            storePassword System.getenv("TREANT_ARCADE_STORE_PASSWORD")
            keyAlias System.getenv("TREANT_ARCADE_KEY_ALIAS")
            keyPassword System.getenv("TREANT_ARCADE_KEY_PASSWORD")
        }
    }
    buildTypes {
        release {
            signingConfig signingConfigs.release
            minifyEnabled false
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
set -a; source ~/.config/homenet/treant-arcade-keystore.env; set +a
cd android
./gradlew bundleRelease
# → android/app/build/outputs/bundle/release/app-release.aab
```

Upload that `.aab` to the Play Console. For a quick sideload/test build instead
of a store bundle, use `./gradlew assembleRelease` (produces an APK).

### 4. (First upload only) versioning

`android/app/build.gradle` sets `versionCode`/`versionName`. Bump `versionCode`
(integer, must strictly increase) on every Play upload; `versionName` is the
human string (e.g. `1.0.0`).

---

## Verifying a signed artifact

```bash
# Confirm the AAB/APK is signed with the arcade key
$ANDROID_HOME/build-tools/<ver>/apksigner verify --print-certs app-release.apk
# or inspect the keystore entry
source ~/.config/homenet/treant-arcade-keystore.env
keytool -list -v -keystore "$TREANT_ARCADE_KEYSTORE" \
  -storepass "$TREANT_ARCADE_STORE_PASSWORD" -alias arcade
```
