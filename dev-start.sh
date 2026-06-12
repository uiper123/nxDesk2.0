#!/bin/bash

# TTGTiSO-Desk Development Launcher
# Запускает API сервер и клиент одновременно

set -e

echo "🚀 Starting TTGTiSO-Desk Development Environment..."

# Цвета для вывода
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Проверка зависимостей
echo -e "${YELLOW}Checking dependencies...${NC}"

if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Error: cargo not found. Please install Rust.${NC}"
    exit 1
fi

if ! command -v node &> /dev/null; then
    echo -e "${RED}Error: node not found. Please install Node.js.${NC}"
    exit 1
fi

# Cleanup при выходе
cleanup() {
    echo -e "\n${YELLOW}Shutting down services...${NC}"
    if [ -n "${API_PID:-}" ]; then
        kill "$API_PID" 2>/dev/null || true
    fi
    if [ -n "${AGENT_PID:-}" ]; then
        kill "$AGENT_PID" 2>/dev/null || true
    fi
    if [ -n "${CLIENT_PID:-}" ]; then
        kill "$CLIENT_PID" 2>/dev/null || true
    fi
    echo -e "${GREEN}Services stopped.${NC}"
    exit 0
}

trap cleanup SIGINT SIGTERM EXIT

if curl -fsS http://127.0.0.1:3001/api/health > /dev/null 2>&1; then
    echo -e "${RED}Error: API port 3001 is already serving /api/health.${NC}"
    echo -e "${YELLOW}Stop the old API server before running ./dev-start.sh again.${NC}"
    echo -e "${YELLOW}Hint: check /tmp/api-server.log or run: pgrep -af api-server${NC}"
    exit 1
fi

# Stop the old system service if running, to avoid port and socket conflicts
sudo systemctl stop ttgtiso-desk-agent.service 2>/dev/null || true

# Запуск Агента
echo -e "${GREEN}Starting Server Agent (requires sudo for multi-user/VNC support)...${NC}"
# Run a dummy sudo command in foreground to cache the password
sudo echo -e "${YELLOW}Sudo permissions acquired.${NC}"
sudo chmod 755 /var/lib/ttgtiso-desk 2>/dev/null || true
cargo build --package server-agent
sudo ./target/debug/server-agent > /tmp/server-agent.log 2>&1 &
AGENT_PID=$!

# Запуск API сервера
echo -e "${GREEN}Starting API Server (http://127.0.0.1:3001)...${NC}"
cargo run --package api-server > /tmp/api-server.log 2>&1 &
API_PID=$!

# Ждём запуска API
echo -e "${YELLOW}Waiting for API server to start...${NC}"
API_READY=false
for i in {1..45}; do
    if ! kill -0 "$API_PID" 2>/dev/null; then
        echo -e "${RED}✗ API Server process exited. Check /tmp/api-server.log${NC}"
        tail -n 40 /tmp/api-server.log || true
        exit 1
    fi

    if curl -s http://127.0.0.1:3001/api/health > /dev/null; then
        API_READY=true
        break
    fi
    sleep 1
done

# Проверка что API запустился
if [ "$API_READY" = "true" ]; then
    echo -e "${GREEN}✓ API Server is running (PID: $API_PID)${NC}"
else
    echo -e "${RED}✗ API Server failed to start in time. Check /tmp/api-server.log${NC}"
    exit 1
fi

# Запуск клиента
echo -e "${GREEN}Starting Desktop Client (Tauri Native Mode)...${NC}"
cd apps/desktop-client
# Используем tauri dev вместо обычного vite dev, так как нам нужен Rust бэкенд для TCP сокетов
npm run tauri dev > /tmp/desktop-client.log 2>&1 &
CLIENT_PID=$!
cd ../..

sleep 3

if ! kill -0 "$CLIENT_PID" 2>/dev/null; then
    echo -e "${RED}✗ Desktop Client process exited. Check /tmp/desktop-client.log${NC}"
    tail -n 60 /tmp/desktop-client.log || true
    exit 1
fi

echo -e "${GREEN}✓ Desktop Client is running (PID: $CLIENT_PID)${NC}"

echo ""
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}🎉 TTGTiSO-Desk is ready!${NC}"
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo -e "  📡 API Server:     ${YELLOW}http://127.0.0.1:3001${NC}"
echo -e "  🖥️  Desktop Client: ${YELLOW}Native GUI Window${NC}"
echo ""
echo -e "  📋 API Logs:       /tmp/api-server.log"
echo -e "  📋 Client Logs:    /tmp/desktop-client.log"
echo ""
echo -e "${YELLOW}Press Ctrl+C to stop all services${NC}"
echo ""

# Показ логов в реальном времени
tail -f /tmp/api-server.log /tmp/desktop-client.log
