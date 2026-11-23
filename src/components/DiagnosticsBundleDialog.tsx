import { Icon } from "./Icon";
import { Dialog } from "./Dialog";

/** Visible, explicit diagnostics preview. Nothing is uploaded or copied automatically. */
export function DiagnosticsBundleDialog({ bundle, onClose }: { bundle: string; onClose: () => void }) {
  function copy() {
    if (!navigator.clipboard) return;
    void navigator.clipboard.writeText(bundle).catch(() => undefined);
  }

  function download() {
    const blob = new Blob([bundle], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = "swiftfetch-diagnostics-redacted.json";
    link.click();
    URL.revokeObjectURL(url);
  }

  return (
    <Dialog title="Shareable diagnostic bundle" description="Redacted local facts only. Nothing has been uploaded." onClose={onClose} wide>
      <div className="diagnostics-dialog-body">
        <p className="field-hint">Review the preview before copying or saving it. URLs, paths, headers and credentials are removed.</p>
        <pre className="diagnostic-json" tabIndex={0} aria-label="Shareable diagnostic JSON">{bundle}</pre>
        <footer className="dialog-footer">
          <button type="button" className="button" onClick={onClose}>Close</button>
          <button type="button" className="button" onClick={copy}><Icon name="copy" size={16} />Copy preview</button>
          <button type="button" className="button button-primary" onClick={download}><Icon name="download" size={16} />Save bundle</button>
        </footer>
      </div>
    </Dialog>
  );
}
