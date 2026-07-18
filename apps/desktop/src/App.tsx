import { useEffect, useMemo, useState } from "react";
import { api } from "./api";
import type {
  CaseSummary,
  Category,
  ExportFormat,
  StorageInfo,
  SupportCase,
} from "./types";

type View =
  "home" | "new" | "findings" | "privacy" | "export" | "history" | "settings";

const views: Array<{ id: View; label: string }> = [
  { id: "home", label: "Home" },
  { id: "new", label: "New diagnostic" },
  { id: "findings", label: "Findings" },
  { id: "privacy", label: "Privacy review" },
  { id: "export", label: "Export" },
  { id: "history", label: "Case history" },
  { id: "settings", label: "Settings" },
];

const categories: Array<{ id: Category; title: string; description: string }> =
  [
    {
      id: "general",
      title: "General diagnostic scan",
      description:
        "A balanced overview of system, storage, services, updates, network, and graphics.",
    },
    {
      id: "performance",
      title: "Slow computer / performance",
      description:
        "Focus on storage pressure, failed services, and relevant system context.",
    },
    {
      id: "network",
      title: "Network or internet problems",
      description:
        "Inspect adapter state, active connections, and DNS configuration.",
    },
    {
      id: "updates",
      title: "Failed operating-system updates",
      description:
        "Inspect the package or update state and restart indicators.",
    },
    {
      id: "storage",
      title: "Storage or disk-space problems",
      description: "Review system-disk capacity and usage thresholds.",
    },
    {
      id: "drivers_graphics",
      title: "Drivers and graphics",
      description: "Inspect GPU, driver, device, and loaded-module evidence.",
    },
    {
      id: "services",
      title: "Failed services or background processes",
      description: "Find failed services and bounded recent error evidence.",
    },
  ];

const scanStages = [
  "System profile",
  "Category collectors",
  "Deterministic rules",
  "Privacy transformation",
  "Local case write",
];

const humanize = (value: string) =>
  value.replaceAll("_", " ").replace(/\b\w/g, (letter) => letter.toUpperCase());
const formatDate = (value: string) =>
  new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(value));

function errorMessage(error: unknown): string {
  if (
    typeof error === "object" &&
    error &&
    "message" in error &&
    typeof error.message === "string"
  )
    return error.message;
  if (typeof error === "string") return error;
  return "The operation could not be completed. Review the local logs and try again.";
}

export default function App() {
  const [view, setView] = useState<View>("home");
  const [category, setCategory] = useState<Category>("general");
  const [redactSsids, setRedactSsids] = useState(true);
  const [activeCase, setActiveCase] = useState<SupportCase | null>(null);
  const [history, setHistory] = useState<CaseSummary[]>([]);
  const [storage, setStorage] = useState<StorageInfo | null>(null);
  const [loading, setLoading] = useState(false);
  const [completedStages, setCompletedStages] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  const refreshHistory = async () => {
    try {
      setHistory(await api.listCases());
    } catch (reason) {
      setError(errorMessage(reason));
    }
  };

  useEffect(() => {
    void api
      .listCases()
      .then(setHistory)
      .catch((reason) => setError(errorMessage(reason)));
  }, []);
  useEffect(() => {
    if (view === "settings" && !storage) {
      void api
        .getStorageInfo()
        .then(setStorage)
        .catch((reason) => setError(errorMessage(reason)));
    }
  }, [storage, view]);

  const navigate = (next: View) => {
    if (!activeCase && ["findings", "privacy", "export"].includes(next)) {
      setNotice("Run a diagnostic or reopen a local case first.");
      setView("new");
      return;
    }
    setNotice(null);
    setError(null);
    setView(next);
  };

  const startScan = async () => {
    setLoading(true);
    setError(null);
    setNotice(null);
    setCompletedStages([]);
    let index = 0;
    const timer = window.setInterval(() => {
      setCompletedStages((current) =>
        current.includes(scanStages[index])
          ? current
          : [...current, scanStages[index]],
      );
      index = Math.min(index + 1, scanStages.length - 1);
    }, 700);
    try {
      const supportCase = await api.runScan(category, redactSsids);
      setActiveCase(supportCase);
      setCompletedStages(scanStages);
      await refreshHistory();
      setView("findings");
    } catch (reason) {
      setError(errorMessage(reason));
    } finally {
      window.clearInterval(timer);
      setLoading(false);
    }
  };

  const openCase = async (caseId: string) => {
    setLoading(true);
    setError(null);
    try {
      setActiveCase(await api.loadCase(caseId));
      setView("findings");
    } catch (reason) {
      setError(errorMessage(reason));
    } finally {
      setLoading(false);
    }
  };

  const deleteCase = async (caseId: string) => {
    try {
      await api.deleteCase(caseId);
      if (activeCase?.caseId === caseId) setActiveCase(null);
      await refreshHistory();
      setNotice(
        "Case moved to the app recovery directory. Existing exports were not changed.",
      );
    } catch (reason) {
      setError(errorMessage(reason));
    }
  };

  const exportCase = async (format: ExportFormat) => {
    if (!activeCase) return;
    setLoading(true);
    setError(null);
    try {
      const result = await api.exportCase(activeCase.caseId, format);
      setNotice(
        `${format.toUpperCase()} export created at ${result.destination}`,
      );
      setActiveCase(await api.loadCase(activeCase.caseId));
    } catch (reason) {
      setError(errorMessage(reason));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="app-shell">
      <aside className="side-rail" aria-label="Primary navigation">
        <div className="brand">
          <span className="brand-mark" aria-hidden="true">
            L
          </span>
          <span>
            <strong>SupportKit</strong>
            <small>Local diagnostics</small>
          </span>
        </div>
        <nav className="desktop-nav">
          {views.map((item) => (
            <button
              key={item.id}
              className="nav-item"
              aria-current={view === item.id ? "page" : undefined}
              onClick={() => navigate(item.id)}
            >
              <span className="nav-indicator" aria-hidden="true" />
              {item.label}
            </button>
          ))}
        </nav>
        <div className="rail-foot">
          <span className="privacy-dot" aria-hidden="true" />
          No telemetry
        </div>
      </aside>

      <header className="mobile-header">
        <div className="brand">
          <span className="brand-mark" aria-hidden="true">
            L
          </span>
          <strong>SupportKit</strong>
        </div>
        <label className="sr-only" htmlFor="mobile-view">
          Current page
        </label>
        <select
          id="mobile-view"
          value={view}
          onChange={(event) => navigate(event.target.value as View)}
        >
          {views.map((item) => (
            <option key={item.id} value={item.id}>
              {item.label}
            </option>
          ))}
        </select>
      </header>

      <main className="workspace" aria-busy={loading}>
        {api.demoMode && (
          <div className="demo-banner" role="status">
            <strong>Browser demo.</strong> Synthetic evidence only; no host
            diagnostics or files.
          </div>
        )}
        {error && (
          <div className="alert alert-error" role="alert">
            <strong>Could not complete the action.</strong>
            <span>{error}</span>
            <button onClick={() => setError(null)}>Dismiss</button>
          </div>
        )}
        {notice && (
          <div className="alert alert-info" role="status">
            <span>{notice}</span>
            <button onClick={() => setNotice(null)}>Dismiss</button>
          </div>
        )}

        {view === "home" && (
          <Home
            history={history}
            onStart={() => navigate("new")}
            onHistory={() => navigate("history")}
          />
        )}
        {view === "new" && (
          <NewDiagnostic
            category={category}
            onCategory={setCategory}
            redactSsids={redactSsids}
            onRedactSsids={setRedactSsids}
            loading={loading}
            completedStages={completedStages}
            onStart={() => void startScan()}
          />
        )}
        {view === "findings" && activeCase && (
          <Findings
            supportCase={activeCase}
            onPrivacy={() => navigate("privacy")}
          />
        )}
        {view === "privacy" && activeCase && (
          <PrivacyReview
            supportCase={activeCase}
            onExport={() => navigate("export")}
          />
        )}
        {view === "export" && activeCase && (
          <ExportView
            supportCase={activeCase}
            loading={loading}
            onExport={(format) => void exportCase(format)}
          />
        )}
        {view === "history" && (
          <History
            cases={history}
            loading={loading}
            onOpen={(id) => void openCase(id)}
            onDelete={(id) => void deleteCase(id)}
          />
        )}
        {view === "settings" && (
          <Settings
            storage={storage}
            redactSsids={redactSsids}
            onRedactSsids={setRedactSsids}
          />
        )}
      </main>
    </div>
  );
}

function PageHeader({
  title,
  description,
  meta,
}: {
  title: string;
  description: string;
  meta?: string;
}) {
  return (
    <header className="page-header">
      <div>
        <h1>{title}</h1>
        <p>{description}</p>
      </div>
      {meta && <code>{meta}</code>}
    </header>
  );
}

function Home({
  history,
  onStart,
  onHistory,
}: {
  history: CaseSummary[];
  onStart: () => void;
  onHistory: () => void;
}) {
  return (
    <div className="page home-page">
      <section className="home-lead">
        <div>
          <p className="plain-label">Read-only · local-first</p>
          <h1>Turn system evidence into a support case.</h1>
          <p>
            Run a focused diagnostic, understand what was found, review privacy
            changes, then share only what you approve.
          </p>
          <button className="button primary large" onClick={onStart}>
            Start a diagnostic
          </button>
        </div>
        <dl className="trust-panel">
          <div>
            <dt>Collection</dt>
            <dd>Bounded and read-only</dd>
          </div>
          <div>
            <dt>Diagnosis</dt>
            <dd>Deterministic rules</dd>
          </div>
          <div>
            <dt>Sharing</dt>
            <dd>Manual export only</dd>
          </div>
          <div>
            <dt>Accounts</dt>
            <dd>Not required</dd>
          </div>
        </dl>
      </section>
      <section className="home-secondary">
        <div>
          <h2>Recent local cases</h2>
          <p>
            {history.length
              ? `${history.length} case${history.length === 1 ? "" : "s"} stored on this device.`
              : "No cases yet. Your first scan will appear here."}
          </p>
        </div>
        <button className="button secondary" onClick={onHistory}>
          Open case history
        </button>
      </section>
    </div>
  );
}

function NewDiagnostic({
  category,
  onCategory,
  redactSsids,
  onRedactSsids,
  loading,
  completedStages,
  onStart,
}: {
  category: Category;
  onCategory: (value: Category) => void;
  redactSsids: boolean;
  onRedactSsids: (value: boolean) => void;
  loading: boolean;
  completedStages: string[];
  onStart: () => void;
}) {
  return (
    <div className="page">
      <PageHeader
        title="New diagnostic"
        description="Choose the problem that best matches what you are seeing. The plan stays read-only."
      />
      {loading ? (
        <ScanProgress completed={completedStages} />
      ) : (
        <>
          <fieldset className="category-grid">
            <legend>Problem category</legend>
            {categories.map((item) => (
              <label
                key={item.id}
                htmlFor={`category-${item.id}`}
                aria-label={item.title}
                className={`category-option ${category === item.id ? "selected" : ""}`}
              >
                <input
                  id={`category-${item.id}`}
                  type="radio"
                  name="category"
                  value={item.id}
                  checked={category === item.id}
                  onChange={() => onCategory(item.id)}
                />
                <span>
                  <strong>{item.title}</strong>
                  <small>{item.description}</small>
                </span>
              </label>
            ))}
          </fieldset>
          <section className="scan-consent">
            <div>
              <h2>Before the scan</h2>
              <ul>
                <li>
                  No settings, services, packages, or files will be changed.
                </li>
                <li>
                  Unavailable collectors are skipped without stopping the scan.
                </li>
                <li>Raw evidence is transformed before the case is stored.</li>
              </ul>
            </div>
            <label
              className="toggle-row"
              htmlFor="scan-redact-ssids"
              aria-label="Redact Wi-Fi names"
            >
              <input
                id="scan-redact-ssids"
                type="checkbox"
                checked={redactSsids}
                onChange={(event) => onRedactSsids(event.target.checked)}
              />
              <span>
                <strong>Redact Wi-Fi names</strong>
                <small>
                  Recommended for support cases shared outside your
                  organisation.
                </small>
              </span>
            </label>
          </section>
          <div className="action-row">
            <button className="button primary large" onClick={onStart}>
              Run read-only scan
            </button>
            <span>
              Usually completes in under a minute. Slow collectors time out.
            </span>
          </div>
        </>
      )}
    </div>
  );
}

function ScanProgress({ completed }: { completed: string[] }) {
  const currentIndex = Math.min(completed.length, scanStages.length - 1);
  return (
    <section className="scan-progress" aria-live="polite">
      <div className="scan-heading">
        <div className="activity-mark" aria-hidden="true" />
        <div>
          <h2>Collecting diagnostic evidence</h2>
          <p>Current stage: {scanStages[currentIndex]}</p>
        </div>
      </div>
      <ol>
        {scanStages.map((stage, index) => (
          <li
            key={stage}
            className={
              completed.includes(stage)
                ? "done"
                : index === currentIndex
                  ? "current"
                  : ""
            }
          >
            <span aria-hidden="true">
              {completed.includes(stage) ? "✓" : index + 1}
            </span>
            {stage}
          </li>
        ))}
      </ol>
      <div className="scan-note">
        <strong>Cancellation state:</strong> not requested. Process cancellation
        is not available in this milestone; each collector has a fixed timeout.
      </div>
    </section>
  );
}

function Findings({
  supportCase,
  onPrivacy,
}: {
  supportCase: SupportCase;
  onPrivacy: () => void;
}) {
  const counts = useMemo(
    () =>
      supportCase.findings.reduce<Record<string, number>>((result, finding) => {
        result[finding.severity] = (result[finding.severity] ?? 0) + 1;
        return result;
      }, {}),
    [supportCase.findings],
  );
  return (
    <div className="page">
      <PageHeader
        title="Findings"
        description="Rules explain the evidence without changing the system."
        meta={`Case ${supportCase.caseId.slice(0, 8)}`}
      />
      <section className="finding-summary">
        <div>
          <strong>{supportCase.findings.length}</strong>
          <span>Total findings</span>
        </div>
        {(["critical", "warning", "notice", "info"] as const).map(
          (severity) => (
            <div key={severity}>
              <strong>{counts[severity] ?? 0}</strong>
              <span>{humanize(severity)}</span>
            </div>
          ),
        )}
      </section>
      {supportCase.scanStatus === "partial" && (
        <div className="incomplete-callout">
          <strong>Partial scan.</strong>{" "}
          {
            supportCase.collectionManifest.filter(
              (item) => item.status !== "complete",
            ).length
          }{" "}
          collector(s) were skipped or failed. Their status is preserved in this
          case.
        </div>
      )}
      <div className="finding-list">
        {supportCase.findings.length === 0 ? (
          <EmptyState
            title="No findings matched"
            text="The completed rules did not identify a problem. Review incomplete collectors before concluding the system is healthy."
          />
        ) : (
          supportCase.findings.map((finding) => (
            <article className="finding-card" key={finding.ruleId}>
              <div className="finding-title">
                <span className={`severity ${finding.severity}`}>
                  {humanize(finding.severity)}
                </span>
                <span className="confidence">
                  {humanize(finding.confidence)}
                </span>
              </div>
              <h2>{finding.title}</h2>
              <p>{finding.explanation}</p>
              <dl>
                <div>
                  <dt>Likely impact</dt>
                  <dd>{finding.likelyImpact}</dd>
                </div>
              </dl>
              <h3>Recommended next steps</h3>
              <ol>
                {finding.recommendedNextSteps.map((step) => (
                  <li key={step}>{step}</li>
                ))}
              </ol>
              <details>
                <summary>Evidence and rule details</summary>
                <p>Evidence: {finding.evidenceRefs.join(", ")}</p>
                <p>
                  Rule: <code>{finding.ruleId}</code> · version{" "}
                  {finding.ruleVersion}
                </p>
                {finding.documentationLinks.map((link) => (
                  <a key={link} href={link} target="_blank" rel="noreferrer">
                    Open relevant documentation
                  </a>
                ))}
              </details>
            </article>
          ))
        )}
      </div>
      <div className="action-row">
        <button className="button primary" onClick={onPrivacy}>
          Review privacy changes
        </button>
        <span>Required before export.</span>
      </div>
    </div>
  );
}

function PrivacyReview({
  supportCase,
  onExport,
}: {
  supportCase: SupportCase;
  onExport: () => void;
}) {
  const count = Object.values(supportCase.redactionManifest.categories).reduce(
    (sum, value) => sum + value,
    0,
  );
  return (
    <div className="page">
      <PageHeader
        title="Privacy review"
        description="See what was transformed and inspect the exact evidence that will be exported."
        meta={`${count} transformations`}
      />
      <div className="privacy-layout">
        <section>
          <h2>Redaction summary</h2>
          <div className="manifest-list">
            {Object.entries(supportCase.redactionManifest.categories).map(
              ([name, value]) => (
                <div key={name}>
                  <span>{humanize(name)}</span>
                  <strong>{value}</strong>
                </div>
              ),
            )}
          </div>
          <h3>Transformed fields</h3>
          <ul className="path-list">
            {supportCase.redactionManifest.transformedPaths.map((path) => (
              <li key={path}>
                <code>{path}</code>
              </li>
            ))}
          </ul>
          {supportCase.redactionManifest.warnings.map((warning) => (
            <p className="privacy-warning" key={warning}>
              <strong>Review needed.</strong> {warning}
            </p>
          ))}
        </section>
        <section>
          <h2>Files prepared for ZIP</h2>
          <ul className="file-list">
            <li>summary.html</li>
            <li>findings.json</li>
            <li>system-profile.json</li>
            <li>collection-manifest.json</li>
            <li>redaction-manifest.json</li>
            <li>checksums.sha256</li>
            <li>evidence/*.json</li>
          </ul>
        </section>
      </div>
      <section className="evidence-review">
        <h2>Redacted evidence</h2>
        {supportCase.evidence.map((item) => (
          <details key={item.id}>
            <summary>
              <span>{humanize(item.kind)}</span>
              <code>{item.id}</code>
            </summary>
            <pre>{JSON.stringify(item.data, null, 2)}</pre>
          </details>
        ))}
      </section>
      <div className="action-row">
        <button className="button primary" onClick={onExport}>
          Continue to export
        </button>
        <span>Exports never upload automatically.</span>
      </div>
    </div>
  );
}

function ExportView({
  supportCase,
  loading,
  onExport,
}: {
  supportCase: SupportCase;
  loading: boolean;
  onExport: (format: ExportFormat) => void;
}) {
  return (
    <div className="page">
      <PageHeader
        title="Export support case"
        description="Create a local file in the app export directory. Sharing remains your decision."
        meta={`Schema ${supportCase.schemaVersion}`}
      />
      <div className="export-grid">
        <ExportCard
          title="Support bundle"
          format="zip"
          description="Self-contained ZIP with offline report, JSON evidence, manifest, and SHA-256 checksums."
          primary
          loading={loading}
          onExport={onExport}
        />
        <ExportCard
          title="Offline report"
          format="html"
          description="Readable static HTML for a non-technical user and an IT technician."
          loading={loading}
          onExport={onExport}
        />
        <ExportCard
          title="Structured case"
          format="json"
          description="The complete versioned, privacy-reviewed support-case document."
          loading={loading}
          onExport={onExport}
        />
      </div>
      <section className="export-history">
        <h2>Export history</h2>
        {supportCase.exportHistory.length ? (
          <ul>
            {supportCase.exportHistory.map((item) => (
              <li key={`${item.exportedAt}-${item.format}`}>
                <span>{item.destinationName}</span>
                <time>{formatDate(item.exportedAt)}</time>
              </li>
            ))}
          </ul>
        ) : (
          <p>No exports have been created for this case.</p>
        )}
      </section>
    </div>
  );
}

function ExportCard({
  title,
  format,
  description,
  primary = false,
  loading,
  onExport,
}: {
  title: string;
  format: ExportFormat;
  description: string;
  primary?: boolean;
  loading: boolean;
  onExport: (format: ExportFormat) => void;
}) {
  return (
    <article className={`export-card ${primary ? "recommended" : ""}`}>
      {primary && <span className="plain-label">Complete format</span>}
      <h2>{title}</h2>
      <p>{description}</p>
      <code>.{format}</code>
      <button
        className={`button ${primary ? "primary" : "secondary"}`}
        disabled={loading}
        aria-busy={loading}
        onClick={() => onExport(format)}
      >
        {loading ? "Creating export…" : `Create ${format.toUpperCase()}`}
      </button>
    </article>
  );
}

function History({
  cases,
  loading,
  onOpen,
  onDelete,
}: {
  cases: CaseSummary[];
  loading: boolean;
  onOpen: (id: string) => void;
  onDelete: (id: string) => void;
}) {
  return (
    <div className="page">
      <PageHeader
        title="Case history"
        description="Privacy-reviewed cases stored locally on this device."
      />
      {cases.length === 0 ? (
        <EmptyState
          title="No local cases"
          text="Run a diagnostic to create your first case."
        />
      ) : (
        <div className="case-list">
          {cases.map((item) => (
            <article key={item.caseId}>
              <div>
                <span className={`severity ${item.highestSeverity ?? "info"}`}>
                  {item.highestSeverity
                    ? humanize(item.highestSeverity)
                    : "No findings"}
                </span>
                <h2>{humanize(item.category)}</h2>
                <p>
                  {formatDate(item.createdAt)} · {item.findingCount} findings ·{" "}
                  {humanize(item.scanStatus)}
                </p>
                <code>{item.caseId}</code>
              </div>
              <div className="case-actions">
                <button
                  className="button secondary"
                  disabled={loading}
                  onClick={() => onOpen(item.caseId)}
                >
                  Open case
                </button>
                <button
                  className="button danger"
                  disabled={loading}
                  onClick={() => onDelete(item.caseId)}
                >
                  Move to recovery
                </button>
              </div>
            </article>
          ))}
        </div>
      )}
    </div>
  );
}

function Settings({
  storage,
  redactSsids,
  onRedactSsids,
}: {
  storage: StorageInfo | null;
  redactSsids: boolean;
  onRedactSsids: (value: boolean) => void;
}) {
  return (
    <div className="page">
      <PageHeader
        title="Settings"
        description="Local privacy defaults and storage boundaries."
      />
      <section className="settings-section">
        <h2>Privacy defaults</h2>
        <label
          className="toggle-row"
          htmlFor="settings-redact-ssids"
          aria-label="Redact Wi-Fi names"
        >
          <input
            id="settings-redact-ssids"
            type="checkbox"
            checked={redactSsids}
            onChange={(event) => onRedactSsids(event.target.checked)}
          />
          <span>
            <strong>Redact Wi-Fi names</strong>
            <small>Applied to new scans. Existing cases are unchanged.</small>
          </span>
        </label>
      </section>
      <section className="settings-section">
        <h2>Local storage</h2>
        {storage ? (
          <dl className="storage-list">
            <div>
              <dt>Cases</dt>
              <dd>
                <code>{storage.caseDirectory}</code>
              </dd>
            </div>
            <div>
              <dt>Exports</dt>
              <dd>
                <code>{storage.exportDirectory}</code>
              </dd>
            </div>
            <div>
              <dt>Raw evidence persisted</dt>
              <dd>{storage.rawEvidencePersisted ? "Yes" : "No"}</dd>
            </div>
          </dl>
        ) : (
          <div className="loading-line">Reading storage information…</div>
        )}
        <p>
          Moving a case to recovery removes it from history but keeps a
          recoverable app-owned copy. Exported files are independent and are
          never deleted automatically.
        </p>
      </section>
      <section className="boundary-panel">
        <h2>Product boundary</h2>
        <p>
          No telemetry, cloud backend, account, automatic upload, remote access,
          AI diagnosis, or repair execution.
        </p>
      </section>
    </div>
  );
}

function EmptyState({ title, text }: { title: string; text: string }) {
  return (
    <div className="empty-state">
      <span className="empty-mark" aria-hidden="true">
        —
      </span>
      <h2>{title}</h2>
      <p>{text}</p>
    </div>
  );
}
