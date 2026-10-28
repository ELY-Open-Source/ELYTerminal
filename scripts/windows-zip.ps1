# windows-zip.ps1: the release build with icon and ConPTY, zipped.
Set-Location (Join-Path $PSScriptRoot '..') -ErrorAction Stop
cargo build --release --locked
if ($LASTEXITCODE -ne 0) { throw "windows-zip: cargo build failed" }
$version = ((cargo pkgid) -split '[#@]')[-1]
if ($version -notmatch '^[0-9]') { throw "windows-zip: cargo pkgid names no version" }
$arch = @{ AMD64 = 'x86_64'; ARM64 = 'aarch64' }[$env:PROCESSOR_ARCHITECTURE]
if (-not $arch) { throw "windows-zip: no name for $env:PROCESSOR_ARCHITECTURE" }
$target = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { 'target' }
$out = Join-Path $target 'release'
$zip = Join-Path $out "ELYTerminal-$version-windows-$arch.zip"
Copy-Item 'vendor/conpty/LICENSE' (Join-Path $out 'LICENSE-conpty') -ErrorAction Stop
$files = 'elyterminal.exe', 'conpty.dll', 'OpenConsole.exe', 'LICENSE-conpty' | ForEach-Object { Join-Path $out $_ }
Compress-Archive -Force -Path $files -DestinationPath $zip -ErrorAction Stop
Write-Output "windows-zip: $zip"
