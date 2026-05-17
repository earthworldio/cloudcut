import React from "react";
import { useProjectStore } from "../stores/projectStore";
import { ZoomIn, ZoomOut, Scissors, Trash2, Move } from "lucide-react";
import type { Clip } from "../types";

export const Timeline: React.FC = () => {
  const {
    tracks,
    zoomScale,
    setZoomScale,
    currentTimeMs,
    setCurrentTime,
    selectedClipId,
    selectClip,
  } = useProjectStore();

  const handleZoom = (delta: number) => {
    setZoomScale(Math.max(0.01, Math.min(2, zoomScale + delta)));
  };

  return (
    <div className="flex flex-col h-full select-none">
      {/* Timeline Toolbar */}
      <div className="h-10 border-b border-border flex items-center justify-between px-4 bg-muted/30">
        <div className="flex items-center gap-4">
          <button className="p-1 hover:bg-muted rounded text-muted-foreground hover:text-primary">
            <Scissors className="w-4 h-4" />
          </button>
          <button className="p-1 hover:bg-muted rounded text-muted-foreground hover:text-primary">
            <Trash2 className="w-4 h-4" />
          </button>
          <div className="h-4 w-px bg-border mx-2" />
          <button className="p-1 hover:bg-muted rounded text-muted-foreground hover:text-primary">
            <Move className="w-4 h-4" />
          </button>
        </div>

        <div className="flex items-center gap-2">
          <button
            onClick={() => handleZoom(-0.02)}
            className="p-1 hover:bg-muted rounded"
          >
            <ZoomOut className="w-4 h-4" />
          </button>
          <input
            type="range"
            min="0.01"
            max="0.5"
            step="0.01"
            value={zoomScale}
            onChange={(e) => setZoomScale(parseFloat(e.target.value))}
            className="w-24 accent-primary"
          />
          <button
            onClick={() => handleZoom(0.02)}
            className="p-1 hover:bg-muted rounded"
          >
            <ZoomIn className="w-4 h-4" />
          </button>
        </div>
      </div>

      {/* Timeline Grid */}
      <div className="flex-1 overflow-auto relative bg-zinc-950/50">
        {/* Playhead Line */}
        <div
          className="absolute top-0 bottom-0 w-0.5 bg-red-500 z-30 pointer-events-none"
          style={{ left: `${currentTimeMs * zoomScale}px` }}
        >
          <div className="w-3 h-3 bg-red-500 rounded-full -ml-[5px] -mt-1" />
        </div>

        {/* Time Ruler (Scaffold) */}
        <div className="h-6 border-b border-border relative bg-card/80 sticky top-0 z-20">
          {Array.from({ length: 50 }).map((_, i) => (
            <div
              key={i}
              className="absolute top-0 bottom-0 border-l border-zinc-800 text-[10px] pl-1 text-zinc-500"
              style={{ left: `${i * 1000 * zoomScale}px` }}
            >
              {i}s
            </div>
          ))}
        </div>

        {/* Tracks List */}
        <div className="min-w-max pb-20">
          {tracks.map((track) => (
            <div
              key={track.id}
              className="h-20 border-b border-zinc-900/50 relative flex items-center group hover:bg-zinc-900/30 transition-colors"
            >
              {/* Track Label Side (Scaffold) */}
              <div className="sticky left-0 w-40 h-full bg-zinc-900/90 border-r border-zinc-800 z-10 flex flex-col justify-center px-3 shadow-xl">
                <span className="text-xs font-medium truncate">
                  {track.label}
                </span>
                <span className="text-[10px] text-zinc-500 uppercase">
                  {track.type}
                </span>
              </div>

              {/* Clips Container */}
              <div className="flex-1 h-full relative">
                {track.clips.map((clip) => (
                  <TimelineClip
                    key={clip.id}
                    clip={clip}
                    zoomScale={zoomScale}
                    isSelected={selectedClipId === clip.id}
                    onSelect={() => selectClip(clip.id)}
                  />
                ))}
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
};

interface TimelineClipProps {
  clip: Clip;
  zoomScale: number;
  isSelected: boolean;
  onSelect: () => void;
}

const TimelineClip: React.FC<TimelineClipProps> = ({
  clip,
  zoomScale,
  isSelected,
  onSelect,
}) => {
  const widthPx = clip.duration_ms * zoomScale;
  const leftPx = clip.track_position_ms * zoomScale;

  return (
    <div
      onClick={(e) => {
        e.stopPropagation();
        onSelect();
      }}
      className={`
        absolute top-2 bottom-2 rounded border transition-all cursor-pointer overflow-hidden
        ${
          isSelected
            ? "bg-blue-500/30 border-blue-400 ring-2 ring-blue-500/20 z-10"
            : "bg-zinc-800 border-zinc-700 hover:bg-zinc-700 hover:border-zinc-600"
        }
      `}
      style={{ left: `${leftPx}px`, width: `${widthPx}px` }}
    >
      <div className="px-2 py-1 text-[11px] font-medium truncate">
        {clip.name}
      </div>

      {/* Trim Handles (Scaffold) */}
      <div className="absolute left-0 top-0 bottom-0 w-1 hover:bg-white/20 cursor-ew-resize" />
      <div className="absolute right-0 top-0 bottom-0 w-1 hover:bg-white/20 cursor-ew-resize" />
    </div>
  );
};
