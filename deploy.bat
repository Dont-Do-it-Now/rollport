@echo off
setlocal
cd /d "%~dp0"
chcp 65001 >nul
set "APP=.\src-tauri\target\release\rollport.exe"
echo [1/3] Closing any running Rollport process...
taskkill /IM rollport.exe /T /F >nul 2>&1
timeout /t 1 /nobreak >nul
echo [2/3] Building Rollport...
call pnpm tauri build
set "BUILD_EXIT=%errorlevel%"
if not "%BUILD_EXIT%"=="0" (
    echo Build failed with exit code %BUILD_EXIT%.
    pause
    exit /b %BUILD_EXIT%
)
if not exist "%APP%" (
    echo Build completed, but the executable was not found.
    pause
    exit /b 1
)
echo [3/3] Launching Rollport...
start "" /D "." "%APP%"
if errorlevel 1 (
    echo Rollport could not be launched.
    pause
    exit /b 1
)
echo Rollport launched. This window will remain open until you press a key.
pause
