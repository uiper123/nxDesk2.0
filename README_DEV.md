# TTGTiSO-Desk Development Quick Start

## 🚀 Быстрый запуск

### Автоматический запуск (рекомендуется)

```bash
./dev-start.sh
```

Скрипт автоматически:
- Запустит API сервер на `http://127.0.0.1:3001`
- Запустит Desktop Client на `http://localhost:1420`
- Покажет логи в реальном времени
- Остановит всё при `Ctrl+C`

### Ручной запуск

**Терминал 1 - API Server:**
```bash
cargo run --package api-server
```

**Терминал 2 - Desktop Client:**
```bash
cd apps/desktop-client
npm run dev
```

## 📦 Структура проекта

```
Nxdesc/
├── apps/
│   ├── api-server/          # HTTP API (Rust/Axum) - Новый!
│   ├── desktop-client/      # React фронтенд (использует реальный API)
│   ├── relay-server/        # TCP прокси для сессий
│   ├── server-agent/        # Агент на удалённом хосте
│   └── admin-cli/          # CLI для администрирования
├── crates/                 # Общие библиотеки
└── docs/
    └── API_INTEGRATION.md  # Подробная документация API
```

## 🔄 Что изменилось

### ✅ Интеграция с реальным API завершена

Все компоненты фронтенда теперь работают с реальным HTTP API вместо фейковых данных:

- **Login** - Реальная аутентификация через API
- **HostList** - Загрузка хостов с сервера, auto-refresh каждые 10 сек
- **AdminPanel** - Управление сессиями через API, auto-refresh каждые 5 сек
- **Logs** - Реальные логи с сервера, auto-refresh каждые 3 сек
- **Settings** - Сохранение настроек на сервере

### 📡 API Endpoints

| Метод | Endpoint | Описание |
|-------|----------|----------|
| GET | `/api/health` | Проверка работоспособности |
| POST | `/api/auth/login` | Аутентификация |
| GET | `/api/hosts` | Список хостов |
| GET | `/api/sessions/active` | Активные сессии |
| POST | `/api/sessions/{id}/terminate` | Завершить сессию |
| GET | `/api/logs` | Получить логи |
| GET | `/api/settings` | Настройки |
| POST | `/api/settings` | Обновить настройки |

## 🧪 Тестирование API

```bash
# Проверка здоровья
curl http://127.0.0.1:3001/api/health

# Список хостов
curl http://127.0.0.1:3001/api/hosts | jq

# Активные сессии
curl http://127.0.0.1:3001/api/sessions/active | jq

# Завершить сессию
curl -X POST http://127.0.0.1:3001/api/sessions/s1/terminate
```

## 📚 Документация

- [API_INTEGRATION.md](docs/API_INTEGRATION.md) - Подробная документация по API
- Архитектура системы
- Примеры использования
- Следующие шаги интеграции

## 🛠️ Разработка

### Текущее состояние

- ✅ HTTP API Server (Rust/Axum)
- ✅ Интеграция фронтенда с API
- ✅ Auto-refresh для всех компонентов
- ✅ Loading/Error состояния
- ⏳ In-memory хранилище (нужна БД)
- ⏳ Интеграция с relay-server
- ⏳ Реальные метрики от server-agent

### Следующие шаги

1. **База данных** - PostgreSQL/SQLite для persistent storage
2. **Relay integration** - Подключить API к relay-server
3. **Agent metrics** - Реальные метрики CPU/RAM от агентов
4. **WebSocket** - Real-time обновления вместо polling
5. **Аутентификация** - PAM/SSH интеграция

## 🎯 Тестирование UI

1. Откройте `http://localhost:1420`
2. Войдите с любым username/host (пока без реальной проверки пароля)
3. Проверьте все разделы:
   - **Hosts Registry** - список хостов обновляется автоматически
   - **Admin Panel** - попробуйте завершить сессию
   - **Audit Logs** - просмотр логов с фильтрацией
   - **Settings** - измените настройки и сохраните

## 📝 Логи

- API Server: `/tmp/api-server.log`
- Desktop Client: `/tmp/desktop-client.log`

Или используйте `./dev-start.sh` для просмотра в реальном времени.

## 🐛 Troubleshooting

**API не запускается:**
```bash
# Проверьте что порт 3001 свободен
lsof -i :3001
# Если занят, убейте процесс
pkill -f api-server
```

**Клиент не подключается к API:**
- Убедитесь что API сервер запущен и отвечает: `curl http://127.0.0.1:3001/api/health`
- Проверьте браузерную консоль на CORS ошибки
- Проверьте что API URL правильный в [api.ts](apps/desktop-client/src/services/api.ts)

**Данные не обновляются:**
- Откройте Developer Tools → Network и проверьте запросы к API
- Проверьте логи API сервера
