"use client";

import { useState } from "react";
import { useBackendConfigStore } from "@/stores/backendConfigStore";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { Plus, Trash2 } from "lucide-react";

export function SkillsConfigPanel() {
  const {
    config,
    validationErrors,
    updateSkills,
    addSkillsDirectory,
    updateSkillsDirectory,
    removeSkillsDirectory,
  } = useBackendConfigStore();
  
  const [newDirectory, setNewDirectory] = useState("");
  const errors = validationErrors.skills;

  const handleAddDirectory = () => {
    if (newDirectory.trim()) {
      addSkillsDirectory(newDirectory.trim());
      setNewDirectory("");
    }
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center space-x-2">
        <Switch
          id="skills-enabled"
          checked={config.skills.enabled}
          onCheckedChange={(checked) => updateSkills({ enabled: checked })}
        />
        <Label htmlFor="skills-enabled">启用 Skills</Label>
      </div>

      <div className="flex items-center space-x-2">
        <Switch
          id="skills-auto-reload"
          checked={config.skills.auto_reload}
          onCheckedChange={(checked) => updateSkills({ auto_reload: checked })}
          disabled={!config.skills.enabled}
        />
        <Label htmlFor="skills-auto-reload">自动重载</Label>
      </div>

      <div className="space-y-4">
        <Label>Skills 目录</Label>
        
        <div className="flex items-center gap-2">
          <Input
            placeholder="添加新目录路径"
            value={newDirectory}
            onChange={(e) => setNewDirectory(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && handleAddDirectory()}
            disabled={!config.skills.enabled}
          />
          <Button onClick={handleAddDirectory} size="sm" disabled={!config.skills.enabled}>
            <Plus className="h-4 w-4 mr-1" />
            添加
          </Button>
        </div>

        {errors?.directories?.[0] && (
          <p className="text-sm text-destructive">{errors.directories[0]}</p>
        )}

        <div className="space-y-2">
          {config.skills.directories.map((dir, index) => (
            <div key={index} className="flex items-center gap-2">
              <Input
                value={dir}
                onChange={(e) => updateSkillsDirectory(index, e.target.value)}
                className={errors?.directories?.[index] ? "border-destructive" : ""}
                disabled={!config.skills.enabled}
              />
              <Button
                variant="ghost"
                size="sm"
                onClick={() => removeSkillsDirectory(index)}
                disabled={!config.skills.enabled}
              >
                <Trash2 className="h-4 w-4 text-destructive" />
              </Button>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
