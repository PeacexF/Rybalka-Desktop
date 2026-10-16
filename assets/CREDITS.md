# Assets & credits

Every third-party file in `assets/` must be listed here with its source, author and licence.
Only CC0 / public-domain (or CC-BY with attribution below) material is accepted.

## Location photos

Prepared with `cargo run -p rybalka-devtools --release --bin photo -- prepare <in> <out> [--crop-top F]`
(cropped to 16:9, resized to 1920×1080).

| File | Original | Author | Licence |
|---|---|---|---|
| `locations/pond/willow/bg.jpg` | [Пруд в посёлке Гжелка 2021-07.jpg](https://commons.wikimedia.org/wiki/File:%D0%9F%D1%80%D1%83%D0%B4_%D0%B2_%D0%BF%D0%BE%D1%81%D1%91%D0%BB%D0%BA%D0%B5_%D0%93%D0%B6%D0%B5%D0%BB%D0%BA%D0%B0_2021-07.jpg) (`--crop-top 0.15`) | Artyom Svetlov | CC0 1.0 |

## Water maps

`locations/**/map.png` are generated from the adjacent `map.ron` specs by
`crates/devtools/src/bin/gen_map.rs` and are part of the project (MIT).
