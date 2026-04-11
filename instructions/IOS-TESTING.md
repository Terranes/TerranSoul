# Testing TerranSoul on iPhone (iOS)

This guide explains how to test TerranSoul on your iPhone 13 Pro Max (or any
iOS 16+ device) using Apple TestFlight and the automated CI pipeline.

---

## Overview

TerranSoul uses **Tauri 2** which supports iOS builds. The repository includes a
GitHub Actions workflow (`ios-testflight.yml`) that automatically builds the iOS
app and uploads it to **TestFlight** whenever code is pushed to `main`. You can
also trigger it manually. Once uploaded, TestFlight delivers the build directly
to your phone.

---

## Prerequisites

Before the automation works you need to set up Apple credentials **once**.

### 1. Apple Developer Account

Sign up at <https://developer.apple.com> (requires the $99/year Apple Developer
Program membership).

### 2. Create an App ID

1. Go to **Certificates, Identifiers & Profiles → Identifiers**.
2. Click **+** and register a new **App ID**.
   - Platform: **iOS**
   - Bundle ID: `com.terranes.terransoul` (must match `identifier` in
     `src-tauri/tauri.conf.json`)

### 3. Create an iOS Distribution Certificate

1. Open **Keychain Access** on your Mac.
2. Go to **Keychain Access → Certificate Assistant → Request a Certificate from a
   Certificate Authority** (save to disk).
3. In the Apple Developer portal go to **Certificates → +** and create an
   **Apple Distribution** certificate using the CSR from step 2.
4. Download the `.cer` file and double-click to install it in Keychain Access.
5. Export the certificate + private key as a **`.p12`** file with a password.

### 4. Create a Provisioning Profile

1. Go to **Profiles → +** in the Apple Developer portal.
2. Select **App Store Connect** distribution type.
3. Choose the App ID from step 2 and the certificate from step 3.
4. Download the `.mobileprovision` file.

### 5. Generate an App Store Connect API Key

1. Go to <https://appstoreconnect.apple.com/access/integrations/api>.
2. Click **+** to generate a new key with **Developer** role.
3. Download the `.p8` key file (you can only download it once).
4. Note the **Key ID** and **Issuer ID**.

### 6. Create the App in App Store Connect

1. Go to <https://appstoreconnect.apple.com> → **My Apps → +**.
2. Create a new app:
   - Platform: **iOS**
   - Name: **TerranSoul**
   - Bundle ID: `com.terranes.terransoul`
   - SKU: `terransoul`

### 7. Configure GitHub Repository Secrets

Go to **Settings → Secrets and variables → Actions** in the GitHub repository
and add these secrets:

| Secret Name | Value |
|-------------|-------|
| `APPLE_ID` | Your Apple ID email |
| `IOS_CERTIFICATE_P12_BASE64` | Base64-encoded `.p12` file (see below) |
| `IOS_CERTIFICATE_PASSWORD` | Password used when exporting the `.p12` |
| `IOS_PROVISIONING_PROFILE_BASE64` | Base64-encoded `.mobileprovision` file |
| `APP_STORE_CONNECT_API_KEY_ID` | Key ID from step 5 |
| `APP_STORE_CONNECT_ISSUER_ID` | Issuer ID from step 5 |
| `APP_STORE_CONNECT_API_KEY` | Contents of the `.p8` key file |

**Encoding files to base64:**

```bash
# Certificate
base64 -i Certificates.p12 | pbcopy

# Provisioning profile
base64 -i TerranSoul.mobileprovision | pbcopy
```

Paste the clipboard contents into the corresponding GitHub secret.

---

## How the Automation Works

Once the secrets above are configured:

1. **Push to `main`** — the `iOS TestFlight Deploy` workflow runs automatically.
2. The workflow builds the Tauri iOS app on a macOS GitHub Actions runner.
3. The resulting `.ipa` is uploaded to App Store Connect / TestFlight.
4. Apple processes the build (usually 5–30 minutes).
5. TestFlight sends a push notification to your enrolled device.
6. Open the **TestFlight** app on your iPhone and tap **Install**.

You can also trigger a build manually from the **Actions** tab using
**Run workflow**.

---

## Installing on Your iPhone 13 Pro Max

### First-Time Setup (one-time)

1. Install the **TestFlight** app from the App Store on your iPhone:
   <https://apps.apple.com/app/testflight/id899247664>
2. In App Store Connect, go to your app → **TestFlight → Internal Testing**.
3. Create an **internal testing group** and add your Apple ID as a tester.
4. You will receive an email invitation — open it on your iPhone and accept.
5. TestFlight will now show TerranSoul in the app list.

### Installing Each Build

After CI uploads a new build:

1. Wait for Apple's processing notification (push notification or email).
2. Open **TestFlight** on your iPhone 13 Pro Max.
3. Tap **TerranSoul → Install**.
4. The app installs and appears on your home screen.

### Enabling Auto-Install (Automatic Updates)

To have new builds install automatically:

1. Open **TestFlight** on your iPhone.
2. Tap **TerranSoul**.
3. Enable **Automatic Updates** toggle.
4. Every new CI build will install automatically when your phone is on Wi-Fi and
   charging (or when you open TestFlight).

---

## Triggering a Manual Build

If you need a build without pushing to `main`:

1. Go to the repository on GitHub.
2. Click **Actions → iOS TestFlight Deploy → Run workflow**.
3. Optionally add a build note (shown to testers in TestFlight).
4. Click **Run workflow**.

---

## Troubleshooting

| Issue | Solution |
|-------|----------|
| Workflow fails at "Install Apple certificate" | Verify `IOS_CERTIFICATE_P12_BASE64` and `IOS_CERTIFICATE_PASSWORD` secrets are correct |
| Upload to TestFlight fails | Check that the App Store Connect API key has **Developer** or **Admin** role |
| Build not showing in TestFlight | Apple processing can take 5–30 min; check App Store Connect for status |
| App crashes on launch | Check that bundle ID matches everywhere (`com.terranes.terransoul`) |
| "No eligible devices" in TestFlight | Ensure your iPhone 13 Pro Max runs iOS 16+ and is registered as a test device |

---

## Device Compatibility

TerranSoul's iOS build targets **iOS 16+** which is supported by:
- iPhone 13 Pro Max ✅
- iPhone 13 / 13 mini / 13 Pro ✅
- All newer iPhones ✅
- iPads with iPadOS 16+ ✅
