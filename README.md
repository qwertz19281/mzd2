# mzd2

dungeon sketch tool (WIP)

## Install

```console
cargo install -f --git https://github.com/qwertz19281/mzd2 --branch release
```

```console
cargo install -f --git https://github.com/qwertz19281/mzd2 --branch master
```

## Goal

Make it easy to sketch maps/dungeons for e.g. action rpg like ALttP or Link's Awakening

## Features

- Extensive abilities to connect/move around images
- Hybrid bitmap-like editing of room (image pixels and selection rectangle)
  - Simple to draw like with bitmap editing
  - Selection rectangles and easier moving of regions
- Multi-layer room editing
- Tags on map, with "warping" ability to quick jump on map or across maps

## Limitations

- Per-map fixed room size (grid of rooms)
- Each map currently limited to 256x256x256 rooms
- 8 pixel quantization
