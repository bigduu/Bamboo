
import React, { useState } from "react";
import { cn } from "@/lib/utils";
import { MessageList } from "./MessageList";
import { InputArea } from "./InputArea";

export interface Message {
  id: string;
  role: "user" | "assistant";
  content: string;
  timestamp: Date;
}

interface ChatContainerProps {
  className?: string;
}

const mockMessages: Message[] = [
  {
    id: "1",
    role: "user",
    content: "你好，请介绍一下你自己",
    timestamp: new Date(Date.now() - 1000 * 60 * 5),
  },
  {
    id: "2",
    role: "assistant",
    content:
      "你好！我是 Bamboo AI，一个智能助手。我可以帮助你回答问题、编写代码、分析数据、创作内容等各种任务。有什么我可以帮助你的吗？",
    timestamp: new Date(Date.now() - 1000 * 60 * 4),
  },
  {
    id: "3",
    role: "user",
    content: "你能用 React 写一个计数器组件吗？",
    timestamp: new Date(Date.now() - 1000 * 60 * 2),
  },
  {
    id: "4",
    role: "assistant",
    content:
      '当然可以！以下是一个简单的 React 计数器组件：\n\n```tsx\nimport { useState } from "react";\n\nfunction Counter() {\n  const [count, setCount] = useState(0);\n  \n  return (\n    <div className="p-4">\n      <h2>计数器: {count}</h2>\n      <button onClick={() => setCount(count + 1)}\u003e\n        增加\n      </button>\n    </div>\n  );\n}\n```\n\n这个组件使用了 React 的 useState hook 来管理计数状态。',
    timestamp: new Date(Date.now() - 1000 * 60 * 1),
  },
];

export function ChatContainer({ className }: ChatContainerProps) {
  const [messages, setMessages] = useState<Message[]>(mockMessages);
  const [isLoading, setIsLoading] = useState(false);

  const handleSendMessage = async (content: string) => {
    const newMessage: Message = {
      id: Date.now().toString(),
      role: "user",
      content,
      timestamp: new Date(),
    };

    setMessages((prev) => [...prev, newMessage]);
    setIsLoading(true);

    // 模拟 AI 回复
    setTimeout(() => {
      const assistantMessage: Message = {
        id: (Date.now() + 1).toString(),
        role: "assistant",
        content: "这是一个模拟的 AI 回复。在实际应用中，这里会调用后端 API 获取真实的 AI 响应。",
        timestamp: new Date(),
      };
      setMessages((prev) => [...prev, assistantMessage]);
      setIsLoading(false);
    }, 1000);
  };

  return (
    <div className={cn("flex h-full flex-col bg-background transition-colors duration-300", className)}>
      <MessageList messages={messages} isLoading={isLoading} />
      <InputArea onSendMessage={handleSendMessage} isLoading={isLoading} />
    </div>
  );
}
