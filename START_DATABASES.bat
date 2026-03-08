@echo off
REM Twister v0.4 - Forensic Database Startup Script
REM Downloads and starts Neo4j and Qdrant

echo ════════════════════════════════════════════════════
echo  Twister Forensic Database Stack
echo  Neo4j + Qdrant (Native Windows)
echo ════════════════════════════════════════════════════

REM Create directories
if not exist "downloads" mkdir downloads
if not exist "databases\neo4j\data" mkdir databases\neo4j\data
if not exist "databases\neo4j\logs" mkdir databases\neo4j\logs
if not exist "databases\qdrant\storage" mkdir databases\qdrant\storage
if not exist "databases\qdrant\snapshots" mkdir databases\qdrant\snapshots

REM Check if Neo4j exists
if not exist "neo4j\bin\neo4j.bat" (
    echo.
    echo [1/4] Neo4j not found. Downloading...
    echo       This may take a few minutes.
    echo.
    
    REM Download Neo4j 5.23 Community
    curl -L -o downloads\neo4j.zip "https://neo4j.com/artifact.php?name=neo4j-community-5.23.0-windows.zip"
    
    echo       Extracting...
    powershell -Command "Expand-Archive -Path downloads\neo4j.zip -DestinationPath . -Force"
    move neo4j-* neo4j 2>nul
    
    if exist "neo4j\bin\neo4j.bat" (
        echo       Neo4j installed successfully.
    ) else (
        echo       ERROR: Neo4j extraction failed.
        echo       Please download manually from: https://neo4j.com/download-center/
        pause
        exit /b 1
    )
)

REM Check if Qdrant exists
if not exist "qdrant\qdrant.exe" (
    echo.
    echo [2/4] Qdrant not found. Downloading...
    echo.
    
    REM Download Qdrant latest
    curl -L -o downloads\qdrant.zip "https://github.com/qdrant/qdrant/releases/latest/download/qdrant-x86_64-pc-windows-msvc.zip"
    
    echo       Extracting...
    powershell -Command "Expand-Archive -Path downloads\qdrant.zip -DestinationPath . -Force"
    
    if exist "qdrant\qdrant.exe" (
        echo       Qdrant installed successfully.
    ) else (
        echo       ERROR: Qdrant extraction failed.
        echo       Please download manually from: https://github.com/qdrant/qdrant/releases
        pause
        exit /b 1
    )
)

echo.
echo [3/4] Starting Neo4j...
echo       Web UI: http://localhost:7474
echo       Bolt:   localhost:7687
echo       User:   neo4j
echo       Pass:   twister_forensic_2024
echo.

REM Start Neo4j
start "Neo4j" cmd /k "cd neo4j\bin && neo4j console"

timeout /t 15 /nobreak >nul

echo.
echo [4/4] Starting Qdrant...
echo       REST:   http://localhost:6333
echo       gRPC:   localhost:6334
echo       Dashboard: http://localhost:6333/dashboard
echo.

REM Start Qdrant
start "Qdrant" cmd /k "cd qdrant && qdrant.exe --storage-path ..\databases\qdrant\storage --snapshots-path ..\databases\qdrant\snapshots"

timeout /t 5 /nobreak >nul

echo.
echo ════════════════════════════════════════════════════
echo  Databases Starting...
echo ════════════════════════════════════════════════════
echo.
echo  Neo4j:  http://localhost:7474
echo  Qdrant: http://localhost:6333/dashboard
echo.
echo  Press any key to open status pages...
pause >nul

start http://localhost:7474
start http://localhost:6333/dashboard

echo.
echo Check the Neo4j and Qdrant console windows for status.
echo.
pause
