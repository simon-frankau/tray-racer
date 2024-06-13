#/bin/sh

set -e

cargo build --release --bin tray-racer-cli

mkdir -p pan-imgs

for PAN in $(seq -f %03g 0 3 357)
do
    echo $PAN
    target/release/tray-racer-cli --pan ${PAN} -w 1280 -h 960 -o pan-imgs/pan-${PAN}.png
done

ffmpeg -r 30 -f image2 -pattern_type glob -i 'pan-imgs/pan-*.png' -vf scale="iw/2:ih/2" -vcodec libx264 -crf 25 -pix_fmt yuv420p pan.mp4
