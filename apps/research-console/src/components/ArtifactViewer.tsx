import ReactMarkdown from "react-markdown";

import { useArtifactPreview } from "../lib/store";
import type { ArtifactDescriptor } from "../lib/types";
import { DataTable } from "./DataTable";

export function ArtifactViewer({ artifact }: { artifact: ArtifactDescriptor | null }) {
  const { data: content, error, loading } = useArtifactPreview(artifact);

  if (!artifact) return <div className="empty-box">选择一个 artifact 以查看内容。</div>;
  if (loading) return <div className="empty-box">加载 artifact…</div>;
  if (error) return <div className="empty-box">{error}</div>;
  if (!content) return <div className="empty-box">没有内容。</div>;

  if (content.payload.kind === "text") {
    const isMarkdown = artifact.path.endsWith(".md") || artifact.path.endsWith(".txt");
    return isMarkdown ? (
      <div className="markdown-view">
        <ReactMarkdown>{content.payload.content}</ReactMarkdown>
      </div>
    ) : (
      <pre className="artifact-pre artifact-pre-tall">{content.payload.content}</pre>
    );
  }

  if (content.payload.kind === "csv") {
    return <DataTable rows={content.payload.rows as Array<Record<string, unknown>>} maxRows={200} />;
  }

  return (
    <pre className="artifact-pre artifact-pre-tall">
      {JSON.stringify(content.payload.rows.slice(0, 200), null, 2)}
    </pre>
  );
}
