# TTGTiSO-Desk — подробный roadmap разработки

## Этап 0. Анализ и фиксация архитектурных решений

### Цель

Перед кодом агент должен провести анализ, определить границы MVP и зафиксировать технические решения.

### Результаты

```text
docs/architecture.md
docs/threat-model.md
docs/protocol.md
docs/mvp-scope.md
docs/adr/
```

### Что должен сделать ИИ

1. Проанализировать требования.
2. Выявить риски: X11-сессии, Astra Linux Fly, Secret Net, работа через один SSH-порт, производительность видео.
3. Предложить MVP.
4. Зафиксировать решения в ADR.
5. Не писать production-код до утверждения архитектуры.

### Definition of Done

- есть описание целевой архитектуры;
- есть список модулей;
- есть MVP-scope;
- есть модель угроз;
- есть схема Mermaid;
- есть список рисков.

## Этап 1. Скелет монорепозитория

### Цель

Создать структуру проекта без бизнес-логики.

### Результаты

```text
Cargo workspace
Tauri workspace
React app
crates/*
apps/*
docs/*
tests/*
CI skeleton
```

### Правила

- каждый файл не более 200–250 строк;
- один модуль — одна ответственность;
- сначала интерфейсы, потом реализация;
- перед каждым файлом в ответе указывать путь;
- никаких God Object.

### Definition of Done

- проект собирается;
- есть пустые crates;
- есть shared types;
- есть базовый logging;
- есть `README.md`;
- есть `cargo fmt`, `cargo clippy`, `npm lint`.

## Этап 2. Протокол TTGTiSO Multiplex Protocol

### Цель

Разработать собственный прикладной протокол поверх SSH.

### Каналы

```text
control
video
input
clipboard
file
audit
heartbeat
```

### Требования

- бинарный framing;
- версионирование протокола;
- heartbeat;
- reconnect;
- backpressure;
- capability negotiation;
- error codes;
- request/response + event stream.

### Definition of Done

- есть спецификация;
- есть Rust-типы сообщений;
- есть unit-тесты сериализации;
- есть mock transport;
- есть тест multiplexing.

## Этап 3. Безопасность

### Цель

Сформировать security baseline.

### Требования

- SSH keys;
- password fallback;
- host key verification;
- keychain на клиентах;
- роли;
- аудит;
- минимальные привилегии;
- интеграционные точки для Secret Net Studio.

### Definition of Done

- есть RBAC;
- есть модель угроз;
- есть журнал безопасности;
- есть secure config;
- есть запрет небезопасных дефолтов;
- есть тесты на авторизацию.

## Этап 4. Серверный агент

### Цель

Создать постоянный агент для Astra Linux.

### Требования

- systemd service;
- автозапуск;
- конфиг в `/etc/ttgtiso-desk/agent.toml`;
- логи через journald + файл;
- health endpoint через локальный unix socket;
- управление сессиями.

### Definition of Done

- агент стартует как сервис;
- читает конфиг;
- пишет логи;
- принимает SSH/control connection;
- создаёт mock-сессию;
- корректно останавливается.

## Этап 5. Astra Linux Session Manager

### Цель

Создавать отдельные графические X11-сессии для пользователей.

### Требования

- поддержка Astra Linux SE 1.8;
- Fly desktop priority;
- X11 only for MVP;
- PAM/system user integration;
- отдельная сессия на пользователя;
- изоляция runtime-директорий;
- лимиты ресурсов.

### Definition of Done

- можно создать отдельную сессию;
- можно остановить сессию;
- можно получить статус;
- сессии изолированы;
- есть fallback mock backend для тестов.

## Этап 6. Видео pipeline

### Цель

Передавать графическую сессию с низкой задержкой.

### MVP

- X11 capture;
- H.264;
- 1080p 30 FPS;
- adaptive bitrate;
- VAAPI если доступен;
- software fallback.

### Definition of Done

- есть capture abstraction;
- есть encoder abstraction;
- есть transport abstraction;
- есть mock video source;
- есть benchmark latency/FPS;
- есть настройка качества.

## Этап 7. Управление вводом

### Цель

Передавать мышь и клавиатуру в удалённую X11-сессию.

### Требования

- mouse move;
- mouse click;
- scroll;
- keyboard input;
- hotkey handling;
- раскладки;
- защита от небезопасных системных комбинаций.

### Definition of Done

- работает ввод в mock-сессии;
- работает X11 backend;
- есть фильтр запрещённых комбинаций;
- есть unit-тесты mapping-клавиш.

## Этап 8. Desktop Client

### Цель

Создать минималистичный кроссплатформенный GUI.

### Экраны

```text
Login
Host List
Connection Card
Active Session
File Transfer
Settings
Logs
Admin Panel
```

### Definition of Done

- клиент запускается;
- можно добавить хост;
- можно подключиться к mock-agent;
- отображается mock video stream;
- отправляется input event;
- есть базовые настройки.

## Этап 9. Clipboard + File Transfer

### Цель

Реализовать буфер обмена и временный файловый обмен.

### Definition of Done

- работает текстовый clipboard;
- работает передача файла;
- есть progress;
- есть resume;
- есть audit event;
- есть лимиты размера.

## Этап 10. Relay Server / Bastion Mode

### Цель

Дать возможность подключения при невозможности прямого доступа.

### Режимы

```text
direct SSH
SSH via jump host
internal relay
single-port mode
```

### Definition of Done

- клиент подключается через relay;
- relay не расшифровывает пользовательские данные без необходимости;
- есть audit;
- есть health-check;
- есть config.

## Этап 11. Установка и offline deployment

### Цель

Подготовить установку для закрытых сетей.

### Требования

- `.deb` для Astra;
- shell installer;
- offline bundle;
- systemd unit;
- unattended install;
- hardening guide.

### Definition of Done

- можно установить агент на Astra;
- сервис стартует после перезагрузки;
- можно удалить агент;
- есть offline-инструкция;
- есть пример конфига.

## Этап 12. Администрирование, аудит, роли

### Цель

Сделать корпоративный слой управления.

### Требования

- роли;
- журнал подключений;
- журнал передачи файлов;
- журнал clipboard-событий;
- экспорт логов;
- просмотр активных сессий;
- отключение сессии администратором.

### Definition of Done

- есть audit storage;
- есть admin CLI;
- есть UI журналов;
- есть фильтры;
- есть экспорт JSON/CSV.

## Этап 13. Тестирование и CI

### Цель

Обеспечить проверяемость проекта.

### Тесты

```text
unit
integration
e2e
security
performance
UI
Astra-specific manual/VM tests
```

### Definition of Done

- CI проходит;
- есть mock server;
- есть e2e сценарий подключения;
- есть performance benchmark;
- есть security regression tests.

## Этап 14. Production hardening

### Цель

Довести до состояния production-ready.

### Definition of Done

- проект можно поставить в тестовую закрытую сеть;
- есть инструкция администратора;
- есть known limitations;
- есть security checklist;
- есть план аттестационной подготовки.
