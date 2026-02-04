
import React from "react";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Edit2, Plus, Trash2 } from "lucide-react";
import type { MaskingRule } from "@/types";

interface MaskingRuleListProps {
  rules: MaskingRule[];
  onToggle: (id: string) => void;
  onEdit: (rule: MaskingRule) => void;
  onDelete: (id: string) => void;
  onAdd: () => void;
}

export function MaskingRuleList({
  rules,
  onToggle,
  onEdit,
  onDelete,
  onAdd,
}: MaskingRuleListProps) {
  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h3 className="text-lg font-semibold">规则列表 ({rules.length})</h3>
        <Button onClick={onAdd} size="sm">
          <Plus className="mr-2 h-4 w-4" />
          添加规则
        </Button>
      </div>

      {rules.length === 0 ? (
        <Card>
          <CardContent className="flex flex-col items-center justify-center py-8 text-center">
            <p className="text-muted-foreground">暂无规则</p>
            <p className="text-sm text-muted-foreground mt-1">点击上方按钮添加第一条规则</p>
          </CardContent>
        </Card>
      ) : (
        <div className="space-y-3">
          {rules.map((rule) => (
            <Card
              key={rule.id}
              className={`transition-opacity ${
                rule.enabled ? "" : "opacity-60"
              }`}
            >
              <CardHeader className="pb-3">
                <div className="flex items-start justify-between">
                  <div className="flex-1 min-w-0">
                    <CardTitle className="text-sm font-medium truncate">
                      {rule.name}
                    </CardTitle>
                    {rule.description && (
                      <p className="text-xs text-muted-foreground mt-1 truncate">
                        {rule.description}
                      </p>
                    )}
                  </div>
                  <div className="flex items-center gap-2 ml-4">
                    <Switch
                      checked={rule.enabled}
                      onCheckedChange={() => onToggle(rule.id)}
                    />
                    <Button
                      variant="ghost"
                      size="icon"
                      onClick={() => onEdit(rule)}
                      className="h-8 w-8"
                    >
                      <Edit2 className="h-4 w-4" />
                    </Button>
                    <Button
                      variant="ghost"
                      size="icon"
                      onClick={() => onDelete(rule.id)}
                      className="h-8 w-8 text-destructive hover:text-destructive"
                    >
                      <Trash2 className="h-4 w-4" />
                    </Button>
                  </div>
                </div>
              </CardHeader>
              <CardContent className="pt-0">
                <div className="grid grid-cols-2 gap-4 text-sm">
                  <div>
                    <span className="text-muted-foreground">
                      {rule.isRegex ? "正则" : "关键字"}: {" "}
                    </span>
                    <code className="bg-muted px-1 py-0.5 rounded text-xs">
                      {rule.pattern}
                    </code>
                  </div>
                  <div>
                    <span className="text-muted-foreground">替换为: {" "}</span>
                    <span>{rule.replacement}</span>
                  </div>
                </div>
              </CardContent>
            </Card>
          ))}
        </div>
      )}
    </div>
  );
}
