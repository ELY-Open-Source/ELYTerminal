# windows-installer.ps1: the zip's files as an NSIS installer for one user. Run windows-zip.ps1 first.
Set-Location (Join-Path $PSScriptRoot '..') -ErrorAction Stop
$version = ((cargo pkgid) -split '[#@]')[-1]
if ($version -notmatch '^[0-9]') { throw "windows-installer: cargo pkgid names no version" }
$arch = @{ AMD64 = 'x86_64'; ARM64 = 'aarch64' }[$env:PROCESSOR_ARCHITECTURE]
if (-not $arch) { throw "windows-installer: no name for $env:PROCESSOR_ARCHITECTURE" }
$target = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { 'target' }
$out = (Resolve-Path (Join-Path $target 'release') -ErrorAction Stop).Path
foreach ($file in 'elyterminal.exe', 'conpty.dll', 'OpenConsole.exe', 'LICENSE-conpty') {
    if (-not (Test-Path (Join-Path $out $file))) { throw "windows-installer: $out lacks $file; run scripts/windows-zip.ps1 first" }
}
$makensis = if ($env:MAKENSIS) { $env:MAKENSIS } else { Join-Path ${env:ProgramFiles(x86)} 'NSIS\makensis.exe' }
if (-not (Test-Path $makensis)) { throw "windows-installer: no makensis at $makensis; install NSIS or set MAKENSIS" }
$setup = Join-Path $out "ELYTerminal-$version-windows-$arch-setup.exe"
$icon = (Resolve-Path 'assets/icons/elyterminal.ico' -ErrorAction Stop).Path
& $makensis /V2 /INPUTCHARSET UTF8 "/DVERSION=$version" "/DSTAGE=$out" "/DOUTFILE=$setup" "/DICON=$icon" 'packaging\windows\installer.nsi'
if ($LASTEXITCODE -ne 0) { throw "windows-installer: makensis failed" }
Write-Output "windows-installer: $setup"
