import { Routes, Route } from "react-router-dom";
import { ThemeProvider } from "@/components/ThemeProvider";
import Layout from "@/components/layout/Layout";
import HomePage from "@/pages/HomePage";
import StatsPage from "@/pages/StatsPage";
import SettingsLayout from "@/pages/settings/SettingsLayout";
import SettingsOverviewPage from "@/pages/settings/SettingsOverviewPage";
import ServerSettingsPage from "@/pages/settings/ServerSettingsPage";
import BackendSettingsPage from "@/pages/settings/BackendSettingsPage";
import PromptsSettingsPage from "@/pages/settings/PromptsSettingsPage";
import MemoriesSettingsPage from "@/pages/settings/MemoriesSettingsPage";
import MaskingSettingsPage from "@/pages/settings/MaskingSettingsPage";

function App() {
  return (
    <ThemeProvider defaultTheme="system" enableSystem>
      <Layout>
        <Routes>
          <Route path="/" element={<HomePage />} />
          <Route path="/stats" element={<StatsPage />} />
          <Route path="/settings" element={<SettingsLayout />}>
            <Route index element={<SettingsOverviewPage />} />
            <Route path="server" element={<ServerSettingsPage />} />
            <Route path="backend" element={<BackendSettingsPage />} />
            <Route path="prompts" element={<PromptsSettingsPage />} />
            <Route path="memories" element={<MemoriesSettingsPage />} />
            <Route path="masking" element={<MaskingSettingsPage />} />
          </Route>
        </Routes>
      </Layout>
    </ThemeProvider>
  );
}

export default App;
