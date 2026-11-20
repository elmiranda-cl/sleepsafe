# Winget Package Manifests for SleepSafe

These manifests follow the official [Windows Package Manager (winget-pkgs)](https://github.com/microsoft/winget-pkgs) schema version 1.9.0.

## How to Submit to winget-pkgs:

1. Fork [microsoft/winget-pkgs](https://github.com/microsoft/winget-pkgs).
2. Copy the folder `manifests/e/elmiranda-cl/SleepSafe/0.1.0/` into the `manifests/e/elmiranda-cl/SleepSafe/0.1.0/` directory of your fork.
3. Validate locally on a Windows machine:
   ```powershell
   winget validate --manifest .\manifests\e\elmiranda-cl\SleepSafe\0.1.0
   winget test --manifest .\manifests\e\elmiranda-cl\SleepSafe\0.1.0
   ```
4. Open a Pull Request to `microsoft/winget-pkgs`. Once merged by Microsoft's automated pipeline, users worldwide can install it via:
   ```cmd
   winget install elmiranda-cl.SleepSafe
   ```
