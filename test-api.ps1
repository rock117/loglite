# Loglite API Test Script
# This script tests all the log ingestion endpoints

$baseUrl = "http://localhost:8000/api"

Write-Host "🧪 Testing Loglite API..." -ForegroundColor Cyan
Write-Host ""

# Test 1: Health Check
Write-Host "1️⃣ Testing health endpoint..." -ForegroundColor Yellow
try {
    $response = Invoke-RestMethod -Uri "$baseUrl/health" -Method Get
    Write-Host "✅ Health check passed: $($response.status)" -ForegroundColor Green
} catch {
    Write-Host "❌ Health check failed: $_" -ForegroundColor Red
    exit 1
}
Write-Host ""

# Test 2: Create App
Write-Host "2️⃣ Creating test application..." -ForegroundColor Yellow
$appData = @{
    name = "test-app"
} | ConvertTo-Json

try {
    $app = Invoke-RestMethod -Uri "$baseUrl/apps" -Method Post -Body $appData -ContentType "application/json"
    Write-Host "✅ App created: $($app.app_id)" -ForegroundColor Green
    $appId = $app.app_id
} catch {
    Write-Host "⚠️ App might already exist, continuing..." -ForegroundColor Yellow
    $appId = "default"
}
Write-Host ""

# Test 3: Java Logs
Write-Host "3️⃣ Testing Java log ingestion..." -ForegroundColor Yellow
$javaLogs = @"
2024-02-09 22:30:15.123 ERROR [main] com.example.App - Connection failed
java.lang.NullPointerException: Cannot invoke method
    at com.example.Service.process(Service.java:42)
    at com.example.App.main(App.java:15)
2024-02-09 22:30:16.456 INFO [worker-1] com.example.Service - Processing request
"@

try {
    $response = Invoke-RestMethod -Uri "$baseUrl/ingest/java" -Method Post -Body $javaLogs -ContentType "text/plain"
    Write-Host "✅ Java logs ingested: $($response.accepted) events" -ForegroundColor Green
} catch {
    Write-Host "❌ Java log ingestion failed: $_" -ForegroundColor Red
}
Write-Host ""

# Test 4: Rust Logs
Write-Host "4️⃣ Testing Rust log ingestion..." -ForegroundColor Yellow
$rustLogs = @"
[2024-02-09T14:30:15Z ERROR my_app] Database connection lost
[2024-02-09T14:30:16Z INFO  my_app::handler] Request completed
[2024-02-09T14:30:17Z WARN  my_app::db] Retrying connection
"@

try {
    $response = Invoke-RestMethod -Uri "$baseUrl/ingest/rust" -Method Post -Body $rustLogs -ContentType "text/plain"
    Write-Host "✅ Rust logs ingested: $($response.accepted) events" -ForegroundColor Green
} catch {
    Write-Host "❌ Rust log ingestion failed: $_" -ForegroundColor Red
}
Write-Host ""

# Test 5: Go Logs
Write-Host "5️⃣ Testing Go log ingestion..." -ForegroundColor Yellow
$goLogs = @"
2024/02/09 22:30:15 [ERROR] main.go:42: Failed to connect
2024/02/09 22:30:16 [INFO] handler.go:28: Request processed
2024/02/09 22:30:17 [WARN] db.go:15: Connection slow
"@

try {
    $response = Invoke-RestMethod -Uri "$baseUrl/ingest/go" -Method Post -Body $goLogs -ContentType "text/plain"
    Write-Host "✅ Go logs ingested: $($response.accepted) events" -ForegroundColor Green
} catch {
    Write-Host "❌ Go log ingestion failed: $_" -ForegroundColor Red
}
Write-Host ""

# Test 6: Auto-Detection
Write-Host "6️⃣ Testing auto-detection..." -ForegroundColor Yellow
$autoLogs = @"
2024-02-09 22:30:15.123 ERROR [main] com.example.App - Auto-detected Java log
java.lang.RuntimeException: Test exception
    at com.example.Test.run(Test.java:10)
"@

try {
    $response = Invoke-RestMethod -Uri "$baseUrl/ingest/auto" -Method Post -Body $autoLogs -ContentType "text/plain"
    Write-Host "✅ Auto-detection worked: $($response.accepted) events" -ForegroundColor Green
} catch {
    Write-Host "❌ Auto-detection failed: $_" -ForegroundColor Red
}
Write-Host ""

# Test 7: Search
Write-Host "7️⃣ Testing search..." -ForegroundColor Yellow
$searchQuery = @{
    app_id = "default"
    q = "ERROR"
    limit = 10
} | ConvertTo-Json

try {
    $results = Invoke-RestMethod -Uri "$baseUrl/search" -Method Post -Body $searchQuery -ContentType "application/json"
    Write-Host "✅ Search completed: $($results.total) total results, showing $($results.items.Count) items" -ForegroundColor Green
    
    if ($results.items.Count -gt 0) {
        Write-Host ""
        Write-Host "Sample results:" -ForegroundColor Cyan
        $results.items | Select-Object -First 3 | ForEach-Object {
            Write-Host "  - [$($_.level)] $($_.message.Substring(0, [Math]::Min(60, $_.message.Length)))..." -ForegroundColor Gray
        }
    }
} catch {
    Write-Host "❌ Search failed: $_" -ForegroundColor Red
}
Write-Host ""

Write-Host "✨ API tests completed!" -ForegroundColor Green
Write-Host ""
Write-Host "Next steps:" -ForegroundColor Cyan
Write-Host "  - View logs in the UI at http://localhost:5173 (if frontend is running)" -ForegroundColor Gray
Write-Host "  - Check test_logs.md for more examples" -ForegroundColor Gray
Write-Host "  - Read QUICKSTART.md for detailed usage" -ForegroundColor Gray
