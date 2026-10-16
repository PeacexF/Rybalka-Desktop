# Assets & credits

Every third-party file in `assets/` must be listed here with its source, author and licence.
Only CC0 / public-domain (or CC-BY with attribution below) material is accepted.

## Location photos

Prepared with `cargo run -p rybalka-devtools --release --bin photo -- prepare <in> <out> [--crop-top F]`
(cropped to 16:9, or a wider panorama with `--aspect`, resized to 1080 px high).

| File | Original | Author | Licence |
|---|---|---|---|
| `locations/pond/willow/bg.jpg` | [Пруд в посёлке Гжелка 2021-07.jpg](https://commons.wikimedia.org/wiki/File:%D0%9F%D1%80%D1%83%D0%B4_%D0%B2_%D0%BF%D0%BE%D1%81%D1%91%D0%BB%D0%BA%D0%B5_%D0%93%D0%B6%D0%B5%D0%BB%D0%BA%D0%B0_2021-07.jpg) (`--crop-top 0.15`) | Artyom Svetlov | CC0 1.0 |
| `locations/pond/reeds/bg.jpg` | [Quiet Pond (34271500641).jpg](https://commons.wikimedia.org/wiki/File:Quiet_Pond_(34271500641).jpg) | Ontheway Advice | CC0 1.0 |
| `locations/lake/jetty/bg.jpg` | [Pursujärvi lake in Renko Finland.jpg](https://commons.wikimedia.org/wiki/File:Pursuj%C3%A4rvi_lake_in_Renko_Finland.jpg) | Kotivalo | CC0 1.0 |
| `locations/lake/reeds/bg.jpg` | [Lake Iso-Kuhanjärvi.jpg](https://commons.wikimedia.org/wiki/File:Lake_Iso-Kuhanj%C3%A4rvi.jpg) | Gebinam | CC0 1.0 |
| `locations/lake/meadow/bg.jpg` | [Lake Kalhujärvi.jpg](https://commons.wikimedia.org/wiki/File:Lake_Kalhuj%C3%A4rvi.jpg) | SeppVei | CC0 1.0 |
| `locations/river/sand/bg.jpg` | [Река Пушма.jpg](https://commons.wikimedia.org/wiki/File:%D0%A0%D0%B5%D0%BA%D0%B0_%D0%9F%D1%83%D1%88%D0%BC%D0%B0.jpg) (`--crop-top 0.1`) | Danil43 21 | CC0 1.0 |
| `locations/river/reach/bg.jpg` | [Река Пра перед Клепиками.JPG](https://commons.wikimedia.org/wiki/File:%D0%A0%D0%B5%D0%BA%D0%B0_%D0%9F%D1%80%D0%B0_%D0%BF%D0%B5%D1%80%D0%B5%D0%B4_%D0%9A%D0%BB%D0%B5%D0%BF%D0%B8%D0%BA%D0%B0%D0%BC%D0%B8.JPG) (`--aspect 2.1 --crop-top 0.18`) | Ullus | CC0 1.0 |
| `locations/reservoir/open/bg.jpg` | [Воткинское водохранилище - верхний бьеф.JPG](https://commons.wikimedia.org/wiki/File:%D0%92%D0%BE%D1%82%D0%BA%D0%B8%D0%BD%D1%81%D0%BA%D0%BE%D0%B5_%D0%B2%D0%BE%D0%B4%D0%BE%D1%85%D1%80%D0%B0%D0%BD%D0%B8%D0%BB%D0%B8%D1%89%D0%B5_-_%D0%B2%D0%B5%D1%80%D1%85%D0%BD%D0%B8%D0%B9_%D0%B1%D1%8C%D0%B5%D1%84.JPG) (`--aspect 2.2 --crop-top 0.3`) | Шнапс | CC0 1.0 |
| `locations/reservoir/cape/bg.jpg` | [Дегтярское водохранилище.jpg](https://commons.wikimedia.org/wiki/File:%D0%94%D0%B5%D0%B3%D1%82%D1%8F%D1%80%D1%81%D0%BA%D0%BE%D0%B5_%D0%B2%D0%BE%D0%B4%D0%BE%D1%85%D1%80%D0%B0%D0%BD%D0%B8%D0%BB%D0%B8%D1%89%D0%B5.jpg) | Istepan1988 | CC0 1.0 |
| `locations/reservoir/beach/bg.jpg` | [Lutizh3.JPG](https://commons.wikimedia.org/wiki/File:Lutizh3.JPG) (`--aspect 2.0 --crop-top 0.12`) | Аимаина хикари | CC0 1.0 |

## Water maps

`locations/**/map.png` are generated from the adjacent `map.ron` specs by
`crates/devtools/src/bin/gen_map.rs` and are part of the project (MIT).
