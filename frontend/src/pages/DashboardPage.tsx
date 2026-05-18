import React, { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import api from "../api/axios";
import type { Project } from "../types";
import {
  Plus,
  Video,
  LogOut,
  Layout,
  Trash2,
  X,
  AlertTriangle,
} from "lucide-react";
import { useAuthStore } from "../stores/authStore";
import { useProjectStore } from "../stores/projectStore";
import { alert, toast } from "../lib/swal";

export const DashboardPage: React.FC = () => {
  const [projects, setProjects] = useState<Project[]>([]);
  const [loading, setLoading] = useState(true);
  const navigate = useNavigate();
  const { user, logout } = useAuthStore();
  const { deleteProject, deleteWorkspace } = useProjectStore();

  useEffect(() => {
    fetchProjects();
  }, []);

  const fetchProjects = async () => {
    try {
      const response = await api.get<Project[]>("/projects");
      setProjects(response.data);
    } catch (err) {
      console.error("Failed to fetch projects", err);
    } finally {
      setLoading(false);
    }
  };

  const handleDeleteProject = async (
    e: React.MouseEvent,
    projectId: string,
  ) => {
    e.stopPropagation();
    const result = await alert.confirm(
      "Delete Project?",
      "All assets and timeline data will be permanently removed. This cannot be undone.",
    );

    if (result.isConfirmed) {
      try {
        await deleteProject(projectId);
        toast.fire({
          icon: "success",
          title: "Project deleted",
        });
        fetchProjects();
      } catch (err) {
        toast.fire({
          icon: "error",
          title: "Failed to delete project",
        });
      }
    }
  };

  const handleDeleteWorkspace = async () => {
    if (!user?.workspace_id) return;

    const result = await alert.confirm(
      "Delete Workspace?",
      "All projects and data in this workspace will be PERMANENTLY DELETED. This is a destructive action.",
    );

    if (result.isConfirmed) {
      try {
        await deleteWorkspace(user.workspace_id);
        toast.fire({
          icon: "success",
          title: "Workspace deleted",
        });
        logout(); // Logout after workspace deletion
      } catch (err) {
        toast.fire({
          icon: "error",
          title: "Failed to delete workspace",
        });
      }
    }
  };

  const createProject = async () => {
    try {
      /* ดึงข้อมูล user ล่าสุดจาก store (ซึ่งมี workspace_id แล้ว) */
      const currentUser = useAuthStore.getState().user;

      if (!currentUser?.workspace_id) {
        alert.error(
          "No Workspace",
          "We could not find an active workspace for your account. Please try logging in again.",
        );
        return;
      }

      const newProject = await api.post<Project>("/projects", {
        name: "New Project " + new Date().toLocaleTimeString(),
        workspace_id: currentUser.workspace_id,
        description: "Created from dashboard",
      });

      toast.fire({
        icon: "success",
        title: "Project created successfully",
      });

      navigate(`/projects/${newProject.data.id}`);
    } catch (err: any) {
      alert.error(
        "Failed to create project",
        err.response?.data?.message ||
          "Please check if you have an active workspace.",
      );
    }
  };

  return (
    <div className="min-h-screen bg-background">
      {/* Header */}
      <header className="border-b border-border bg-card/50 backdrop-blur-sm sticky top-0 z-10">
        <div className="max-w-7xl mx-auto px-4 h-16 flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Video className="w-6 h-6 text-primary" />
            <span className="text-xl font-bold">CloudCut</span>
          </div>

          <div className="flex items-center gap-4">
            <span className="text-sm text-muted-foreground">
              Welcome, {user?.name}
            </span>
            <div className="h-6 w-px bg-border mx-2" />
            <button
              onClick={handleDeleteWorkspace}
              className="flex items-center gap-2 px-3 py-1.5 text-xs font-medium text-red-500 hover:bg-red-500/10 rounded transition-colors"
              title="Delete Workspace"
            >
              <Trash2 className="w-3.5 h-3.5" /> Delete Workspace
            </button>
            <button
              onClick={logout}
              className="p-2 hover:bg-muted rounded-full transition-colors text-muted-foreground"
              title="Logout"
            >
              <LogOut className="w-5 h-5" />
            </button>
          </div>
        </div>
      </header>

      <main className="max-w-7xl mx-auto px-4 py-8">
        <div className="flex items-center justify-between mb-8">
          <div>
            <h2 className="text-2xl font-bold">My Projects</h2>
            <p className="text-muted-foreground">
              Manage and edit your video projects
            </p>
          </div>
          <button
            onClick={createProject}
            className="flex items-center gap-2 px-4 py-2 bg-primary text-primary-foreground font-semibold rounded hover:opacity-90 transition-opacity"
          >
            <Plus className="w-5 h-5" />
            Create Project
          </button>
        </div>

        {loading ? (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
            {[1, 2, 3].map((i) => (
              <div
                key={i}
                className="h-48 bg-card border border-border rounded-lg animate-pulse"
              />
            ))}
          </div>
        ) : projects.length === 0 ? (
          <div className="text-center py-20 bg-card border border-dashed border-border rounded-lg">
            <Layout className="w-12 h-12 text-muted-foreground mx-auto mb-4" />
            <h3 className="text-lg font-medium">No projects found</h3>
            <p className="text-muted-foreground mb-6">
              Start your first video editing journey today!
            </p>
            <button
              onClick={createProject}
              className="px-6 py-2 border border-primary text-primary rounded hover:bg-primary/10 transition-colors"
            >
              Get Started
            </button>
          </div>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
            {projects.map((project) => (
              <div
                key={project.id}
                onClick={() => navigate(`/projects/${project.id}`)}
                className="group bg-card border border-border rounded-lg overflow-hidden hover:border-primary/50 transition-all cursor-pointer shadow-sm hover:shadow-md relative"
              >
                {/* Delete Button (X) */}
                <button
                  onClick={(e) => handleDeleteProject(e, project.id)}
                  className="absolute top-2 right-2 p-1.5 bg-black/40 text-white hover:bg-red-500 rounded-md opacity-0 group-hover:opacity-100 transition-all z-20"
                  title="Delete Project"
                >
                  <X className="w-4 h-4" />
                </button>

                <div className="h-32 bg-muted flex items-center justify-center group-hover:bg-muted/80 transition-colors">
                  <Video className="w-10 h-10 text-muted-foreground group-hover:scale-110 transition-transform" />
                </div>
                <div className="p-4">
                  <h4 className="font-semibold truncate">{project.name}</h4>
                  <p className="text-sm text-muted-foreground mt-1 line-clamp-1">
                    {project.description || "No description"}
                  </p>
                  <div className="mt-4 text-xs text-muted-foreground">
                    Last updated:{" "}
                    {new Date(project.updated_at).toLocaleDateString()}
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </main>
    </div>
  );
};
