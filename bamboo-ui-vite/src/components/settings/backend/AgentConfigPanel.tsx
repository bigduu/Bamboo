
import { useBackendConfigStore } from "@/stores/backendConfigStore";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";

export function AgentConfigPanel() {
  const { config, validationErrors, updateAgent } = useBackendConfigStore();
  const errors = validationErrors.agent;

  return (
    <div className="space-y-6">
      <div className="space-y-2">
        <Label htmlFor="agent-max-rounds">最大轮次</Label>
        <Input
          id="agent-max-rounds"
          type="number"
          min={1}
          value={config.agent.max_rounds}
          onChange={(e) => updateAgent({ max_rounds: parseInt(e.target.value, 10) || 0 })}
          className={errors?.max_rounds ? "border-destructive" : ""}
        />
        {errors?.max_rounds ? (
          <p className="text-sm text-destructive">{errors.max_rounds}</p>
        ) : (
          <p className="text-sm text-muted-foreground">Agent 最大对话轮次</p>
        )}
      </div>

      <div className="space-y-2">
        <Label htmlFor="agent-timeout">超时 (秒)</Label>
        <Input
          id="agent-timeout"
          type="number"
          min={1}
          value={config.agent.timeout_seconds}
          onChange={(e) => updateAgent({ timeout_seconds: parseInt(e.target.value, 10) || 0 })}
          className={errors?.timeout_seconds ? "border-destructive" : ""}
        />
        {errors?.timeout_seconds ? (
          <p className="text-sm text-destructive">{errors.timeout_seconds}</p>
        ) : (
          <p className="text-sm text-muted-foreground">Agent 请求超时时间（秒）</p>
        )}
      </div>

      <div className="space-y-2">
        <Label htmlFor="agent-system-prompt">系统提示词</Label>
        <Textarea
          id="agent-system-prompt"
          value={config.agent.system_prompt}
          onChange={(e) => updateAgent({ system_prompt: e.target.value })}
          rows={6}
          className={errors?.system_prompt ? "border-destructive" : ""}
        />
        {errors?.system_prompt ? (
          <p className="text-sm text-destructive">{errors.system_prompt}</p>
        ) : (
          <p className="text-sm text-muted-foreground">Agent 的系统提示词，用于定义 AI 的行为和角色</p>
        )}
      </div>
    </div>
  );
}
