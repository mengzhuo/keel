#!/bin/sh
set -e
export PATH=/usr/bin:/bin
ffmpeg -version | head -1
ffmpeg -hide_banner -loglevel error -f lavfi -i testsrc=size=64x64:rate=5:duration=0.4 -pix_fmt rgb24 -f md5 -
ffmpeg -hide_banner -loglevel error -f lavfi -i sine=frequency=440:duration=0.2 -f md5 -
ffmpeg -hide_banner -loglevel error -y -f lavfi -i testsrc=size=64x64:rate=5:duration=0.4 -c:v mpeg4 -threads 2 /tmp/ff.avi
ffprobe -v error -show_entries stream=codec_name,width,height -of csv=p=0 /tmp/ff.avi
ffmpeg -hide_banner -loglevel error -y -i /tmp/ff.avi -pix_fmt rgb24 -f md5 -
ffmpeg -hide_banner -loglevel error -y -f lavfi -i sine=frequency=440:duration=0.2 -c:a pcm_s16le /tmp/ff.wav
ffmpeg -hide_banner -loglevel error -y -i /tmp/ff.wav -f md5 -
echo FFMPEG_OK
