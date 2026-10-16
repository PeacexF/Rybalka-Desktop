# Рыбалка Desktop

Фанатский офлайн-ремейк классической «Русской Рыбалки 2/3» на **Rust + Tauri 2**.
2D-панорамы водоёмов, поплавок, донка/фидер и спиннинг, магазин, путёвки и базы, рекорды.

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

## Инструменты разработчика

```sh
# Отчёт по балансу: интервалы поклёвок по снастям/наживкам/часам и исходы вываживания
cargo run -p rybalka-devtools --release --bin balance -- pond willow

# Подготовка фото водоёма (кроп 16:9 или --aspect 2.2 для панорамы, высота 1080) и сетка координат
cargo run -p rybalka-devtools --release --bin photo -- prepare in.jpg out.jpg --crop-top 0.15
cargo run -p rybalka-devtools --release --bin photo -- grid out.jpg grid.jpg

# Карта глубин из спеки map.ron (+ превью наложения на фото)
cargo run -p rybalka-devtools --release --bin gen_map -- \
  assets/locations/pond/willow/map.ron assets/locations/pond/willow/map.png \
  --preview assets/locations/pond/willow/bg.jpg /tmp/preview.jpg
```

- `F3` в игре: оверлей с частотой поклёвок по видам и кнопки времени/погоды.
- `F4` в dev-сборке: редактор карты водоёма поверх фото — кисти воды/суши, глубины, дна и зон,
  калибровка перспективы (горизонт, берег, дистанция, ширина обзора) и сохранение `map.png`
  прямо в `assets/`. `gen_map` перезаписывает ручные правки, поэтому крупные изменения лучше
  вносить в `map.ron`, а редактором — подправлять и подбирать калибровку.
- Широкие панорамы листаются стрелками / `A` `D` или мышью у края экрана.
- `RYBALKA_DEV_START="21.5,Rain"`: новая сессия стартует в этот час и с этой погодой.
- Ошибки фронтенда пишутся в stderr процесса (`[ui:error] …`), в том числе в релизной сборке.
- Сохранение — SQLite в `app_data_dir/save.sqlite` (macOS: `~/Library/Application Support/dev.peacexf.rybalka/`), автосейв каждые 3 с и при выходе. Удалите файл, чтобы начать с чистого листа.

## Лицензия

MIT, см. [LICENSE](LICENSE). Атрибуция сторонних ассетов — в `assets/CREDITS.md`.
