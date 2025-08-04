@echo off
setlocal enabledelayedexpansion

:: CAI - Prompt Manager CLI Tool
:: Windows batch script for development and usage

:: Set colors (if supported)
set "RED=[91m"
set "GREEN=[92m"
set "YELLOW=[93m"
set "BLUE=[94m"
set "NC=[0m"

:: Function to show usage
if "%1"=="--help" goto :show_help
if "%1"=="-h" goto :show_help

:: Check if we're in the right directory
if not exist "Cargo.toml" (
    echo %RED%❌ Cargo.toml not found. Please run this script from the CAI project root directory.%NC%
    exit /b 1
)

:: Verify this is the CAI project
findstr /c:"name = \"prompt-manager\"" Cargo.toml >nul
if errorlevel 1 (
    echo %RED%❌ This doesn't appear to be the CAI project directory.%NC%
    exit /b 1
)

:: Parse arguments
set "BUILD=false"
set "TEST=false"
set "RELEASE=false"
set "CLEAN=false"
set "CAI_ARGS="

:parse_args
if "%1"=="" goto :done_parsing
if "%1"=="-b" set "BUILD=true" & shift & goto :parse_args
if "%1"=="--build" set "BUILD=true" & shift & goto :parse_args
if "%1"=="-t" set "TEST=true" & shift & goto :parse_args
if "%1"=="--test" set "TEST=true" & shift & goto :parse_args
if "%1"=="-r" set "RELEASE=true" & shift & goto :parse_args
if "%1"=="--release" set "RELEASE=true" & shift & goto :parse_args
if "%1"=="-c" set "CLEAN=true" & shift & goto :parse_args
if "%1"=="--clean" set "CLEAN=true" & shift & goto :parse_args

:: Collect remaining arguments for CAI
set "CAI_ARGS=%CAI_ARGS% %1"
shift
goto :parse_args

:done_parsing

:: Execute requested actions
if "%CLEAN%"=="true" (
    echo %BLUE%ℹ️  Cleaning build artifacts...%NC%
    cargo clean
    if errorlevel 1 exit /b 1
    echo %GREEN%✅ Clean completed%NC%
)

if "%TEST%"=="true" (
    echo %BLUE%ℹ️  Running tests...%NC%
    cargo test
    if errorlevel 1 exit /b 1
    echo %GREEN%✅ All tests passed%NC%
)

if "%BUILD%"=="true" (
    echo %BLUE%ℹ️  Building CAI project...%NC%
    if "%RELEASE%"=="true" (
        cargo build --release
        if errorlevel 1 exit /b 1
        echo %GREEN%✅ Release build completed%NC%
    ) else (
        cargo build
        if errorlevel 1 exit /b 1
        echo %GREEN%✅ Debug build completed%NC%
    )
)

:: Determine binary path
if "%RELEASE%"=="true" (
    set "BINARY_PATH=.\target\release\cai.exe"
) else (
    set "BINARY_PATH=.\target\debug\cai.exe"
)

:: Check if binary exists, build if not
if not exist "%BINARY_PATH%" (
    echo %YELLOW%⚠️  Binary not found, building...%NC%
    if "%RELEASE%"=="true" (
        cargo build --release
    ) else (
        cargo build
    )
    if errorlevel 1 exit /b 1
)

:: Set default prompts directory
if not defined CAI_PROMPTS_DIR set "CAI_PROMPTS_DIR=.\prompts"

:: Create prompts directory if it doesn't exist and we're not just showing help
if "%CAI_ARGS%"=="" goto :show_default_help
echo %CAI_ARGS% | findstr /i "help" >nul
if not errorlevel 1 goto :run_cai

if not exist "%CAI_PROMPTS_DIR%" (
    echo %YELLOW%⚠️  Prompts directory not found: %CAI_PROMPTS_DIR%%NC%
    echo %BLUE%ℹ️  Creating prompts directory...%NC%
    mkdir "%CAI_PROMPTS_DIR%"
    
    :: Create a sample prompt file
    (
        echo name: "Sample Prompts"
        echo description: "Sample prompt collection to get you started"
        echo subjects:
        echo   - name: "General"
        echo     prompts:
        echo       - title: "Welcome prompt"
        echo         content: "Welcome to CAI! This is a sample prompt to help you get started."
        echo         score: 0
        echo         id: "welcome-001"
    ) > "%CAI_PROMPTS_DIR%\sample.yaml"
    
    echo %GREEN%✅ Created sample prompts directory with example file%NC%
)

:run_cai
echo %BLUE%ℹ️  Running CAI: %BINARY_PATH%%CAI_ARGS%%NC%
"%BINARY_PATH%" %CAI_ARGS%
goto :eof

:show_default_help
echo.
echo 🤖 CAI - Prompt Manager CLI Tool
echo.
echo Usage: %0 [OPTIONS] [COMMAND] [ARGS...]
echo.
echo OPTIONS:
echo     -h, --help     Show this help message
echo     -b, --build    Build the project before running
echo     -t, --test     Run tests before executing
echo     -r, --release  Build and run in release mode
echo     -c, --clean    Clean build artifacts before building
echo.
echo COMMANDS:
echo     list                    List all available prompts
echo     search ^<query^>          Search prompts by keyword
echo     show ^<file_name^>        Show details of a specific prompt file
echo     query ^<file^> ^<subject^> ^<prompt^>  Query a specific prompt
echo     chat                    Start interactive chat mode
echo.
echo EXAMPLES:
echo     %0 list                                    # List all prompts
echo     %0 search "debugging"                      # Search for debugging prompts
echo     %0 show bug_fixing                         # Show bug_fixing prompt file
echo     %0 chat                                    # Start chat mode
echo.
echo     %0 --build list                            # Build first, then list
echo     %0 --test --release chat                   # Test, build release, then chat
echo.
echo ENVIRONMENT VARIABLES:
echo     OPENROUTER_API_KEY     Required for chat mode
echo     CAI_PROMPTS_DIR        Default prompts directory (default: .\prompts^)
echo.
echo For chat mode, get your API key from: https://openrouter.ai/
echo.
echo %BLUE%ℹ️  Quick start examples:%NC%
echo   %0 list              # List all available prompts
echo   %0 search "debug"    # Search for debugging-related prompts
echo   %0 chat              # Start interactive chat mode (requires OPENROUTER_API_KEY^)
goto :eof

:show_help
echo.
echo 🤖 CAI - Prompt Manager CLI Tool
echo.
echo This script helps you build and run the CAI prompt management CLI.
echo.
echo Usage: %0 [OPTIONS] [COMMAND] [ARGS...]
echo.
echo OPTIONS:
echo     -h, --help     Show this help message
echo     -b, --build    Build the project before running
echo     -t, --test     Run tests before executing  
echo     -r, --release  Build and run in release mode
echo     -c, --clean    Clean build artifacts before building
echo.
echo The script will automatically:
echo   - Check if you're in the correct project directory
echo   - Build the project if the binary doesn't exist
echo   - Create a sample prompts directory if none exists
echo   - Pass through all CAI-specific commands and arguments
echo.
echo For CAI-specific help, run: %0 --help
goto :eof