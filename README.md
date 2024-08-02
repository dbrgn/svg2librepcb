# svg2librepcb

A small program to generate LibrePCB packages from an SVG file, along with an
Inkscape extension.

## Building

You need Rust and Cargo installed. Then run:

    cargo build --release

Then you can find the binary at `target/release/svg2librepcb`.

## CLI Usage

Use `svg2librepcb --help` to view the usage help.

Example:

    svg2librepcb \
        --outpath ~/LibrePCB-Workspace/v0.1/libraries/local/MyLibrary.lplib/ \
        --name MyName \
        --author Danilo \
        logo.svg

This will result in the following files being generated in the output directory:

    cmp/2288621c-6056-4531-90d1-21e9f6f72175/
    ├── component.lp
    └── .librepcb-cmp
    dev/f9207d36-6bc1-41f9-a61c-cf1f9657b8e3/
    ├── device.lp
    └── .librepcb-dev
    pkg/8d92aac5-2fe0-460c-baad-35e9361d5f79/
    ├── .librepcb-pkg
    └── package.lp
    sym/c1fbc16a-a380-4387-aee7-a3facd5f50aa/
    ├── .librepcb-sym
    └── symbol.lp

## SVG Constraints

- Only paths are considered, without transformations or style.
- If you have an object that consists of outer and inner paths (e.g. a donut
  shape), you need to join the inner and outer path. The "Cut Closed Loops" section of [this](https://learn.sparkfun.com/tutorials/importing-custom-images-into-eagle/method-1-svg-to-polygon#:~:text=pop%2Dup%20window-,Cut%20Closed%20Loops,-Any%20shapes%20that) SparkFun tutorial can help with that.

## Inkscape Extension

You can use this program as an Inkscape extension:

![Screenshot](inkscape/screenshot.png)

To use it, create a release build and copy the binary and the extension file to
the Inkscape extension directory:

    cargo build --release
    cp target/release/svg2librepcb ~/.config/inkscape/extensions/
    cp inkscape/svg2librepcb.inx ~/.config/inkscape/extensions/

Alternatively, during development, you can also symlink the two files:

    cargo build
    ln -s $(pwd)/target/debug/svg2librepcb ~/.config/inkscape/extensions/
    ln -s $(pwd)/inkscape/svg2librepcb.inx ~/.config/inkscape/extensions/

Then, launch the extension through "Extensions > Export > Export to LibrePCB".
