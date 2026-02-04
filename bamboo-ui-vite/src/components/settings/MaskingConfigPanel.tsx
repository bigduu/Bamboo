
import React, { useState, useEffect, useCallback } from "react";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Loader2, Play, Save, Shield } from "lucide-react";
import { MaskingRuleList } from "./MaskingRuleList";
import { MaskingRuleEditor } from "./MaskingRuleEditor";
import { getMaskingConfig, saveMaskingConfig, testMasking } from "@/lib/api";
import type { MaskingConfig, MaskingRule, MaskingTestResponse } from "@/types";

const createId = () => {
  if (typeof crypto !== "undefined" && "randomUUID" in crypto) {
    return crypto.randomUUID();
  }
  return `rule_${Date.now()}_${Math.random().toString(36).slice(2, 10)}`;
};

const DEFAULT_RULE: MaskingRule = {
  id: "",
  name: "",
  pattern: "",
  replacement: "***",
  enabled: true,
  description: "",
  isRegex: false,
};

export function MaskingConfigPanel() {
  const [config, setConfig] = useState<MaskingConfig>({
    enabled: true,
    rules: [],
  });
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [editingRule, setEditingRule] = useState<MaskingRule | null>(null);
  const [isAdding, setIsAdding] = useState(false);
  
  // Test state
  const [testText, setTestText] = useState("");
  const [testResult, setTestResult] = useState<MaskingTestResponse | null>(null);
  const [testing, setTesting] = useState(false);

  // Load config on mount
  useEffect(() => {
    loadConfig();
  }, []);

  const loadConfig = async () => {
    try {
      setLoading(true);
      const data = await getMaskingConfig();
      setConfig(data);
    } catch (error) {
      console.error("Failed to load masking config:", error);
      // Use default config if API fails
    } finally {
      setLoading(false);
    }
  };

  const handleSave = async () => {
    try {
      setSaving(true);
      const saved = await saveMaskingConfig(config);
      setConfig(saved);
      alert("配置已保存");
    } catch (error) {
      console.error("Failed to save masking config:", error);
      alert("保存失败");
    } finally {
      setSaving(false);
    }
  };

  const handleToggleRule = (id: string) => {
    setConfig((prev) => ({
      ...prev,
      rules: prev.rules.map((rule) =>
        rule.id === id ? { ...rule, enabled: !rule.enabled } : rule
      ),
    }));
  };

  const handleDeleteRule = (id: string) => {
    if (confirm("确定要删除这条规则吗？")) {
      setConfig((prev) => ({
        ...prev,
        rules: prev.rules.filter((rule) => rule.id !== id),
      }));
    }
  };

  const handleEditRule = (rule: MaskingRule) => {
    setEditingRule({ ...rule });
    setIsAdding(false);
  };

  const handleAddRule = () => {
    setEditingRule({ ...DEFAULT_RULE, id: createId() });
    setIsAdding(true);
  };

  const handleSaveRule = (rule: MaskingRule) => {
    if (isAdding) {
      setConfig((prev) => ({
        ...prev,
        rules: [...prev.rules, rule],
      }));
    } else {
      setConfig((prev) => ({
        ...prev,
        rules: prev.rules.map((r) => (r.id === rule.id ? rule : r)),
      }));
    }
    setEditingRule(null);
    setIsAdding(false);
  };

  const handleCancelEdit = () => {
    setEditingRule(null);
    setIsAdding(false);
  };

  const handleTest = async () => {
    if (!testText.trim()) return;
    
    try {
      setTesting(true);
      const result = await testMasking({
        text: testText,
        config,
      });
      setTestResult(result);
    } catch (error) {
      console.error("Test failed:", error);
      alert("测试失败");
    } finally {
      setTesting(false);
    }
  };

  // Auto-test when text or config changes (with debounce)
  useEffect(() => {
    if (!testText.trim()) {
      setTestResult(null);
      return;
    }

    const timer = setTimeout(() => {
      handleTest();
    }, 500);

    return () => clearTimeout(timer);
  }, [testText, config]);

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <Loader2 className="h-8 w-8 animate-spin" />
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <Shield className="h-6 w-6 text-primary" />
          <div>
            <h2 className="text-xl font-semibold">Masking 配置</h2>
            <p className="text-sm text-muted-foreground">配置敏感信息脱敏规则</p>
          </div>
        </div>
        <div className="flex items-center gap-4">
          <div className="flex items-center gap-2">
            <Switch
              checked={config.enabled}
              onCheckedChange={(checked) =>
                setConfig((prev) => ({ ...prev, enabled: checked }))
              }
            />
            <span className="text-sm">{config.enabled ? "已启用" : "已禁用"}</span>
          </div>
          <Button onClick={handleSave} disabled={saving}>
            {saving ? (
              <>
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                保存中
              </>
            ) : (
              <>
                <Save className="mr-2 h-4 w-4" />
                保存配置
              </>
            )}
          </Button>
        </div>
      </div>

      <div className="grid gap-6 lg:grid-cols-2">
        {/* Rules Section */}
        <div className="space-y-4">
          {editingRule ? (
            <>
              <MaskingRuleEditor
                rule={editingRule}
                onChange={setEditingRule}
                onDelete={isAdding ? undefined : () => {
                  handleDeleteRule(editingRule.id);
                  setEditingRule(null);
                }}
                onCancel={handleCancelEdit}
                isNew={isAdding}
              />
              <div className="flex gap-2">
                <Button 
                  onClick={() => handleSaveRule(editingRule)}
                  className="flex-1"
                >
                  <Save className="mr-2 h-4 w-4" />
                  保存规则
                </Button>
                <Button 
                  variant="outline" 
                  onClick={handleCancelEdit}
                >
                  取消
                </Button>
              </div>
            </>
          ) : (
            <MaskingRuleList
              rules={config.rules}
              onToggle={handleToggleRule}
              onEdit={handleEditRule}
              onDelete={handleDeleteRule}
              onAdd={handleAddRule}
            />
          )}
        </div>

        {/* Test Section */}
        <div className="space-y-4">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Play className="h-5 w-5" />
                实时预览
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="space-y-2">
                <label className="text-sm font-medium">测试文本</label>
                <Textarea
                  value={testText}
                  onChange={(e) => setTestText(e.target.value)}
                  placeholder="输入要测试的文本..."
                  rows={4}
                />
              </div>

              {testing && (
                <div className="flex items-center gap-2 text-sm text-muted-foreground">
                  <Loader2 className="h-4 w-4 animate-spin" />
                  测试中...
                </div>
              )}

              {testResult && (
                <div className="space-y-4">
                  <div className="space-y-2">
                    <label className="text-sm font-medium">脱敏结果</label>
                    <div className="p-3 bg-muted rounded-md font-mono text-sm whitespace-pre-wrap">
                      {testResult.masked}
                    </div>
                  </div>

                  {testResult.matches.length > 0 && (
                    <div className="space-y-2">
                      <label className="text-sm font-medium">
                        匹配项 ({testResult.matches.length})
                      </label>
                      <div className="space-y-2">
                        {testResult.matches.map((match, index) => (
                          <div
                            key={index}
                            className="flex items-center gap-2 text-sm p-2 bg-yellow-50 dark:bg-yellow-950 rounded"
                          >
                            <span className="font-medium">{match.ruleName}:</span>
                            <code className="bg-yellow-100 dark:bg-yellow-900 px-1 rounded">
                              {match.matched}
                            </code>
                            <span className="text-muted-foreground">
                              → {match.position.start}-{match.position.end}
                            </span>
                          </div>
                        ))}
                      </div>
                    </div>
                  )}
                </div>
              )}
            </CardContent>
          </Card>
        </div>
      </div>
    </div>
  );
}
