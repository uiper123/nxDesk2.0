# TTGTiSO-Desk - Интеграция с реальным API

## Архитектура

Система теперь использует реальный HTTP API вместо фейковых данных:

- **API Server** (Rust/Axum) - HTTP API на порту `3001`
- **Desktop Client** (React/Vite) - Фронтенд на порту `1420`
- **Relay Server** (Rust/Tokio) - TCP прокси для сессий (отдельно)

## Запуск системы

### 1. Запуск API сервера

```bash
cd /mnt/storage/projects/Nxdesc
cargo run --package api-server
```

API будет доступен на `http://127.0.0.1:3001`

### 2. Запуск клиента

```bash
cd apps/desktop-client
npm run dev
```

Клиент будет доступен на `http://localhost:1420`

## API Endpoints

- `GET /api/health` - Проверка работоспособности
- `POST /api/auth/login` - Аутентификация пользователя
- `GET /api/hosts` - Список хостов
- `GET /api/sessions/active` - Активные сессии
- `POST /api/sessions/{id}/terminate` - Завершение сессии
- `GET /api/logs` - Получение логов
- `GET /api/settings` - Получение настроек
- `POST /api/settings` - Обновление настроек

## Изменённые компоненты

### Фронтенд

1. **Login** ([Login.tsx](apps/desktop-client/src/components/Login/Login.tsx))
   - Использует `apiService.login()` вместо setTimeout
   - Показывает реальные ошибки от API

2. **HostList** ([HostList.tsx](apps/desktop-client/src/components/HostList/HostList.tsx))
   - Загружает хосты с API при монтировании
   - Автоматически обновляется каждые 10 секунд
   - Показывает loading/error состояния

3. **AdminPanel** ([AdminPanel.tsx](apps/desktop-client/src/components/AdminPanel/AdminPanel.tsx))
   - Загружает активные сессии с API
   - Автоматически обновляется каждые 5 секунд
   - Реально завершает сессии через API

4. **Logs** ([Logs.tsx](apps/desktop-client/src/components/Logs/Logs.tsx))
   - Загружает логи с API
   - Автоматически обновляется каждые 3 секунды
   - Фильтрация работает на клиенте

5. **Settings** ([Settings.tsx](apps/desktop-client/src/components/Settings/Settings.tsx))
   - Загружает настройки при открытии
   - Сохраняет изменения на сервер
   - Показывает состояние сохранения

### API Сервис

Создан единый API клиент ([api.ts](apps/desktop-client/src/services/api.ts)):
- TypeScript типы для всех запросов/ответов
- Централизованная обработка ошибок
- Единая точка настройки базового URL

### Бэкенд

Новый **API Server** ([apps/api-server](apps/api-server/)):
- `main.rs` - Точка входа и роутинг
- `handlers.rs` - HTTP обработчики
- `models.rs` - Модели данных
- `state.rs` - Глобальное состояние приложения

## Текущее состояние данных

Сейчас данные хранятся в памяти API сервера (in-memory). Начальные данные:

- **4 хоста**: 2 online, 1 busy, 1 offline
- **3 активные сессии**: разные пользователи с метриками CPU/RAM
- **7 лог записей**: INFO, WARN, ERROR, AUDIT
- **Настройки по умолчанию**: auto quality, VAAPI encoder, 30 FPS, audio off

## Следующие шаги

Для полноценной интеграции с реальной системой нужно:

1. **Подключить к базе данных** (PostgreSQL/SQLite)
   - Persistent хранение хостов и сессий
   - История логов

2. **Интеграция с relay-server**
   - API должен общаться с relay-server для получения реальных данных о сессиях
   - Мониторинг активных подключений

3. **Интеграция с server-agent**
   - Получение реальных метрик (CPU, RAM, сетевой трафик)
   - Список подключённых агентов

4. **Реальная аутентификация**
   - PAM интеграция
   - SSH ключи
   - JWT токены для сессий

5. **WebSocket для real-time обновлений**
   - Вместо polling использовать WebSocket
   - Push уведомления об изменениях

## Тестирование API

```bash
# Health check
curl http://127.0.0.1:3001/api/health

# Получить хосты
curl http://127.0.0.1:3001/api/hosts

# Получить сессии
curl http://127.0.0.1:3001/api/sessions/active

# Логин
curl -X POST http://127.0.0.1:3001/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"host":"192.168.1.100","port":22,"username":"operator","password":"test"}'

# Завершить сессию
curl -X POST http://127.0.0.1:3001/api/sessions/s1/terminate

# Получить логи
curl http://127.0.0.1:3001/api/logs

# Получить настройки
curl http://127.0.0.1:3001/api/settings

# Обновить настройки
curl -X POST http://127.0.0.1:3001/api/settings \
  -H "Content-Type: application/json" \
  -d '{"quality":"high","encoder":"vaapi","fps":60,"audio":true}'
```

## Разработка

### Добавление нового endpoint

1. Добавьте модель в `apps/api-server/src/models.rs`
2. Создайте handler в `apps/api-server/src/handlers.rs`
3. Зарегистрируйте роут в `apps/api-server/src/main.rs`
4. Добавьте метод в `apps/desktop-client/src/services/api.ts`
5. Используйте в компоненте

### CORS

API сервер настроен с `allow_origin(Any)` для разработки. 
В продакшене нужно ограничить до конкретных origin'ов.
