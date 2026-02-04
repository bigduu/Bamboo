"use client";

import React, { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { useMemoryStore } from "@/stores/memoryStore";
import { useSessionStore } from "@/stores/sessionStore";

export function MemoryConfigPanel() {
  const { memories, sessionMemory, loading, fetchMemories, fetchSessionMemory } = useMemoryStore();
  const { sessions } = useSessionStore();
  const [selectedSessionId, setSelectedSessionId] = useState<string>("");

  useEffect(() => {
    fetchMemories();
  }, [fetchMemories]);

  useEffect(() => {
    if (selectedSessionId) {
      fetchSessionMemory(selectedSessionId);
    }
  }, [selectedSessionId, fetchSessionMemory]);

  if (loading) {
    return <div>Loading...</div>;
  }

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-xl font-semibold">Memory Management</h2>
        <p className="text-sm text-muted-foreground">View and manage conversation memories</p>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Session Memory</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div>
            <label className="text-sm font-medium">Select Session</label>
            <select
              className="w-full p-2 border rounded-md"
              value={selectedSessionId}
              onChange={(e) => setSelectedSessionId(e.target.value)}
            >
              <option value="">Select a session...</option>
              {sessions.map((session) => (
                <option key={session.id} value={session.id}>
                  {session.title}
                </option>
              ))}
            </select>
          </div>

          {sessionMemory && sessionMemory.memories.length > 0 ? (
            <div className="space-y-2">
              <p className="text-sm text-muted-foreground">
                Last updated: {new Date(sessionMemory.updated_at).toLocaleString()}
              </p>
              <div className="space-y-2">
                {sessionMemory.memories.map((memory) => (
                  <Card key={memory.id}>
                    <CardContent className="p-4">
                      <p className="text-sm">{memory.content}</p>
                      <p className="text-xs text-muted-foreground mt-2">
                        {new Date(memory.created_at).toLocaleString()}
                      </p>
                    </CardContent>
                  </Card>
                ))}
              </div>
            </div>
          ) : selectedSessionId ? (
            <p className="text-muted-foreground">No memories for this session</p>
          ) : null}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>All Memories</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="space-y-2">
            {memories.slice(0, 20).map((memory) => (
              <Card key={memory.id}>
                <CardContent className="p-4">
                  <p className="text-sm">{memory.content}</p>
                  <div className="flex items-center gap-2 mt-2">
                    <Badge variant="secondary">{memory.session_id.slice(0, 8)}...</Badge>
                    <span className="text-xs text-muted-foreground">
                      {new Date(memory.created_at).toLocaleString()}
                    </span>
                  </div>
                </CardContent>
              </Card>
            ))}
            {memories.length === 0 && (
              <p className="text-muted-foreground">No memories yet</p>
            )}
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
