# FFmpeg

**Status: ✅ passes under keel**

| | |
|---|---|
| Package | `ffmpeg-8.0.1-r1` (Alpine 3.23) |
| Version | `ffmpeg version 8.0.1 Copyright (c) 2000-2025 the FFmpeg developers` |
| Baseline | same package with stock musl 1.2.6 |

## What was tested

FFmpeg loads `libavcodec`, `libavformat`, `libavfilter`, `libavutil`,
`libswresample` and `libswscale`, starts worker threads for encode/decode,
allocates aligned frame buffers and uses SIMD dispatch, `mmap`, `poll` and
`clock_gettime`.

- `ffmpeg -version`;
- generate a deterministic `testsrc` video and hash the raw frames with the
  `md5` muxer;
- generate a 440 Hz sine tone and hash the PCM samples;
- encode an MPEG-4 Part 2 AVI with `-threads 2`;
- inspect it with `ffprobe`;
- decode the AVI back to raw RGB and hash it;
- round-trip PCM through a WAV file.

## Commands

```sh
export PATH=/usr/bin:/bin
ffmpeg -version | head -1
ffmpeg -hide_banner -loglevel error \
  -f lavfi -i testsrc=size=64x64:rate=5:duration=0.4 \
  -pix_fmt rgb24 -f md5 -
ffmpeg -hide_banner -loglevel error \
  -f lavfi -i sine=frequency=440:duration=0.2 -f md5 -
ffmpeg -hide_banner -loglevel error -y \
  -f lavfi -i testsrc=size=64x64:rate=5:duration=0.4 \
  -c:v mpeg4 -threads 2 /tmp/ff.avi
ffprobe -v error -show_entries stream=codec_name,width,height \
  -of csv=p=0 /tmp/ff.avi
ffmpeg -hide_banner -loglevel error -y -i /tmp/ff.avi \
  -pix_fmt rgb24 -f md5 -
ffmpeg -hide_banner -loglevel error -y \
  -f lavfi -i sine=frequency=440:duration=0.2 -c:a pcm_s16le /tmp/ff.wav
ffmpeg -hide_banner -loglevel error -y -i /tmp/ff.wav -f md5 -
```

## Output under keel

```text
ffmpeg version 8.0.1 Copyright (c) 2000-2025 the FFmpeg developers
MD5=a3d49171f1997201608275441925af88
MD5=b4f27f6fef491b0bc750e9d56eee4d33
mpeg4,64,64
MD5=a7ae3604f59864eb02d54c96a1fbfb7b
MD5=b4f27f6fef491b0bc750e9d56eee4d33
FFMPEG_OK
```

The audio MD5 is the same before and after the WAV round trip, as expected.

## Baseline comparison

Identical output and exit status under musl.

## Notes

- The `testsrc` and `sine` sources are deterministic, and the `md5` muxer
  prints only the digest, so the test does not depend on timestamps or
  container metadata.
- The MPEG-4 encoder is single-threaded internally; `-threads 2` still
  creates FFmpeg's frame-threading context, which exercises pthread creation
  and TLS.
- Hardware acceleration, network protocols and real-world media containers
  were not exercised.
