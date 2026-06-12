# TTGTiSO-Desk

TTGTiSO-Desk — это кроссплатформенная система удалённого графического доступа для закрытых локальных сетей. Разработана для Astra Linux Special Edition 1.8 "Воронеж" (X11/Fly) с обеспечением строгой многопользовательской изоляции и высокой производительности видео на базе GStreamer.

## Структура репозитория

```text
/ttgtiso-desk
├── apps/
│   ├── desktop-client/        # Tauri v2 + React клиентское приложение
│   ├── server-agent/          # Кроссплатформенный серверный агент (Rust): systemd на Linux, служба на Windows
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
- **Desktop-клиент** (Linux `.deb`/`.AppImage`/`.rpm` и Arch Linux `.pkg.tar.zst`, Windows `.msi`/`.exe`) через `tauri-action` с подписанными артефактами обновлений и файлом `latest.json`;
- **Серверный агент** (`ttgtiso-desk-agent-linux-x86_64` + `SHA256SUMS` + скрипты установки/обновления).

### Установка на Arch Linux
Для пользователей Arch Linux (и производных Manjaro, CachyOS, EndeavourOS) доступна установка полностью нативного пакета. В отличие от формата AppImage, нативный пакет корректно линкуется с системными библиотеками Mesa, что исключает ошибки вида `EGL_BAD_PARAMETER`.

**Вариант 1: Сборка из `PKGBUILD` (рекомендуемый)**
Склонируйте репозиторий и выполните сборку:
```bash
git clone https://github.com/uiper123/nxDesk2.0.git
cd nxDesk2.0
makepkg -si
```

**Вариант 2: Готовый пакет из релизов**
На странице релизов GitHub к каждой версии теперь прикрепляется файл `*.pkg.tar.zst`. Вы можете скачать его и установить двойным кликом (или через pacman):
```bash
sudo pacman -U ttgtiso-desk-0.1.8-1-x86_64.pkg.tar.zst
```

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

## Серверный агент на Windows

Серверный агент кроссплатформенный и на Windows устанавливается как **служба с автозапуском** (работает в фоне, переживает перезагрузку и выход пользователя из системы). Захват экрана выполняется через Win32 GDI, ввод — через SendInput; GStreamer/X11 на Windows не требуются. Формат кадров (PNG) совпадает с Linux, поэтому desktop-клиент работает без изменений.

Установка (PowerShell **от имени администратора**):
```powershell
# установка последнего релиза с GitHub и регистрация автозапускаемой службы
powershell -ExecutionPolicy Bypass -File install-agent.ps1

# установка из локально собранного бинарника
powershell -ExecutionPolicy Bypass -File install-agent.ps1 -BinaryPath .\target\release\server-agent.exe

# обновление и удаление
powershell -ExecutionPolicy Bypass -File update-agent.ps1
powershell -ExecutionPolicy Bypass -File install-agent.ps1 -Uninstall
```

Служба: `TTGTiSODeskAgent` (автозапуск, LocalSystem, авто-перезапуск при сбое). Бинарник — `C:\Program Files\TTGTiSO-Desk\`, конфиг и логи — `C:\ProgramData\TTGTiSO-Desk\`. Также доступны встроенные команды `--install-service`, `--uninstall-service`, `--run-service`. Подробнее: `docs/installation-windows.md`.
