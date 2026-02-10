# Loglite Backend Startup Script for Windows

Write-Host "🚀 Starting Loglite Backend..." -ForegroundColor Cyan

# Check if PostgreSQL is running
Write-Host "Checking PostgreSQL connection..." -ForegroundColor Yellow
$pgReady = pg_isready 2>$null
if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ PostgreSQL is not running or not accessible" -ForegroundColor Red
    Write-Host "Please start PostgreSQL and ensure it's running on localhost:5432" -ForegroundColor Yellow
    exit 1
}
Write-Host "✅ PostgreSQL is ready" -ForegroundColor Green

# Check if database exists
Write-Host "Checking database..." -ForegroundColor Yellow
$dbExists = psql -U postgres -lqt 2>$null | Select-String -Pattern "loglite"
if (-not $dbExists) {
    Write-Host "Creating database 'loglite'..." -ForegroundColor Yellow
    psql -U postgres -c "CREATE DATABASE loglite;" 2>$null
    if ($LASTEXITCODE -eq 0) {
        Write-Host "✅ Database created" -ForegroundColor Green
    } else {
        Write-Host "❌ Failed to create database" -ForegroundColor Red
        exit 1
    }
} else {
    Write-Host "✅ Database exists" -ForegroundColor Green
}

# Check if .env exists
if (-not (Test-Path "loglite-backend\.env")) {
    Write-Host "Creating .env from .env.example..." -ForegroundColor Yellow
    Copy-Item "loglite-backend\.env.example" "loglite-backend\.env"
    Write-Host "✅ .env created" -ForegroundColor Green
}

# Start backend
Write-Host ""
Write-Host "Starting backend server..." -ForegroundColor Cyan
Write-Host "API will be available at: http://localhost:8000" -ForegroundColor Green
Write-Host ""

Set-Location loglite-backend
cargo run --release
