import React from "react";
import { Link } from "react-router-dom";
import { Settings, Server, Database, MessageSquare, Eye } from "lucide-react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

export default function SettingsOverviewPage() {
  const settingsCards = [
    {
      title: "服务器配置",
      description: "配置 Bamboo 服务器的连接设置",
      icon: Server,
      to: "/settings/server",
    },
    {
      title: "后端配置",
      description: "管理后端服务的所有配置项",
      icon: Database,
      to: "/settings/backend",
    },
    {
      title: "提示词管理",
      description: "管理系统提示词和模板",
      icon: MessageSquare,
      to: "/settings/prompts",
    },
    {
      title: "记忆管理",
      description: "配置长期记忆和上下文",
      icon: Database,
      to: "/settings/memories",
    },
    {
      title: "Masking 配置",
      description: "配置敏感信息脱敏规则",
      icon: Eye,
      to: "/settings/masking",
    },
  ];

  return (
    <div>
      <h1 className="text-3xl font-bold mb-2">设置</h1>
      <p className="text-muted-foreground mb-8">管理 Bamboo 的所有配置项</p>

      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
        {settingsCards.map((card) => (
          <Link key={card.to} to={card.to}>
            <Card className="hover:bg-muted/50 transition-colors cursor-pointer">
              <CardHeader>
                <div className="flex items-center gap-3">
                  <div className="p-2 rounded-lg bg-primary/10">
                    <card.icon className="h-5 w-5 text-primary" />
                  </div>
                  <CardTitle className="text-lg">{card.title}</CardTitle>
                </div>
                <CardDescription>{card.description}</CardDescription>
              </CardHeader>
            </Card>
          </Link>
        ))}
      </div>
    </div>
  );
}
