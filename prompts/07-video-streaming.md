# Prompt 07 — Video Streaming Pipeline

Implement the video pipeline for remote graphical sessions.

## Goal

Stream an X11 session to the client with low latency.

## MVP Target

- 1080p
- 30 FPS
- adaptive quality
- low latency priority
- H.264
- VAAPI if available
- software fallback required

## Required Crate

- `crates/video-pipeline`

## Required Interfaces

Create:

- `CaptureSource`
- `VideoEncoder`
- `EncodedFrame`
- `VideoStream`
- `BitrateController`
- `FrameClock`
- `VideoMetrics`

## Backend Requirements

- X11 capture backend;
- mock frame generator;
- GStreamer encoder backend;
- software fallback encoder.

## Tests

- mock stream test;
- frame timing test;
- bitrate controller test;
- encoder fallback test;
- performance benchmark skeleton.

## Documentation

Create:
- `docs/video-pipeline.md`
