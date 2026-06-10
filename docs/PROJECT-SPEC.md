# TTGTiSO-Desk — рабочая концепция

TTGTiSO-Desk — это кроссплатформенная система удалённого графического доступа для закрытых локальных сетей, ориентированная на Astra Linux Special Edition 1.8 “Воронеж”, X11/Fly, многопользовательскую работу и безопасное подключение через SSH/защищённые каналы.

## Главная идея

- На сервере Astra Linux установлен постоянный агент.
- Каждый пользователь получает отдельную изолированную графическую X11-сессию.
- Клиент подключается без участия удалённого пользователя.
- RDP и VNC не используются как основной протокол.
- Транспорт: SSH + собственный multiplexed-протокол поверх SSH.
- Для закрытых сетей добавляется внутренний relay/jump-host.
- Клиент: Tauri v2 + React + TypeScript.
- Ядро: Rust.
- Видео: X11 capture + GStreamer/H.264.
- Файлы: SFTP или защищённый файловый канал.
- Буфер обмена: двусторонний.
- Обязательны аудит, роли, журналирование, безопасная конфигурация.

## Принятые архитектурные решения

### Ядро

Выбор: Rust.

Основные библиотеки:

```text
tokio
serde
serde_json
thiserror
anyhow
tracing
tracing-subscriber
russh или ssh2
tauri v2
tauri-specta / specta
gstreamer
x11rb
zbus
directories
keyring
```

### Клиент

Выбор: Tauri v2 + React + TypeScript.

Web-клиент не входит в MVP, но архитектурно предусматривается через будущий WebRTC gateway.

### Серверный агент

- `systemd service`;
- автозапуск после перезагрузки;
- конфигурация в `/etc/ttgtiso-desk/`;
- runtime-данные в `/var/lib/ttgtiso-desk/`;
- логи в `/var/log/ttgtiso-desk/`;
- отдельный системный пользователь `ttgtiso-desk`.

### Графические сессии

- Новая отдельная X11-сессия на каждого пользователя.
- Приоритет — Astra Linux Fly.
- KDE/GNOME поддерживаются через абстракцию `os_pal`.
- Wayland не в MVP.
- X11 — основной backend.

### Протокол

```text
Client GUI
    ↓
SSH transport
    ↓
TTGTiSO multiplexed protocol
    ├── video channel
    ├── input channel
    ├── clipboard channel
    ├── file channel
    ├── control channel
    └── audit/telemetry channel
```

### Видео

MVP:

- 1080p / 30 FPS;
- адаптивный bitrate;
- приоритет низкой задержки;
- H.264 как основной кодек;
- VAAPI при наличии;
- software fallback обязательно;
- GStreamer pipeline.

### Безопасность

- SSH;
- ключи;
- пароль как fallback;
- интеграция с Secret Net Studio архитектурно предусматривается;
- 2FA не требуется;
- чувствительные данные шифруются;
- аудит подключений обязателен;
- пользовательское подтверждение подключения не требуется.

Роли:

```text
user
admin
support_operator
auditor
```

### Закрытая сеть

- Работа без интернета обязательна.
- Строгий firewall.
- Режим через один SSH-порт.
- Optional relay внутри закрытой сети.
- Optional jump host/bastion.
- GitHub API только optional, не зависимость.

## Рекомендуемая структура репозитория

```text
/ttgtiso-desk
├── apps/
│   ├── desktop-client/
│   ├── server-agent/
│   ├── relay-server/
│   └── admin-cli/
├── crates/
│   ├── protocol/
│   ├── transport/
│   ├── security/
│   ├── session-manager/
│   ├── video-pipeline/
│   ├── input-injector/
│   ├── clipboard/
│   ├── file-transfer/
│   ├── audit/
│   ├── config/
│   ├── os-pal/
│   └── shared-types/
├── packages/
│   ├── ui/
│   └── shared-types/
├── docs/
├── prompts/
├── packaging/
├── scripts/
└── tests/
```

## Рекомендуемый MVP

1. Серверный агент на Astra Linux.
2. Подключение клиента по SSH.
3. Создание отдельной X11-сессии.
4. Передача изображения 1080p/30 FPS.
5. Управление мышью и клавиатурой.
6. Двусторонний текстовый clipboard.
7. Передача файлов через SFTP.
8. Журнал подключений.
9. Минимальный Tauri-клиент.
10. Offline-установка без интернета.

## Не включать в MVP 1

- web-клиент;
- звук;
- multi-monitor;
- WebRTC;
- полноценная интеграция Secret Net;
- автообновление через GitHub;
- запись видео сессий;
- сложная админ-панель;
- LDAP/AD/ALD Pro;
- Wayland.
