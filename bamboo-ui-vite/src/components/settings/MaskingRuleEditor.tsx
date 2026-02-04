
import React from "react";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Trash2, Save, X } from "lucide-react";
import type { MaskingRule } from "@/types";

interface MaskingRuleEditorProps {
  rule: MaskingRule;
  onChange: (rule: MaskingRule) => void;
  onDelete?: () => void;
  onCancel?: () => void;
  isNew?: boolean;
}

export function MaskingRuleEditor({
  rule,
  onChange,
  onDelete,
  onCancel,
  isNew = false,
}: MaskingRuleEditorProps) {
  const handleChange = (field: keyof MaskingRule, value: string | boolean) => {
    onChange({ ...rule, [field]: value });
  };

  return (
    <Card className="border-l-4 border-l-primary">
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between">
          <CardTitle className="text-sm font-medium">
            {isNew ? "新建规则" : "编辑规则"}
          </CardTitle>
          <div className="flex items-center gap-2">
            {!isNew && onDelete && (
              <Button
                variant="ghost"
                size="icon"
                onClick={onDelete}
                className="h-8 w-8 text-destructive hover:text-destructive"
              >
                <Trash2 className="h-4 w-4" />
              </Button>
            )}
            {onCancel && (
              <Button
                variant="ghost"
                size="icon"
                onClick={onCancel}
                className="h-8 w-8"
              >
                <X className="h-4 w-4" />
              </Button>
            )}
          </div>
        </div>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="space-y-2">
          <label className="text-sm font-medium">规则名称</label>
          <Input
            value={rule.name}
            onChange={(e) => handleChange("name", e.target.value)}
            placeholder="例如：手机号脱敏"
          />
        </div>

        <div className="space-y-2">
          <label className="text-sm font-medium">描述（可选）</label>
          <Input
            value={rule.description || ""}
            onChange={(e) => handleChange("description", e.target.value)}
            placeholder="规则描述"
          />
        </div>

        <div className="flex items-center justify-between">
          <label className="text-sm font-medium">使用正则表达式</label>
          <Switch
            checked={rule.isRegex}
            onCheckedChange={(checked) => handleChange("isRegex", checked)}
          />
        </div>

        <div className="space-y-2">
          <label className="text-sm font-medium">
            {rule.isRegex ? "正则表达式" : "关键字"}
          </label>
          <Input
            value={rule.pattern}
            onChange={(e) => handleChange("pattern", e.target.value)}
            placeholder={rule.isRegex ? "例如：\\d{11}" : "例如：手机号"}
          />
        </div>

        <div className="space-y-2">
          <label className="text-sm font-medium">替换文本</label>
          <Input
            value={rule.replacement}
            onChange={(e) => handleChange("replacement", e.target.value)}
            placeholder="例如：***"
          />
        </div>

        <div className="flex items-center justify-between pt-2">
          <label className="text-sm font-medium">启用规则</label>
          <Switch
            checked={rule.enabled}
            onCheckedChange={(checked) => handleChange("enabled", checked)}
          />
        </div>
      </CardContent>
    </Card>
  );
}
