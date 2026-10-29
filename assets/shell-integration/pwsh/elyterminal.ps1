# ELYTerminal: wrap the prompt to report folder and title and to mark prompts and commands.
$global:ElyPrompt = $function:prompt
$global:ElyRan = $false
function global:prompt {
    # First: every statement sets $? again.
    $ok = $?
    $esc, $bel = [char]27, [char]7
    if ($global:ElyRan) {
        $code = 0
        if (-not $ok) {
            # Each native run sets a new $LASTEXITCODE, equal or not; strict mode refuses an unset one.
            $exit = Get-Variable LASTEXITCODE -Scope Global -ValueOnly -ErrorAction Ignore
            $ran = $null -ne $exit -and -not [object]::ReferenceEquals($exit, $global:ElyExit)
            $code = if (-not $global:ElyShell -and $ran -and $exit -ne 0) { $exit } else { 1 }
        }
        [Console]::Write("$esc]133;D;$code$bel")
        $global:ElyRan = $false
    }
    [Console]::Write("$esc]133;A;aid=elyterminal$bel")
    # The user's prompt sees a failed command's $? still.
    if (-not $ok) { Write-Error '' -ErrorAction Ignore }
    $text = & $global:ElyPrompt
    $folder = $executionContext.SessionState.Path.CurrentLocation
    if ($folder.Provider.Name -eq 'FileSystem') {
        $parts = $folder.ProviderPath.Replace('\', '/') -split '/'
        $path = ($parts | ForEach-Object { [Uri]::EscapeDataString($_) }) -join '/'
        if (-not $path.StartsWith('/')) { $path = "/$path" }
        $leaf = (Split-Path -Leaf $folder.ProviderPath) -replace '\p{Cc}', ''
        [Console]::Write("$esc]7;file://$path$bel$esc]2;$leaf$bel")
    }
    "$text$esc]133;B$bel"
}
# Git Bash's /e/x is E:\x where nothing lies at \e\x here.
function global:ElyDrivePath([string]$line) {
    # One line, one plain token: no quote of any kind.
    $token = '^([ \t]*cd[ \t]+)/([A-Za-z])(/[\w.\-/*?\[\]+@#%!,=~]*)?([ \t]*)$'
    if ($env:OS -ne 'Windows_NT' -or $line -match '[\r\n]' -or $line -notmatch $token) { return $line }
    $lead, $letter, $rest, $tail = $Matches[1], $Matches[2], "$($Matches[3])", $Matches[4]
    $cd = Get-Command cd -ErrorAction Ignore
    $own = -not ($cd -is [System.Management.Automation.AliasInfo] -and
        $cd.ResolvedCommand -is [System.Management.Automation.CmdletInfo] -and
        $cd.ResolvedCommand.Name -eq 'Set-Location')
    # A cd of the user's own, or a path that exists, stays as typed.
    if ($own -or (Test-Path -Path "/$letter$rest" -ErrorAction Ignore) -or
        -not (Test-Path -LiteralPath "${letter}:\" -ErrorAction Ignore)) { return $line }
    "$lead$($letter.ToUpperInvariant()):\$($rest.TrimStart('/').Replace('/', '\'))$tail"
}
# A command starts once PSReadLine hands over a line that is not blank.
if (Test-Path Function:\PSConsoleHostReadLine) {
    $global:ElyReadLine = $function:PSConsoleHostReadLine
    function global:PSConsoleHostReadLine {
        $line = ElyDrivePath (& $global:ElyReadLine)
        if (-not [string]::IsNullOrWhiteSpace($line)) {
            $global:ElyRan = $true
            $global:ElyExit = Get-Variable LASTEXITCODE -Scope Global -ValueOnly -ErrorAction Ignore
            # A program's code is the line's, unless the line surely ends in PowerShell: a cmdlet, a name not found, or no command.
            $ast = [System.Management.Automation.Language.Parser]::ParseInput($line, [ref]$null, [ref]$null)
            # Strict mode refuses .Count on one statement alone, so the statements stay a list.
            $statements = @(if ($ast.EndBlock) { $ast.EndBlock.Statements })
            $last = if ($statements.Count) { $statements[$statements.Count - 1] }
            if ($last -is [System.Management.Automation.Language.PipelineAst]) {
                $elements = $last.PipelineElements
                $last = $elements[$elements.Count - 1]
            }
            $name = if ($last -is [System.Management.Automation.Language.CommandAst]) { $last.GetCommandName() }
            $global:ElyShell = if ($name) {
                $pattern = [System.Management.Automation.WildcardPattern]::Escape($name)
                $info = Get-Command -Name $pattern -ErrorAction Ignore | Select-Object -First 1
                while ($info -and $info.CommandType -eq 'Alias') { $info = $info.ResolvedCommand }
                $null -eq $info -or $info.CommandType -eq 'Cmdlet'
            } else {
                $null -eq $last -or -not $last.Find({ $args[0] -is [System.Management.Automation.Language.CommandAst] }, $true)
            }
            [Console]::Write("$([char]27)]133;C$([char]7)")
        }
        $line
    }
} else {
    [Console]::Error.WriteLine('ELYTerminal: PowerShell marks no command without PSReadLine')
}
