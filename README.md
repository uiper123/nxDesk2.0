# TTGTiSO-Desk

TTGTiSO-Desk — это кроссплатформенная система удалённого графического доступа для закрытых локальных сетей. Разработана для Astra Linux Special Edition 1.8 "Воронеж" (X11/Fly) с обеспечением строгой многопользовательской изоляции и высокой производительности видео на базе GStreamer.

## Структура репозитория

```text
/ttgtiso-desk
├── apps/
│   ├── desktop-client/        # Tauri v2 + React клиентское приложение
│   ├── server-agent/          # Серверный systemd-агент (Rust)
│   ├── relay-server/          # Прокси/бастион сервер (Rust)
│   └── admin-cli/             # консоль администратора (Rust)
├── crates/
│   ├── protocol/              # Фрейминг и парсинг протокола TTMP
│   ├── transport/             # SSH-транспорт и мультиплексирование
│   ├── security/              # Аутентификация и RBAC
│   ├── session-manager/       # Управление X11-сессиями
│   ├── video-pipeline/        # Захват и H.264 кодирование (GStreamer)
│   ├── input-injector/        # Эмуляция ввода X11/XTest
│   ├── clipboard/             # Буфер обмена
│   ├── file-transfer/         # Файловый обмен
│   ├── audit/                 # Логирование событий безопасности
│   ├── config/                # Конфигурация агента и клиента
│   ├── os-pal/                # Слой абстракции ОС
│   └── shared-types/          # Общие типы для Rust и TypeScript
├── packages/
│   ├── ui/                    # Общие UI React-компоненты
│   └── shared-types/          # Экспортированные TypeScript типы
└── docs/                      # Архитектурная документация и ADR
```

## Требования для сборки

- **Rust** 1.75+
- **Node.js** 20+
- **System dependencies (Linux):**
  `libgstreamer1.0-dev`, `libgstreamer-plugins-base1.0-dev`, `libx11-dev`, `libxtst-dev`

## Команды

### Rust (Backend / Agent / CLI)
Проверка компиляции всего workspace:
```bash
cargo check
```
Запуск юнит-тестов:
```bash
cargo test
```

### Node.js (Client UI / Frontend)
Установка всех зависимостей:
```bash
npm install
```
Запуск клиента в режиме разработки:
```bash
npm run dev
```
