[CmdletBinding(DefaultParameterSetName = 'GGUF')]
param(
    [Parameter(Mandatory, ParameterSetName = 'GGUF')][string]$ModelPath,
    [Parameter(Mandatory, ParameterSetName = 'Ollama')]
    [ValidatePattern('^[\w.-]+:[\w.-]+$')][string]$OllamaModel,
    [string]$ServerPath,
    [string]$Alias = 'odyssey-local',
    [ValidateRange(1, 65535)][int]$Port = 8080,
    [ValidateRange(1024, 131072)][int]$Context = 4096
)
$ErrorActionPreference = 'Stop'
if ($OllamaModel) {
    $name, $tag = $OllamaModel.Split(':')
    if ($name -in '.', '..' -or $tag -in '.', '..') { throw 'Invalid model name or tag.' }
    $store = if ($env:OLLAMA_MODELS) { $env:OLLAMA_MODELS } else { Join-Path $env:USERPROFILE '.ollama\models' }
    $manifest = Get-Content -Raw -LiteralPath (Join-Path $store "manifests\registry.ollama.ai\library\$name\$tag") | ConvertFrom-Json
    $layer = @($manifest.layers | Where-Object mediaType -eq 'application/vnd.ollama.image.model')
    if ($layer.Count -ne 1 -or $layer[0].digest -notmatch '^sha256:[a-f0-9]{64}$') {
        throw 'This manifest must contain one local GGUF model layer.'
    }
    $ModelPath = Join-Path $store ('blobs\' + $layer[0].digest.Replace(':', '-'))
}
if (-not $ServerPath) {
    $binary = Get-Command llama-server.exe -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
    $ServerPath = if ($binary) { $binary.Source } else { Join-Path $env:LOCALAPPDATA 'Programs\Ollama\lib\ollama\llama-server.exe' }
}
$server = (Resolve-Path -LiteralPath $ServerPath).Path
$model = (Resolve-Path -LiteralPath $ModelPath).Path
& $server -m $model --alias $Alias --host 127.0.0.1 --port $Port -c $Context -np 1 -ngl 0 --jinja --offline --no-webui --no-context-shift --cors-origins localhost
exit $LASTEXITCODE
