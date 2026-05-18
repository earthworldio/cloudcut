import React, { useEffect, useState, useMemo } from "react";
import { useParams, useNavigate } from "react-router-dom";
import api from "../api/axios";
import { useProjectStore } from "../stores/projectStore";
import { usePlaybackStore } from "../stores/playbackStore";
import {
  ChevronLeft,
  Play,
  Pause,
  SkipBack,
  SkipForward,
  Settings,
  Download,
  Edit2,
  Check,
  X,
} from "lucide-react";
import { Timeline, AssetPool } from "../components";
import { VideoPlayer } from "../components/player/VideoPlayer";
import { ExportModal } from "../components/ExportModal";
import { useKeyboardShortcuts } from "../hooks/useKeyboardShortcuts";
import { toast } from "../lib/swal";

export const EditorPage: React.FC = () => {
  useKeyboardShortcuts();

  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const [loading, setLoading] = useState(true);
  const [isEditingName, setIsEditingName] = useState(false);
  const [tempName, setTempName] = useState("");
  const [showExportModal, setShowExportModal] = useState(false);

  const {
    currentProject,
    loadProject,
    updateProjectNameLocal,
    tracks,
    assets,
  } = useProjectStore();
  const { isPlaying, currentTimeMs, play, pause, seek } = usePlaybackStore();


  /* 1. Global Clock for Playhead movement */
  useEffect(() => {
    let animationFrameId: number;
    let lastTime = performance.now();

    const update = () => {
      const now = performance.now();
      const delta = now - lastTime;
      lastTime = now;

      if (isPlaying) {
        /* ใช้ฟังก์ชันอัปเดตแบบ functional เพื่อเลี่ยงการพึ่งพา currentTimeMs ใน dependency array */
        usePlaybackStore
          .getState()
          .seek(usePlaybackStore.getState().currentTimeMs + delta);
      }
      animationFrameId = requestAnimationFrame(update);
    };

    animationFrameId = requestAnimationFrame(update);
    return () => cancelAnimationFrame(animationFrameId);
  }, [isPlaying]);

  /* 2. Determine Active Clip under Playhead for Preview */
  const activeClipData = useMemo(() => {
    /* เรียงลำดับแทร็กวิดีโอ (Video 2 อยู่บนสุด มีความสำคัญกว่า Video 1) */
    const videoTracks = tracks
      .filter((t) => t.type.toLowerCase() === "video")
      .sort((a, b) => b.label.localeCompare(a.label));

    for (const track of videoTracks) {
      const clip = track.clips.find(
        (c) =>
          currentTimeMs >= c.track_position_ms &&
          currentTimeMs < c.track_position_ms + c.duration_ms,
      );

      if (clip) {
        const asset = assets.find((a) => a.id === clip.asset_id);
        if (asset?.url) {
          /* คำนวณเวลาในไฟล์วิดีโอจริง: เวลาปัจจุบัน - จุดเริ่มบนแทร็ก + จุดเริ่มในวิดีโอ */
          const internalTimeMs =
            currentTimeMs - clip.track_position_ms + clip.in_point_ms;
          return { url: asset.url, timeMs: internalTimeMs };
        }
      }
    }
    return null;
  }, [tracks, assets, currentTimeMs]);

  useEffect(() => {
    if (id) {
      loadProject(id).then(() => setLoading(false));
    }
  }, [id]);

  const handleNameUpdate = async () => {
    if (!id || !tempName.trim() || tempName === currentProject?.name) {
      setIsEditingName(false);
      return;
    }

    try {
      await api.patch(`/projects/${id}`, { name: tempName });
      updateProjectNameLocal(tempName);
      toast.fire({
        icon: "success",
        title: "Project renamed",
      });
    } catch (err) {
      toast.fire({
        icon: "error",
        title: "Failed to rename project",
      });
      setTempName(currentProject?.name || "");
    } finally {
      setIsEditingName(false);
    }
  };

  if (loading) {
    return (
      <div className="h-screen flex items-center justify-center bg-background">
        <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-primary"></div>
      </div>
    );
  }

  return (
    <div className="h-screen flex flex-col bg-background overflow-hidden">
      {/* Top Navigation */}
      <header className="h-12 border-b border-border flex items-center justify-between px-4 bg-card">
        <div className="flex items-center gap-4">
          <button
            onClick={() => navigate("/dashboard")}
            className="p-1 hover:bg-muted rounded transition-colors"
          >
            <ChevronLeft className="w-5 h-5" />
          </button>

          <div className="flex items-center gap-2 group">
            {isEditingName ? (
              <div className="flex items-center gap-1">
                <input
                  autoFocus
                  className="bg-input border border-primary px-2 py-0.5 rounded text-sm outline-none w-64"
                  value={tempName}
                  onChange={(e) => setTempName(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") handleNameUpdate();
                    if (e.key === "Escape") {
                      setTempName(currentProject?.name || "");
                      setIsEditingName(false);
                    }
                  }}
                />
                <button
                  onClick={handleNameUpdate}
                  className="p-1 hover:bg-green-500/20 text-green-500 rounded transition-colors"
                  title="Save"
                >
                  <Check className="w-4 h-4" />
                </button>
                <button
                  onClick={() => {
                    setTempName(currentProject?.name || "");
                    setIsEditingName(false);
                  }}
                  className="p-1 hover:bg-red-500/20 text-red-500 rounded transition-colors"
                  title="Cancel"
                >
                  <X className="w-4 h-4" />
                </button>
              </div>
            ) : (
              <div
                className="flex items-center gap-2 cursor-pointer hover:bg-muted px-2 py-0.5 rounded transition-colors"
                onClick={() => setIsEditingName(true)}
              >
                <h1 className="font-semibold">{currentProject?.name}</h1>
                <Edit2 className="w-3 h-3 text-muted-foreground opacity-0 group-hover:opacity-100 transition-opacity" />
              </div>
            )}
          </div>

          <div className="text-xs text-muted-foreground bg-muted px-2 py-0.5 rounded">
            {currentProject?.settings.resolution} @{" "}
            {currentProject?.settings.fps}fps
          </div>
        </div>

        <div className="flex items-center gap-2">
          <button className="flex items-center gap-2 px-3 py-1 bg-secondary text-secondary-foreground text-sm rounded hover:opacity-90">
            <Settings className="w-4 h-4" /> Settings
          </button>
          <button
            onClick={() => setShowExportModal(true)}
            className="flex items-center gap-2 px-3 py-1 bg-primary text-primary-foreground text-sm font-semibold rounded hover:opacity-90"
          >
            <Download className="w-4 h-4" /> Export
          </button>
        </div>
      </header>

      {/* Main Grid Area */}
      <div className="flex-1 grid grid-cols-12 gap-0 overflow-hidden">
        {/* Left: Media Pool */}
        <div className="col-span-3 border-r border-border bg-card">
          <AssetPool />
        </div>

        {/* Center: Video Preview */}
        <div className="col-span-6 bg-black flex flex-col relative">
          <div className="flex-1 flex items-center justify-center p-8">
            <VideoPlayer
              src={activeClipData?.url}
              offsetMs={activeClipData?.timeMs}
            />
          </div>

          {/* Playback Controls */}
          <div className="h-16 bg-card border-t border-border flex items-center justify-center gap-6">
            <button
              onClick={() => seek(0)}
              className="p-2 hover:bg-muted rounded-full transition-colors"
            >
              <SkipBack className="w-5 h-5" />
            </button>
            <button
              onClick={isPlaying ? pause : play}
              className="p-3 bg-primary text-primary-foreground rounded-full hover:opacity-90 shadow-lg"
            >
              {isPlaying ? (
                <Pause className="w-6 h-6 fill-current" />
              ) : (
                <Play className="w-6 h-6 fill-current" />
              )}
            </button>
            <button className="p-2 hover:bg-muted rounded-full transition-colors">
              <SkipForward className="w-5 h-5" />
            </button>

            <div className="absolute right-6 text-xl font-mono text-primary tabular-nums">
              {formatTime(currentTimeMs)}
            </div>
          </div>
        </div>

        {/* Right: Inspector */}
        <div className="col-span-3 border-l border-border bg-card flex flex-col">
          <div className="p-3 border-b border-border">
            <h2 className="text-sm font-bold uppercase tracking-wider text-muted-foreground">
              Inspector
            </h2>
          </div>
          <div className="p-4 space-y-6 overflow-y-auto">
            <div className="text-center py-20 text-muted-foreground text-sm">
              Select a clip to edit its properties
            </div>
          </div>
        </div>
      </div>

      {/* Bottom: Timeline Area */}
      <div className="h-1/3 border-t border-border flex flex-col bg-card/50">
        <Timeline />
      </div>

      {/* Export Modal */}
      {showExportModal && id && (
        <ExportModal projectId={id} onClose={() => setShowExportModal(false)} />
      )}
    </div>
  );
};

/* Helper: Format milliseconds to HH:MM:SS:mm */
function formatTime(ms: number) {
  const hours = Math.floor(ms / 3600000);
  const minutes = Math.floor((ms % 3600000) / 60000);
  const seconds = Math.floor((ms % 60000) / 1000);
  const mil = Math.floor((ms % 1000) / 10);

  return `${hours.toString().padStart(2, "0")}:${minutes.toString().padStart(2, "0")}:${seconds.toString().padStart(2, "0")}:${mil.toString().padStart(2, "0")}`;
}
