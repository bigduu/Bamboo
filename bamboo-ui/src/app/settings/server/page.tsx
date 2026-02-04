import { ServerConfigPanel } from "@/components/settings/ServerConfigPanel";

export default function ServerSettingsPage() {
  return (
    <div className="container max-w-4xl py-8">
      <h1 className="text-3xl font-bold mb-8">Server Settings</h1>
      <ServerConfigPanel />
    </div>
  );
}
