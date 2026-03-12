import { Navigate, Route, Routes } from "react-router-dom";

import { Shell } from "./components/Shell";
import { CampaignsPage } from "./pages/CampaignsPage";
import { LivePage } from "./pages/LivePage";
import { RunsPage } from "./pages/RunsPage";
import { ComparePage } from "./pages/ComparePage";
import { ArtifactsPage } from "./pages/ArtifactsPage";
import { ResearchPage } from "./pages/ResearchPage";
import { RunDetailPage } from "./pages/RunDetailPage";

export function App() {
  return (
    <Shell>
      <Routes>
        <Route path="/" element={<Navigate to="/campaigns" replace />} />
        <Route path="/campaigns" element={<CampaignsPage />} />
        <Route path="/live" element={<LivePage />} />
        <Route path="/runs" element={<RunsPage />} />
        <Route path="/runs/:runId" element={<RunDetailPage />} />
        <Route path="/compare" element={<ComparePage />} />
        <Route path="/artifacts" element={<ArtifactsPage />} />
        <Route path="/research" element={<ResearchPage />} />
      </Routes>
    </Shell>
  );
}
