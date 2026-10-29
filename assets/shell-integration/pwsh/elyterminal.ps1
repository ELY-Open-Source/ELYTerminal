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
# Git Bash's /e/x is E:\x, where no such folder lies here.
if ($env:OS -eq 'Windows_NT') {
    & {
        # Set-Location's own proxy keeps its parameters, pipeline and errors.
        $command = Get-Command Set-Location -CommandType Cmdlet
        $proxy = [System.Management.Automation.ProxyCommand]::Create($command)
        $drive = {
            $path = $PSBoundParameters['Path']
            if ($path -is [string] -and $path -match '^/([A-Za-z])(/.*)?$') {
                $root, $rest = "$($Matches[1]):\", "$($Matches[2])".TrimStart('/')
                if (-not (Test-Path -LiteralPath $path) -and (Test-Path -LiteralPath $root)) {
                    $PSBoundParameters['Path'] = $root + $rest.Replace('/', '\')
                }
            }
        }
        $begin = [regex]::Match($proxy, '(?m)^begin\r?\n\{')
        if (-not $begin.Success) { throw "ELYTerminal: Set-Location's proxy has no begin block" }
        $ely = $proxy.Insert($begin.Index + $begin.Length, "`n$drive")
        Set-Item -Path function:global:ElyLocation -Value ([ScriptBlock]::Create($ely)) -ErrorAction Stop
        Set-Alias -Name cd -Value ElyLocation -Option AllScope -Scope Global -Force
    }
}
# A command starts once PSReadLine hands over a line that is not blank.
if (Test-Path Function:\PSConsoleHostReadLine) {
    $global:ElyReadLine = $function:PSConsoleHostReadLine
    function global:PSConsoleHostReadLine {
        $line = & $global:ElyReadLine
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
