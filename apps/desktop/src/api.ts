import { invoke } from "@tauri-apps/api/core";
import type {
  CaseSummary,
  Category,
  ExportFormat,
  StorageInfo,
  SupportCase,
} from "./types";

const browserCases = new Map<string, SupportCase>();
const isTauri = () => "__TAURI_INTERNALS__" in window;

const fixtureCase = (category: Category): SupportCase => ({
  schemaVersion: "1.0.0",
  caseId: crypto.randomUUID(),
  createdAt: new Date().toISOString(),
  appVersion: "0.1.0-demo",
  platform: "fedora_kde_44_x64",
  category,
  scanStatus: "partial",
  systemProfile: {
    osName: "Fedora Linux",
    osVersion: "44",
    architecture: "x86_64",
    desktop: "KDE Plasma 6",
    hostname: "[REDACTED:hostname]",
  },
  collectionManifest: [
    {
      collectorId: "fedora.system-profile",
      status: "complete",
      message: null,
      durationMs: 12,
      outputTruncated: false,
    },
    {
      collectorId: "common.disk-usage",
      status: "complete",
      message: null,
      durationMs: 24,
      outputTruncated: false,
    },
    {
      collectorId: "fedora.failed-units",
      status: "complete",
      message: null,
      durationMs: 31,
      outputTruncated: false,
    },
    {
      collectorId: "fedora.update-state",
      status: "skipped",
      message: "Synthetic browser demo: no host command was run.",
      durationMs: 0,
      outputTruncated: false,
    },
  ],
  findings: [
    {
      ruleId: "common.storage.root-critical",
      title: "System disk is critically full",
      severity: "critical",
      confidence: "confirmed_problem",
      platform: "fedora_kde_44_x64",
      category,
      explanation: "The system disk has crossed the critical usage threshold.",
      evidenceRefs: ["evidence.storage.root"],
      likelyImpact:
        "Updates, applications, and the desktop session may fail or become slow.",
      recommendedNextSteps: [
        "Review large personal files and application caches.",
        "Keep at least 15 percent free space before retrying updates.",
      ],
      documentationLinks: [
        "https://docs.fedoraproject.org/en-US/quick-docs/disk-usage/",
      ],
      ruleVersion: 1,
    },
    {
      ruleId: "fedora.services.failed-units",
      title: "System services have failed",
      severity: "warning",
      confidence: "strong_indicator",
      platform: "fedora_kde_44_x64",
      category,
      explanation: "Two systemd units are in a failed state.",
      evidenceRefs: ["evidence.services.failed"],
      likelyImpact:
        "Features provided by the failed services may be unavailable.",
      recommendedNextSteps: [
        "Review the bounded journal evidence.",
        "Share this case with an administrator before changing service state.",
      ],
      documentationLinks: [
        "https://www.freedesktop.org/software/systemd/man/latest/systemctl.html",
      ],
      ruleVersion: 1,
    },
  ],
  evidence: [
    {
      id: "evidence.storage.root",
      collectorId: "common.disk-usage",
      kind: "disk_usage",
      collectedAt: new Date().toISOString(),
      data: { mount: "/", usedPercent: 94, availableBytes: 8589934592 },
    },
    {
      id: "evidence.services.failed",
      collectorId: "fedora.failed-units",
      kind: "failed_services",
      collectedAt: new Date().toISOString(),
      data: {
        failedCount: 2,
        units: ["fixture-a.service", "fixture-b.service"],
        sourceAddress: "[REDACTED:ip_address]",
      },
    },
    {
      id: "evidence.network.connections",
      collectorId: "fedora.network",
      kind: "network_state",
      collectedAt: new Date().toISOString(),
      data: { connected: true, ssid: "[REDACTED:wifi_ssid]" },
    },
  ],
  redactionManifest: {
    version: 1,
    categories: { hostname: 1, ip_address: 1, wifi_ssid: 1 },
    transformedPaths: [
      "$.systemProfile.hostname",
      "$.evidence[1].data.sourceAddress",
      "$.evidence[2].data.ssid",
    ],
    warnings: [
      "Review unclassified free-text evidence before sharing this case.",
    ],
  },
  exportHistory: [],
});

export const api = {
  demoMode: !isTauri(),
  async runScan(
    category: Category,
    redactSsids: boolean,
  ): Promise<SupportCase> {
    if (isTauri()) return invoke("run_scan", { category, redactSsids });
    await new Promise((resolve) => setTimeout(resolve, 650));
    const supportCase = fixtureCase(category);
    if (!redactSsids) delete supportCase.redactionManifest.categories.wifi_ssid;
    browserCases.set(supportCase.caseId, supportCase);
    return supportCase;
  },
  async listCases(): Promise<CaseSummary[]> {
    if (isTauri()) return invoke("list_cases");
    return [...browserCases.values()].map((item) => ({
      caseId: item.caseId,
      createdAt: item.createdAt,
      platform: item.platform,
      category: item.category,
      scanStatus: item.scanStatus,
      findingCount: item.findings.length,
      highestSeverity: item.findings[0]?.severity ?? null,
    }));
  },
  async loadCase(caseId: string): Promise<SupportCase> {
    if (isTauri()) return invoke("load_case", { caseId });
    const item = browserCases.get(caseId);
    if (!item) throw new Error("The selected demo case no longer exists.");
    return item;
  },
  async deleteCase(caseId: string): Promise<void> {
    if (isTauri()) return invoke("delete_case", { caseId });
    browserCases.delete(caseId);
  },
  async exportCase(
    caseId: string,
    format: ExportFormat,
  ): Promise<{ destination: string; format: ExportFormat }> {
    if (isTauri()) return invoke("export_case", { caseId, format });
    await new Promise((resolve) => setTimeout(resolve, 250));
    return {
      destination: `Browser demo only — supportkit-case-${caseId}.${format}`,
      format,
    };
  },
  async getStorageInfo(): Promise<StorageInfo> {
    if (isTauri()) return invoke("get_storage_info");
    return {
      caseDirectory: "Browser demo memory (not persisted)",
      exportDirectory: "Browser demo (files are not written)",
      rawEvidencePersisted: false,
    };
  },
};
