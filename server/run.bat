@echo off
echo.
echo   AI Launcher - Building...
echo.
cargo build --release -p ai-launcher-server
if %ERRORLEVEL% NEQ 0 (
    echo Build failed! Make sure Rust is installed: https://rustup.rs
    pause
    exit /b 1
)
echo.
echo   Starting AI Launcher Server...
echo   Swagger UI: http://localhost:8080/swagger-ui/
echo.
target\release\ai-launcher-server.exe
pause
