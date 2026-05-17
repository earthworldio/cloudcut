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

    const syncTime = () => {
      if (!videoRef.current) return;
      const videoTime = videoRef.current.currentTime * 1000;
      if (Math.abs(videoTime - offsetMs) > 100) {
        videoRef.current.currentTime = offsetMs / 1000;
      }
    };

    syncTime();
  }, [offsetMs, src]);

  /* Sync Volume/Mute */
  useEffect(() => {
    if (!videoRef.current) return;
    videoRef.current.volume = volume;
    videoRef.current.muted = isMuted;
  }, [volume, isMuted]);

  return (
    <div className="relative w-full h-full bg-black flex items-center justify-center overflow-hidden rounded-lg border border-border">
      <video
        ref={videoRef}
        src={src}
        className={`max-w-full max-h-full ${src ? "block" : "hidden"}`}
        playsInline
        onLoadedMetadata={(e) => {
          if (offsetMs !== undefined) {
            e.currentTarget.currentTime = offsetMs / 1000;
          }
        }}
        onEnded={() => {
          /* 
             We don't pause here because the timeline should continue 
             to the next clip or empty space.
          */
        }}
      />
      {!src && (
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
