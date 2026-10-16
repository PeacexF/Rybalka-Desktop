# Рыбалка Desktop

Фанатский офлайн-ремейк классической «Русской Рыбалки 2/3» на **Rust + Tauri 2**.
2D-панорамы водоёмов, поплавок, донка/фидер и спиннинг, магазин, путёвки, рекорды.

> Некоммерческий фан-проект. Ассеты, названия и тексты оригинальной игры не используются.

## Стек

- `crates/core` — вся игровая логика и симуляция (чистый Rust, без Tauri)
- `src-tauri` — приложение Tauri: игровой поток (20 тиков/с), команды, сохранения
- `ui` — Vite + React + TypeScript + PixiJS (рендер сцены и интерфейс)
- `content` — данные игры (RON): рыба, наживки, снасти, водоёмы

## Запуск

Нужны Rust (stable), Node.js 22+ и [системные зависимости Tauri](https://tauri.app/start/prerequisites/).

```sh
npm install
npm run tauri dev      # dev-сборка с hot reload фронта
npm run tauri build    # релизный бандл
```

## Проверки

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm run lint && npm run typecheck && npm run format:check
```

TypeScript-типы для IPC генерируются из Rust через `ts-rs` в `ui/src/ipc/bindings`
при `cargo test` (или `npm run bindings`). CI проверяет, что они закоммичены актуальными.

## Лицензия

MIT, см. [LICENSE](LICENSE). Атрибуция сторонних ассетов — в `assets/CREDITS.md`.
