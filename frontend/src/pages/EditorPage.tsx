import React, { useEffect, useState } from "react";
import { useParams, useNavigate } from "react-router-dom";
import api from "../api/axios";
import type { TimelineData } from "../types";
import { useProjectStore } from "../stores/projectStore";
import {
  ChevronLeft,
  Play,
  Pause,
  SkipBack,
  SkipForward,
  Layers,
  Settings,
  Download,
  Search,
  Plus,
} from "lucide-react";
import { Timeline } from "../components";

export const EditorPage: React.FC = () => {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const [loading, setLoading] = useState(true);

  const { currentProject, setTimeline, currentTimeMs } = useProjectStore();

  useEffect(() => {
    if (id) {
      fetchTimeline(id);
    }
  }, [id]);

  const fetchTimeline = async (projectId: string) => {
    try {
      const response = await api.get<TimelineData>(
        `/projects/${projectId}/timeline`,
      );
      setTimeline(response.data);
    } catch (err) {
      console.error("Failed to load timeline", err);
      navigate("/dashboard");
    } finally {
      setLoading(false);
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
          <h1 className="font-semibold">{currentProject?.name}</h1>
          <div className="text-xs text-muted-foreground bg-muted px-2 py-0.5 rounded">
            {currentProject?.settings.resolution} @{" "}
            {currentProject?.settings.fps}fps
          </div>
        </div>

        <div className="flex items-center gap-2">
          <button className="flex items-center gap-2 px-3 py-1 bg-secondary text-secondary-foreground text-sm rounded hover:opacity-90">
            <Settings className="w-4 h-4" /> Settings
          </button>
          <button className="flex items-center gap-2 px-3 py-1 bg-primary text-primary-foreground text-sm font-semibold rounded hover:opacity-90">
            <Download className="w-4 h-4" /> Export
          </button>
        </div>
      </header>

      {/* Main Grid Area */}
      <div className="flex-1 grid grid-cols-12 gap-0 overflow-hidden">
        {/* Left: Media Pool */}
        <div className="col-span-3 border-r border-border flex flex-col bg-card">
          <div className="p-3 border-b border-border flex items-center justify-between">
            <h2 className="text-sm font-bold uppercase tracking-wider text-muted-foreground flex items-center gap-2">
              <Layers className="w-4 h-4" /> Assets
            </h2>
            <button className="p-1 hover:bg-muted rounded">
              <Plus className="w-4 h-4" />
            </button>
          </div>
          <div className="p-2">
            <div className="relative">
              <Search className="w-4 h-4 absolute left-2 top-2 text-muted-foreground" />
              <input
                placeholder="Search assets..."
                className="w-full bg-input border border-border rounded pl-8 pr-2 py-1 text-sm outline-none focus:ring-1 focus:ring-ring"
              />
            </div>
          </div>
          <div className="flex-1 overflow-y-auto p-4 text-center">
            <div className="border-2 border-dashed border-border rounded-lg py-10">
              <p className="text-sm text-muted-foreground">No assets yet.</p>
              <button className="mt-2 text-xs text-primary hover:underline">
                Upload media
              </button>
            </div>
          </div>
        </div>

        {/* Center: Video Preview */}
        <div className="col-span-6 bg-black flex flex-col relative">
          <div className="flex-1 flex items-center justify-center p-8">
            <div className="aspect-video bg-zinc-900 w-full shadow-2xl flex items-center justify-center border border-zinc-800">
              <div className="text-zinc-700 text-lg italic">
                Preview Monitor
              </div>
            </div>
          </div>

          {/* Playback Controls */}
          <div className="h-16 bg-card border-t border-border flex items-center justify-center gap-6">
            <button className="p-2 hover:bg-muted rounded-full transition-colors">
              <SkipBack className="w-5 h-5" />
            </button>
            <button className="p-3 bg-primary text-primary-foreground rounded-full hover:opacity-90 shadow-lg">
              <Play className="w-6 h-6 fill-current" />
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
    </div>
  );
};

// Helper: Format milliseconds to HH:MM:SS:mm
function formatTime(ms: number) {
  const hours = Math.floor(ms / 3600000);
  const minutes = Math.floor((ms % 3600000) / 60000);
  const seconds = Math.floor((ms % 60000) / 1000);
  const mil = Math.floor((ms % 1000) / 10);

  return `${hours.toString().padStart(2, "0")}:${minutes.toString().padStart(2, "0")}:${seconds.toString().padStart(2, "0")}:${mil.toString().padStart(2, "0")}`;
}
