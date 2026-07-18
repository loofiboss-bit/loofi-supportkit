import { expect, test } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";

async function expectNoSeriousAccessibilityViolations(
  page: import("@playwright/test").Page,
) {
  const result = await new AxeBuilder({ page })
    .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa"])
    .analyze();
  const violations = result.violations.filter((item) =>
    ["serious", "critical"].includes(item.impact ?? ""),
  );
  expect(
    violations,
    violations.map((item) => `${item.id}: ${item.help}`).join("\n"),
  ).toEqual([]);
}

test("browser demo completes the diagnostic-to-export workflow", async ({
  page,
}) => {
  await page.goto("/");
  await expect(
    page.getByRole("heading", {
      name: "Turn system evidence into a support case.",
    }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Start a diagnostic" }).click();
  await page.getByRole("radio", { name: /General diagnostic scan/ }).check();
  await page.getByRole("button", { name: "Run read-only scan" }).click();
  await expect(page.getByRole("heading", { name: "Findings" })).toBeVisible();
  await page.getByRole("button", { name: "Review privacy changes" }).click();
  await expect(
    page.getByRole("heading", { name: "Privacy review" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Continue to export" }).click();
  await page.getByRole("button", { name: "Create ZIP" }).click();
  await expect(
    page.getByRole("status").filter({ hasText: "ZIP export created" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Case history" }).click();
  await expect(
    page.getByRole("heading", { name: "Case history" }),
  ).toBeVisible();
  await expect(page.getByRole("button", { name: "Open case" })).toBeVisible();
});

test("core pages fit narrow laptop and phone widths without horizontal overflow", async ({
  page,
}) => {
  for (const width of [320, 375, 414, 768]) {
    await page.setViewportSize({ width, height: 800 });
    await page.goto("/");
    const expectNoOverflow = async (view: string) => {
      const overflow = await page.evaluate(
        () =>
          document.documentElement.scrollWidth >
          document.documentElement.clientWidth,
      );
      expect(overflow, `${view} horizontally overflows at ${width}px`).toBe(
        false,
      );
    };
    await expectNoOverflow("home");
    await page.getByRole("button", { name: "Start a diagnostic" }).click();
    await expectNoOverflow("new diagnostic");
    await page.getByRole("button", { name: "Run read-only scan" }).click();
    await expect(page.getByRole("heading", { name: "Findings" })).toBeVisible();
    await expectNoOverflow("findings");
    await page.getByRole("button", { name: "Review privacy changes" }).click();
    await expectNoOverflow("privacy review");
    await page.getByRole("button", { name: "Continue to export" }).click();
    await expectNoOverflow("export");
  }
});

test("home primary action fits a 1280 by 800 laptop viewport", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/");
  const action = await page
    .getByRole("button", { name: "Start a diagnostic" })
    .boundingBox();
  expect(action).not.toBeNull();
  expect(action!.y + action!.height).toBeLessThanOrEqual(800);
});

test("core workflow has no serious WCAG accessibility violations", async ({
  page,
}) => {
  await page.goto("/");
  await expectNoSeriousAccessibilityViolations(page);
  await page.getByRole("button", { name: "Start a diagnostic" }).click();
  await expectNoSeriousAccessibilityViolations(page);
  await page.getByRole("button", { name: "Run read-only scan" }).click();
  await expect(page.getByRole("heading", { name: "Findings" })).toBeVisible();
  await expectNoSeriousAccessibilityViolations(page);
  await page.getByRole("button", { name: "Review privacy changes" }).click();
  await expectNoSeriousAccessibilityViolations(page);
  await page.getByRole("button", { name: "Continue to export" }).click();
  await expectNoSeriousAccessibilityViolations(page);
});
