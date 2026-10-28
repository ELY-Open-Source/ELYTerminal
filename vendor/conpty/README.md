# conpty

Microsoft's ConPTY, `conpty.dll` and `OpenConsole.exe`, from the NuGet package `Microsoft.Windows.Console.ConPTY` 1.25.260930003 (SHA-256 `02b07b349af66d801159bdf9e440d4a1ce78bb951f37fc8609731665afdae7ee`), built from microsoft/terminal and signed by Microsoft. `LICENSE` is that repository's MIT license.

Windows' own conhost drops a program's mouse modes, `CSI ? 1000 h`, `1002 h` and `1006 h`, so no terminal hears them; OpenConsole, which Windows Terminal runs on, passes them through. alacritty loads `conpty.dll` from the executable's folder before the system's, and `conpty.dll` starts the `OpenConsole.exe` beside it.

`build.rs` copies the target's pair beside every Windows build, its tests and examples among them; `scripts/windows-zip.ps1` packs it with the executable. Each folder holds one architecture's pair: `x86_64` from the package's `x64` folders, `aarch64` from its `arm64`.

| File | SHA-256 |
|---|---|
| `x86_64/conpty.dll` | `feeef341d891643c62d30b6b07800bc70f0bc148f44cb8c3bee84aa557ae805a` |
| `x86_64/OpenConsole.exe` | `3d66b23d0a71bb8eed2b77edc8b9df9bf54ce6c8fb74c863a30e760997f80586` |
| `aarch64/conpty.dll` | `151a02355227710c20c9f8f1eaf68ac4668aa09cdc921adc7ec661804d8eb1bf` |
| `aarch64/OpenConsole.exe` | `4313825a3ba634d113cc6ed3bc8ce735e3c19d360cecd985794787479469f91f` |
