
import React, { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Switch } from "@/components/ui/switch";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { usePromptStore } from "@/stores/promptStore";
import type { SystemPrompt } from "@/types";

export function PromptConfigPanel() {
  const { prompts, loading, fetchPrompts, addPrompt, editPrompt, removePrompt, setDefault } = usePromptStore();
  const [editingPrompt, setEditingPrompt] = useState<SystemPrompt | null>(null);
  const [isCreating, setIsCreating] = useState(false);

  useEffect(() => {
    fetchPrompts();
  }, [fetchPrompts]);

  const handleCreate = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    const form = e.currentTarget;
    const formData = new FormData(form);
    
    await addPrompt({
      name: formData.get("name") as string,
      content: formData.get("content") as string,
      category: (formData.get("category") as string) || "general",
      is_default: formData.get("is_default") === "on",
      is_custom: true,
    });
    
    setIsCreating(false);
    form.reset();
  };

  const handleUpdate = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    if (!editingPrompt) return;
    
    const form = e.currentTarget;
    const formData = new FormData(form);
    
    await editPrompt(editingPrompt.id, {
      name: formData.get("name") as string,
      content: formData.get("content") as string,
      category: formData.get("category") as string,
      is_default: formData.get("is_default") === "on",
    });
    
    setEditingPrompt(null);
  };

  if (loading) {
    return <div>Loading...</div>;
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-xl font-semibold">System Prompts</h2>
          <p className="text-sm text-muted-foreground">Manage your system prompts</p>
        </div>
        <Button onClick={() => setIsCreating(true)}>Create Prompt</Button>
      </div>

      {isCreating && (
        <Card>
          <CardHeader>
            <CardTitle>Create New Prompt</CardTitle>
          </CardHeader>
          <CardContent>
            <form onSubmit={handleCreate} className="space-y-4">
              <div>
                <label className="text-sm font-medium">Name</label>
                <Input name="name" required />
              </div>
              <div>
                <label className="text-sm font-medium">Category</label>
                <Input name="category" defaultValue="general" />
              </div>
              <div>
                <label className="text-sm font-medium">Content</label>
                <Textarea name="content" rows={6} required />
              </div>
              <div className="flex items-center gap-2">
                <Switch name="is_default" />
                <label className="text-sm">Set as default</label>
              </div>
              <div className="flex gap-2">
                <Button type="submit">Create</Button>
                <Button type="button" variant="outline" onClick={() => setIsCreating(false)}>
                  Cancel
                </Button>
              </div>
            </form>
          </CardContent>
        </Card>
      )}

      <div className="grid gap-4">
        {prompts.map((prompt) => (
          <Card key={prompt.id}>
            <CardHeader>
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <CardTitle className="text-lg">{prompt.name}</CardTitle>
                  {prompt.is_default && <Badge>Default</Badge>}
                  {prompt.is_custom && <Badge variant="secondary">Custom</Badge>}
                </div>
                <div className="flex gap-2">
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => setEditingPrompt(prompt)}
                  >
                    Edit
                  </Button>
                  {!prompt.is_default && (
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => setDefault(prompt.id)}
                    >
                      Set Default
                    </Button>
                  )}
                  <Button
                    variant="destructive"
                    size="sm"
                    onClick={() => removePrompt(prompt.id)}
                  >
                    Delete
                  </Button>
                </div>
              </div>
            </CardHeader>
            <CardContent>
              {editingPrompt?.id === prompt.id ? (
                <form onSubmit={handleUpdate} className="space-y-4">
                  <div>
                    <label className="text-sm font-medium">Name</label>
                    <Input name="name" defaultValue={prompt.name} required />
                  </div>
                  <div>
                    <label className="text-sm font-medium">Category</label>
                    <Input name="category" defaultValue={prompt.category} />
                  </div>
                  <div>
                    <label className="text-sm font-medium">Content</label>
                    <Textarea name="content" rows={6} defaultValue={prompt.content} required />
                  </div>
                  <div className="flex items-center gap-2">
                    <Switch name="is_default" defaultChecked={prompt.is_default} />
                    <label className="text-sm">Set as default</label>
                  </div>
                  <div className="flex gap-2">
                    <Button type="submit">Save</Button>
                    <Button
                      type="button"
                      variant="outline"
                      onClick={() => setEditingPrompt(null)}
                    >
                      Cancel
                    </Button>
                  </div>
                </form>
              ) : (
                <div className="space-y-2">
                  <p className="text-sm text-muted-foreground">Category: {prompt.category}</p>
                  <pre className="bg-muted p-4 rounded-md text-sm overflow-auto">
                    {prompt.content}
                  </pre>
                </div>
              )}
            </CardContent>
          </Card>
        ))}
      </div>
    </div>
  );
}
