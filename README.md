# svg2librepcb

A small program to generate LibrePCB packages from an SVG file.

## SVG Constraints

- Only paths are considered, without transformations or style.
- If you have an object that consists of outer and inner paths (e.g. a donut
  shape), you need to join the inner and outer path.
