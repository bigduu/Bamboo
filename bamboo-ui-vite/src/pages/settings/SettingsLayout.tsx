import React from "react";
import { Link, Outlet } from "react-router-dom";
import { Settings, Server, Shield, Brain, Wrench, Bot, Database, FileText, MessageSquare, Sparkles, Database as DatabaseIcon, Eye } from "lucide-react";

export default function SettingsLayout() {
  const navItems = [
    { to: "/settings", label: "概览", icon: Settings },
    { to: "/settings/server", label: "服务器", icon: Server },
    { to: "/settings/backend", label: "后端配置", icon: Database },
    { to: "/settings/prompts", label: "提示词", icon: MessageSquare },
    { to: "/settings/memories", label: "记忆", icon: DatabaseIcon },
    { to: "/settings/masking", label: "Masking", icon: Eye },
  ];

  return (
    <div className="min-h-screen bg-background">
      <div className="container mx-auto py-8">
        <div className="flex gap-8">
          <aside className="w-64 shrink-0">
            <nav className="space-y-1">
              {navItems.map((item) => (
                <Link
                  key={item.to}
                  to={item.to}
                  className="flex items-center gap-3 px-4 py-3 rounded-lg text-sm font-medium text-muted-foreground hover:bg-muted hover:text-foreground transition-colors"
                >
                  <item.icon className="h-4 w-4" />
                  {item.label}
                </Link>
              ))}
            </nav>
          </aside>
          <main className="flex-1">
            <Outlet />
          </main>
        </div>
      </div>
    </div>
  );
}
