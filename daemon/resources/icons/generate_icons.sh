# This script requires inkscape installed to generate the various sized icons for this app, we need the following
# icon list (and location):
#    File: pipeweaver.svg, Size: 512x512 Target: /usr/share/icons/hicolor/scalable/apps/pipeweaver.svg
#    File: pipeweaver.png, Size: 48x48, Target: /usr/share/icons/hicolor/48x48/apps/pipeweaver.png
#    File: pipeweaver-large.png, Size: 128x128, Target: /usr/share/pixmaps/pipeweaver.png

inkscape pipeweaver.svg --export-filename=pipeweaver-large.png -w 128 -h 128
inkscape pipeweaver.svg --export-filename=pipeweaver.png -w 48 -h 48

## Note, this requires imagemagick to be installed to create the various Windows icon sizes from SVG..
mkdir tmp/
FILE_PATHS=""
for size in 16 32 48 128 256; do
  FILE_NAME="tmp/$size.png"
  inkscape pipeweaver.svg --export-filename=$FILE_NAME -w $size -h $size
  FILE_PATHS="${FILE_PATHS} ${FILE_NAME}"
done
magick $FILE_PATHS goxlr-utility.ico
rm -r tmp/