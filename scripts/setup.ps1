$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

Write-Host "== IPTV Peru API: instalacion ==" -ForegroundColor Cyan

if (-not (Test-Path ".venv")) {
    Write-Host "Creando entorno virtual..."
    python -m venv .venv
}

$venvPython = Join-Path $root ".venv\Scripts\python.exe"

Write-Host "Instalando dependencias..."
& $venvPython -m pip install --upgrade pip --quiet
& $venvPython -m pip install -r requirements.txt --quiet

if (-not (Test-Path ".env")) {
    Copy-Item ".env.example" ".env"
    Write-Host "Archivo .env creado desde .env.example (ajusta tu contrasena de PostgreSQL si la tienes)." -ForegroundColor Yellow
}

Write-Host "Creando base de datos 'iptv' si no existe..."
$envOut = & psql -U postgres -h localhost -w -d postgres -t -c "SELECT 1 FROM pg_database WHERE datname='iptv';"
if ($envOut -notmatch "1") {
    & psql -U postgres -h localhost -w -d postgres -c "CREATE DATABASE iptv;"
    Write-Host "Base de datos 'iptv' creada." -ForegroundColor Green
} else {
    Write-Host "La base de datos 'iptv' ya existe."
}

Write-Host "Listo. Para arrancar el servidor:" -ForegroundColor Cyan
Write-Host "  .\.venv\Scripts\python.exe run.py"
