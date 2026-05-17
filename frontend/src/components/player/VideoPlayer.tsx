import React, { useRef, useEffect } from "react";
import { usePlaybackStore } from "../../stores/playbackStore";

interface VideoPlayerProps {
  src?: string;
  offsetMs?: number;
}

export const VideoPlayer: React.FC<VideoPlayerProps> = ({ src, offsetMs }) => {
  const videoRef = useRef<HTMLVideoElement>(null);
  const { isPlaying, currentTimeMs, seek, volume, isMuted } =
    usePlaybackStore();

  /* 1. Sync Play/Pause */
  useEffect(() => {
    if (!videoRef.current || !src) return;
    if (isPlaying) {
      videoRef.current
        .play()
        .catch((e) => console.warn("Video play interrupted", e));
    } else {
      videoRef.current.pause();
    }
  }, [isPlaying, src]);

  /* 2. Sync Time (Specific for this clip) */
  useEffect(() => {
    if (!videoRef.current || offsetMs === undefined || !src) return;

    const videoTime = videoRef.current.currentTime * 1000;
    /* หากเวลาคลาดเคลื่อนเกิน 100ms ให้ปรับจูน */
    if (Math.abs(videoTime - offsetMs) > 100) {
      videoRef.current.currentTime = offsetMs / 1000;
    }
  }, [offsetMs, src]);

  /* Sync Volume/Mute */
  useEffect(() => {
    if (!videoRef.current) return;
    videoRef.current.volume = volume;
    videoRef.current.muted = isMuted;
  }, [volume, isMuted]);

  const handleTimeUpdate = () => {
    /* 
      If we have a global clock driving currentTimeMs, 
      We only sync from video if the drift is large,
      Otherwise the video might stutter due to rapid state updates.
    */
    if (!videoRef.current || !isPlaying) return;
    const videoTimeMs = videoRef.current.currentTime * 1000;
    if (Math.abs(videoTimeMs - currentTimeMs) > 150) {
      seek(videoTimeMs);
    }
  };

  return (
    <div className="relative w-full h-full bg-black flex items-center justify-center overflow-hidden rounded-lg border border-border">
      {src ? (
        <video
          ref={videoRef}
          src={src}
          className="max-w-full max-h-full"
          onTimeUpdate={handleTimeUpdate}
          onEnded={() => usePlaybackStore.getState().pause()}
        />
      ) : (
        <div className="text-muted-foreground flex flex-col items-center gap-2">
          <div className="w-12 h-12 rounded-full border-2 border-dashed border-muted-foreground flex items-center justify-center">
            <span className="text-xs">?</span>
          </div>
          <span className="text-xs">No media selected</span>
        </div>
      )}
    </div>
  );
};
