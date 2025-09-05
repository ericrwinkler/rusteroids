@echo off
:: AI Development Assistant Screenshot Validation Script
:: For use by GitHub Copilot when implementing rendering changes

setlocal EnableDelayedExpansion

set PROJECT_ROOT=C:\Users\Eric_\Projects\rusteroids
set SCREENSHOT_TOOL=%PROJECT_ROOT%\tools\screenshot_tool\target\debug\screenshot_tool.exe
set VALIDATION_DIR=%PROJECT_ROOT%\tools\screenshot_tool\screenshots

if "%1"=="" goto help
if "%1"=="help" goto help

set CHANGE_TYPE=%1

echo [INFO] Starting screenshot validation for: %CHANGE_TYPE%

:: Create validation directory
if not exist "%VALIDATION_DIR%" mkdir "%VALIDATION_DIR%"

:: Generate timestamp using PowerShell
for /f "tokens=*" %%i in ('powershell -command "Get-Date -Format 'yyyy-MM-dd'"') do set "datestamp=%%i"
for /f "tokens=*" %%i in ('powershell -command "Get-Date -Format 'yyyyMMdd_HHmmss'"') do set "timestamp=%%i"

:: Ensure screenshot tool is built
if not exist "%SCREENSHOT_TOOL%" (
    echo [INFO] Building screenshot tool...
    cd "%PROJECT_ROOT%\tools\screenshot_tool"
    cargo build
    cd "%PROJECT_ROOT%"
    if not exist "%SCREENSHOT_TOOL%" (
        echo [ERROR] Failed to build screenshot tool
        exit /b 1
    )
)

:: Ensure main project builds
echo [INFO] Building main project...
cargo build >nul 2>&1
if errorlevel 1 (
    echo [ERROR] Main project build failed - cannot validate
    exit /b 1
)

:: Capture validation screenshot
set PREFIX=%CHANGE_TYPE%_%timestamp%
echo [INFO] Capturing screenshot with prefix: %PREFIX%

"%SCREENSHOT_TOOL%" --prefix "%PREFIX%" --output "%VALIDATION_DIR%" --executable "%PROJECT_ROOT%\target\debug\teapot_app.exe" --wait 4000

if errorlevel 1 (
    echo [ERROR] Screenshot validation failed
    exit /b 1
) else (
    echo [SUCCESS] Screenshot validation completed
    echo [INFO] Screenshot saved to: %VALIDATION_DIR%\%PREFIX%_*.png
    exit /b 0
)

:help
echo AI Assistant Screenshot Validation Script
echo Usage: validate_rendering.bat ^<type^>
echo.
echo Types:
echo   material    - Material system changes
echo   pipeline    - Pipeline management changes
echo   ubo         - UBO structure changes
echo   shader      - Shader modifications
echo   baseline    - Pre-change baseline
echo   validation  - Post-change validation
echo.
echo Examples:
echo   validate_rendering.bat baseline
echo   validate_rendering.bat material
echo   validate_rendering.bat validation
exit /b 0
