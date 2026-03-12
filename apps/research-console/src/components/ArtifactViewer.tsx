import ReactMarkdown from "react-markdown";

import { humanizeKey } from "../lib/format";
import { useArtifactPreview } from "../lib/store";
import type { ArtifactDescriptor } from "../lib/types";
import { DataTable } from "./DataTable";

export function ArtifactViewer({ artifact }: { artifact: ArtifactDescriptor | null }) {
  const { data: content, error, loading } = useArtifactPreview(artifact);

  if (!artifact) return <div className="empty-box">选择一个 artifact 以查看内容。</div>;
  if (loading) return <div className="empty-box">加载 artifact…</div>;
  if (error) return <div className="empty-box">{error}</div>;
  if (!content) return <div className="empty-box">没有内容。</div>;

  const metaBar = (
    <div className="artifact-meta-bar">
      <span>{artifact.name}</span>
      <span>{humanizeKey(artifact.kind)}</span>
      <span>{artifact.role ?? artifact.scope ?? "artifact"}</span>
      <span>{artifact.format ?? "text"}</span>
      <span>{artifact.size_bytes != null ? `${artifact.size_bytes} bytes` : "size n/a"}</span>
      <span>{artifact.path}</span>
    </div>
  );

  if (content.payload.kind === "text") {
    const isDiff = content.format === "diff" || artifact.path.endsWith(".diff") || artifact.name === "patch.diff";
    const isMarkdown =
      artifact.path.endsWith(".md") ||
      artifact.name === "report.txt" ||
      artifact.name.endsWith("run-evidence.txt") ||
      artifact.name.endsWith("model-comparison.md") ||
      artifact.name.endsWith("verbosity-analysis.md");
    if (isDiff) {
      const lines = content.payload.content.split("\n");
      return (
        <div className="artifact-view-stack">
          {metaBar}
          <div className="diff-view artifact-pre-tall">
            {lines.map((line, index) => {
              const tone =
                line.startsWith("@@") ? "diff-hunk" :
                line.startsWith("+") ? "diff-add" :
                line.startsWith("-") ? "diff-remove" :
                line.startsWith("diff --git") || line.startsWith("index ") || line.startsWith("--- ") || line.startsWith("+++ ")
                  ? "diff-meta"
                  : "diff-context";
              return (
                <div key={`${index}-${line.slice(0, 24)}`} className={`diff-line ${tone}`}>
                  <span className="diff-line-no">{index + 1}</span>
                  <code>{line || " "}</code>
                </div>
              );
            })}
          </div>
        </div>
      );
    }
    return isMarkdown ? (
      <div className="artifact-view-stack">
        {metaBar}
        <div className="markdown-view">
          <ReactMarkdown>{content.payload.content}</ReactMarkdown>
        </div>
      </div>
    ) : (
      <div className="artifact-view-stack">
        {metaBar}
        <pre className="artifact-pre artifact-pre-tall">{content.payload.content}</pre>
      </div>
    );
  }

  if (content.payload.kind === "csv") {
    return (
      <div className="artifact-view-stack">
        {metaBar}
        <DataTable rows={content.payload.rows as Array<Record<string, unknown>>} maxRows={240} compact />
      </div>
    );
  }

  return (
    <div className="artifact-view-stack">
      {metaBar}
      <pre className="artifact-pre artifact-pre-tall">
        {JSON.stringify(content.payload.rows.slice(0, 300), null, 2)}
      </pre>
    </div>
  );
}
