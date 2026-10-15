# Windows Inno Setup installer

## Goal and authorization
Keep the existing portable ZIP and add a Windows Inno Setup executable. User approved the proposed approach and explicitly requested design and implementation.

## Requirements and acceptance
- Current-user installation into LocalAppData/Programs/NSB without elevation.
- Stable AppId for upgrades, Start Menu shortcut, optional desktop shortcut, uninstall entry and optional launch.
- Derive version from nsb-gui Cargo metadata. Support x64-compatible Windows.
- CI builds and uploads ZIP and setup separately, failing on missing output.
- User configuration at USERPROFILE/.config/nsb and data at USERPROFILE/.nsb remain outside installer ownership and survive uninstall.

## Boundaries
Only workflow, installer script and packaging documentation change. No runtime changes, automatic updates, signing, MSI/MSIX, ARM64 or bundled kernels.

## Design
Compile gui/installer/nsb.iss with runner-preinstalled ISCC, passing absolute executable/icon/output paths and package version as defines. Fixed AppId and PrivilegesRequired=lowest; default destination {localappdata}/Programs/NSB. Use standard files-in-use handling without application auto-restart. Setup output NSB-windows-x64-setup.exe, separate artifact from ZIP.

## Execution and validation
Implement script and workflow, document contract. Only read tmp/dev.log after changes; no build/check or independent review. Installer execution awaits CI and manual installation. No commit requested.

## Implementation outcome
Installer script and Windows workflow changes completed. Compiler defines: AppVersion, AppExe, AppIcon, OutputDir. English and Simplified Chinese supported. Workflow checks compiler, metadata, exit status and expected output. Existing ZIP and separate setup uploaded. Only tmp/dev.log inspected: no related errors. No build/check, installer compilation or independent review performed; Windows CI and manual installation remain follow-up validation.

## CI follow-up: missing translation
Inno Setup 6.7.1 on Windows runner does not include Languages/ChineseSimplified.isl. Remove the unsupported translation dependency and retain bundled English Default.isl; installation behavior unchanged. Earlier English+Chinese implementation notes are superseded. No local compiler run permitted; only development log inspection.
