# svg2librepcb

A small program to generate LibrePCB packages from an SVG file.

**WORK IN PROGRESS!**

## SVG Constraints

- Only paths are considered, without transformations or style.
- If you have an object that consists of outer and inner paths (e.g. a donut
  shape), you need to join the inner and outer path.

## Inkscape Extension

You can use this program as an Inkscape extension. Simply create a release
build and copy the binary and the extension file to the Inkscape extension
directory:

    cargo build --release
    cp target/release/svg2librepcb ~/.config/inkscape/extensions/
    cp inkscape/svg2librepcb.inx ~/.config/inkscape/extensions/

Alternatively, during development, you can also symlink the two files:

    cargo build
    ln -s $(pwd)/target/debug/svg2librepcb ~/.config/inkscape/extensions/
    ln -s $(pwd)/inkscape/svg2librepcb.inx ~/.config/inkscape/extensions/

Then, launch the extension through "Extensions > Export > Export to LibrePCB".
