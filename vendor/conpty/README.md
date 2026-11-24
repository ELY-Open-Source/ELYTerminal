# conpty

Microsoft's ConPTY, `conpty.dll` and `OpenConsole.exe`, from the NuGet package `Microsoft.Windows.Console.ConPTY` 1.24.261001001 (SHA-256 `4d6aaddc1d2385c9f5897df28f33879f699f8f2783315d5204cf3d8c3616ac5f`), built from microsoft/terminal and signed by Microsoft. `LICENSE` is that repository's MIT license.

Windows' own conhost drops a program's mouse modes, `CSI ? 1000 h`, `1002 h` and `1006 h`, so no terminal hears them; OpenConsole, which Windows Terminal runs on, passes them through. 1.25 loses whole chunks of a kitty graphics stream herdr relays, 1.24.261001001 none (issue #119, herdr#5657, microsoft/terminal#19621), so it stays at 1.24 until that closes. alacritty loads `conpty.dll` from the executable's folder before the system's, and `conpty.dll` starts the `OpenConsole.exe` beside it.

`OpenConsoleProxy.dll` comes from Windows Terminal 1.25.3131.0's release zips, `Microsoft.WindowsTerminal_1.25.3131.0_x64.zip` and `_arm64.zip`, MIT and signed by Microsoft: it carries the pipe and process handles OpenConsole hands a default terminal, `ITerminalHandoff3`, which COM cannot pass without it.

`build.rs` copies the target's files beside every Windows build, its tests and examples among them; `scripts/windows-zip.ps1` packs it with the executable. Each folder holds one architecture's files: `x86_64` from the package's `x64` folders, `aarch64` from its `arm64`.

| File | SHA-256 |
|---|---|
| `x86_64/conpty.dll` | `5afea6c9480e2c7dfd195376dd10996b0ae25cdc6f014e6247fd8aedc8c34852` |
| `x86_64/OpenConsole.exe` | `6f8e68dec4e8e5e15a54ecc8afa127b0e62139edc004ffbb4892125ff01d5737` |
| `aarch64/conpty.dll` | `015cb55e4f82b0b2bf008dfd98a0398c8239db186b9a6846f66eb918cfe3dd17` |
| `aarch64/OpenConsole.exe` | `5e4370eb99c591e2e68ed094079b56a16c9c5a00d1b76f684c46c9139439c303` |
| `x86_64/OpenConsoleProxy.dll` | `248fa9d255a7c45d7cabb03c84829ce495f4148697976b29c6009dd7ea1fbfe1` |
| `aarch64/OpenConsoleProxy.dll` | `81cfac4e66387f1661165ac221b7ac86f3a098c32f37ce51da9001150c1ffbc1` |
