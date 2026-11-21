# GloriaSetup rollout

## What changed

- `gloria.toml` already existed; `accent` and `description` now match the GloriaLauncher catalog (`#f59e0b`, "Universal offline media & document converter").
- `.github/workflows/build-installers.yml` now builds, ensures the release exists (`gh release create --generate-notes` if missing), packs with `projectGloria/GloriaSetup@v0.1.0` (signed when the secrets are set) and attaches `OmniConvert_<version>_x64-setup.exe` to the tag's release. The separate `softprops/action-gh-release` job was removed. The `installers` artifact upload is kept.

## Please

- Add the repo secrets `WINDOWS_PFX_BASE64` and `WINDOWS_PFX_PASSWORD`. Without them the setup is unsigned (SmartScreen may warn).
- `OmniConvert.ico` does not exist in the root, so `icon` stays `src-tauri/icons/icon.ico` (the app's own icon).
- The runtime files are `omniconvert.exe`, `ffmpeg.exe`, `ffprobe.exe`, `pandoc.exe`, `typst.exe` and `pdfium.dll`, all copied flat into `app-dist/` (sidecars resolve next to the exe).
- `data_folders` use the Tauri identifier `com.omniconvert.app`; the app only writes its crash journal to `%LOCALAPPDATA%`, `%APPDATA%` is listed as a safe default. Remove it if you do not want it wiped on purge.
- `legacy_names = ["OmniConvert"]` and `app_user_model_id` were kept from the existing manifest; the previous NSIS/MSI installers used that DisplayName.
- Nothing was built or run locally, so the workflow is untested. Check the first tag run.
- The `v1.2.0`-style tag version overrides the `0.1.0` in `gloria.toml` and `tauri.conf.json` at build time.
