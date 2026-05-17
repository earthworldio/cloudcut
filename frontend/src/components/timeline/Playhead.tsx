import React, { useRef, useCallback } from "react";
import { usePlaybackStore } from "../../stores/playbackStore";
import { useUIStore } from "../../stores/uiStore";

export const Playhead: React.FC = () => {
  const currentTimeMs = usePlaybackStore((state) => state.currentTimeMs);
  const seek = usePlaybackStore((state) => state.seek);
  const zoomLevel = useUIStore((state) => state.zoomLevel);

  const playheadRef = useRef<HTMLDivElement>(null);

  const x = (currentTimeMs / 1000) * zoomLevel;

  const handleMouseDown = useCallback(
    (e: React.MouseEvent) => {
      const startX = e.clientX;
      const startMs = currentTimeMs;

      const onMouseMove = (moveEvent: MouseEvent) => {
        const deltaX = moveEvent.clientX - startX;
        const deltaMs = (deltaX / zoomLevel) * 1000;
        seek(Math.max(0, startMs + deltaMs));
      };

      const onMouseUp = () => {
        window.removeEventListener("mousemove", onMouseMove);
        window.removeEventListener("mouseup", onMouseUp);
      };

      window.addEventListener("mousemove", onMouseMove);
      window.addEventListener("mouseup", onMouseUp);
    },
    [currentTimeMs, zoomLevel, seek],
  );

  return (
    <div
      ref={playheadRef}
      className="absolute top-0 bottom-0 w-px bg-red-500 z-50 cursor-ew-resize group"
      style={{ left: x }}
      onMouseDown={handleMouseDown}
    >
      {/* Handle */}
      <div className="absolute -top-1 -left-1.5 w-3 h-3 bg-red-500 rotate-45" />

      {/* Visual line */}
      <div className="absolute inset-y-0 -left-[0.5px] w-[2px] bg-red-500/30 group-hover:bg-red-500/50" />
    </div>
  );
};
