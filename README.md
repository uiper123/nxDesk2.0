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

## Система обновлений

Проект использует GitHub Releases как канал доставки обновлений для обоих компонентов.

### Выпуск новой версии
```bash
./scripts/bump-version.sh 0.2.0   # обновит версии, создаст коммит и тег v0.2.0
git push origin main --tags        # пуш тега запускает workflow .github/workflows/release.yml
```
Workflow собирает:
- **Desktop-клиент** (Linux `.deb`/`.AppImage`/`.rpm`, Windows `.msi`/`.exe`) через `tauri-action` с подписанными артефактами обновлений и файлом `latest.json`;
- **Серверный агент** (`ttgtiso-desk-agent-linux-x86_64` + `SHA256SUMS` + скрипты установки/обновления).

### Обновление desktop-клиента
Клиент проверяет обновления через Tauri Updater (Settings → Application Updates → "Check for Updates"). Обновление скачивается с GitHub Releases, проверяется по встроенной криптографической подписи и устанавливается с перезапуском приложения.

Для подписи обновлений в CI необходимо добавить секреты репозитория:
- `TAURI_SIGNING_PRIVATE_KEY` — приватный ключ (`tauri signer generate`);
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` — пароль ключа (пустая строка, если без пароля).

Публичный ключ хранится в `apps/desktop-client/src-tauri/tauri.conf.json` (`plugins.updater.pubkey`).

### Обновление серверного агента
```bash
# установка сразу из GitHub-релиза
sudo ./scripts/install-agent.sh --from-github

# проверка наличия обновлений (exit 10 = доступно обновление)
ttgtiso-desk-update --check

# обновление с авто-бэкапом и откатом при сбое запуска
sudo ttgtiso-desk-update
```
Скрипт `update-agent.sh` скачивает свежий бинарник из последнего релиза, проверяет SHA256, делает резервную копию текущего бинарника (`/usr/bin/ttgtiso-desk-agent.bak`) и автоматически откатывается, если сервис не стартует. Подробнее: `docs/update-strategy.md`.
