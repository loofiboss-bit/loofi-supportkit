export type Platform = "windows_11_x64" | "fedora_kde_44_x64";
export type Category =
  | "performance"
  | "network"
  | "updates"
  | "storage"
  | "drivers_graphics"
  | "services"
  | "general";
export type Severity = "critical" | "warning" | "notice" | "info";
export type Confidence =
  | "confirmed_problem"
  | "strong_indicator"
  | "possible_cause"
  | "informational_observation";
export type CollectorState =
  "complete" | "partial" | "skipped" | "failed" | "timed_out" | "cancelled";
export type ExportFormat = "zip" | "html" | "json";

export interface Evidence {
  id: string;
  collectorId: string;
  kind: string;
  collectedAt: string;
  data: Record<string, unknown>;
}

export interface Finding {
  ruleId: string;
  title: string;
  severity: Severity;
  confidence: Confidence;
  platform: Platform;
  category: Category;
  explanation: string;
  evidenceRefs: string[];
  likelyImpact: string;
  recommendedNextSteps: string[];
  documentationLinks: string[];
  ruleVersion: number;
}

export interface CollectorRecord {
  collectorId: string;
  status: CollectorState;
  message: string | null;
  durationMs: number;
  outputTruncated: boolean;
}

export interface SupportCase {
  schemaVersion: "1.0.0";
  caseId: string;
  createdAt: string;
  appVersion: string;
  platform: Platform;
  category: Category;
  scanStatus: "complete" | "partial" | "cancelled";
  systemProfile: {
    osName: string;
    osVersion: string;
    architecture: string;
    desktop: string | null;
    hostname: string | null;
  };
  collectionManifest: CollectorRecord[];
  findings: Finding[];
  evidence: Evidence[];
  redactionManifest: {
    version: 1;
    categories: Record<string, number>;
    transformedPaths: string[];
    warnings: string[];
  };
  exportHistory: Array<{
    exportedAt: string;
    format: ExportFormat;
    destinationName: string;
  }>;
}

export interface CaseSummary {
  caseId: string;
  createdAt: string;
  platform: Platform;
  category: Category;
  scanStatus: "complete" | "partial" | "cancelled";
  findingCount: number;
  highestSeverity: Severity | null;
}

export interface StorageInfo {
  caseDirectory: string;
  exportDirectory: string;
  rawEvidencePersisted: boolean;
}
