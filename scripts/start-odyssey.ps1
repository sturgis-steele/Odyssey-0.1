[CmdletBinding()]
param(
    [switch]$Build,
    [string]$WslDistro
)
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$previousWslEnv = $env:WSLENV
Push-Location $root
try {
    if ($WslDistro) {
        $settings = 'APP_ENV','APP_NAME','BACKEND_HOST','BACKEND_PORT','LLAMA_BASE_URL','LLAMA_MODEL','MAX_TOOL_ITERATIONS','MAX_OUTPUT_TOKENS','CONTEXT_TOKENS','HISTORY_TURNS','MODEL_TIMEOUT_SECONDS','TEMPERATURE','ENABLE_THINKING','SYSTEM_PROMPT','ODYSSEY_TIMEZONE','LOG_LEVEL'
        $forward = @($previousWslEnv -split ':' | Where-Object { $_ })
        foreach ($key in $settings) {
            if ([Environment]::GetEnvironmentVariable($key) -and !($forward -match "^$key(/|$)")) { $forward += $key }
        }
        foreach ($key in 'ODYSSEY_ROOT','DATABASE_PATH') {
            $value = [Environment]::GetEnvironmentVariable($key)
            if ($value -and !($forward -match "^$key(/|$)")) {
                $forward += if ($value -match '^[A-Za-z]:[\\/]') { "$key/p" } else { $key }
            }
        }
        $env:WSLENV = $forward -join ':'
        $binary = './backend-rust/target/linux-x86_64/release/odyssey-backend'
        if ($Build -or !(Test-Path -LiteralPath $binary -PathType Leaf)) {
            & wsl.exe -d $WslDistro --cd $root --exec bash -lc 'if [ -f ~/.cargo/env ]; then . ~/.cargo/env; fi; cargo build --locked --release --manifest-path backend-rust/Cargo.toml --target-dir backend-rust/target/linux-x86_64'
            if ($LASTEXITCODE) { exit $LASTEXITCODE }
        }
        & wsl.exe -d $WslDistro --cd $root --exec $binary
        exit $LASTEXITCODE
    }
    $target = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $root 'backend-rust\target' }
    $binary = Join-Path $target 'release\odyssey-backend.exe'
    if ($Build -or !(Test-Path -LiteralPath $binary -PathType Leaf)) {
        & cargo build --locked --release --manifest-path backend-rust/Cargo.toml
        if ($LASTEXITCODE) { exit $LASTEXITCODE }
    }
    & $binary
    exit $LASTEXITCODE
} finally {
    $env:WSLENV = $previousWslEnv
    Pop-Location
}
