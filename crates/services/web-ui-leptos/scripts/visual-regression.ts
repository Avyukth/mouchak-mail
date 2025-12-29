/**
 * Visual Regression Test Suite for Mouchak Mail Web UI
 *
 * Uses Playwright via dev-browser skill pattern for screenshot comparison.
 * Captures baselines across 3 viewports x 4 pages = 12 screenshots.
 *
 * Usage:
 *   bun run scripts/visual-regression.ts baseline  # Generate baselines
 *   bun run scripts/visual-regression.ts compare   # Compare current vs baseline
 *   bun run scripts/visual-regression.ts           # Same as compare
 *
 * Environment:
 *   VR_BASE_URL - Base URL to test (default: http://localhost:1420)
 *   VR_THRESHOLD - Pixel diff threshold (default: 0.001 = 0.1%)
 */

import { chromium, type Page, type Browser } from "playwright";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "fs";
import { join, dirname } from "path";

// ============================================================================
// Configuration
// ============================================================================

interface Viewport {
  name: string;
  width: number;
  height: number;
}

interface TestPage {
  name: string;
  path: string;
}

const VIEWPORTS: Viewport[] = [
  { name: "mobile", width: 375, height: 667 },
  { name: "tablet", width: 768, height: 1024 },
  { name: "desktop", width: 1440, height: 900 },
];

const PAGES: TestPage[] = [
  { name: "home", path: "/" },
  { name: "projects", path: "/projects" },
  { name: "inbox", path: "/inbox" },
  { name: "thread", path: "/thread/1" },
];

const BASE_URL = process.env.VR_BASE_URL || "http://localhost:1420";
const THRESHOLD = parseFloat(process.env.VR_THRESHOLD || "0.001");
const BASELINES_DIR = join(dirname(import.meta.dir), "baselines");
const CURRENT_DIR = join(dirname(import.meta.dir), ".vr-current");

// ============================================================================
// Utilities
// ============================================================================

function ensureDir(dir: string): void {
  if (!existsSync(dir)) {
    mkdirSync(dir, { recursive: true });
  }
}

function screenshotPath(dir: string, page: string, viewport: string): string {
  return join(dir, `${page}-${viewport}.png`);
}

async function waitForPageLoad(page: Page): Promise<void> {
  await page.waitForLoadState("networkidle");
  // Additional wait for WASM hydration
  await page.waitForTimeout(500);
}

function compareBuffers(baseline: Buffer, current: Buffer): number {
  if (baseline.length !== current.length) {
    return 1.0; // Completely different if sizes differ
  }

  let diffPixels = 0;
  const totalPixels = baseline.length;

  for (let i = 0; i < totalPixels; i++) {
    if (baseline[i] !== current[i]) {
      diffPixels++;
    }
  }

  return diffPixels / totalPixels;
}

// ============================================================================
// Screenshot Capture
// ============================================================================

async function captureScreenshots(
  browser: Browser,
  outputDir: string,
  isBaseline: boolean
): Promise<void> {
  const mode = isBaseline ? "Baseline" : "Current";
  console.log(`\n📸 Capturing ${mode} Screenshots`);
  console.log(`   Base URL: ${BASE_URL}`);
  console.log(`   Output: ${outputDir}\n`);

  ensureDir(outputDir);

  for (const viewport of VIEWPORTS) {
    const context = await browser.newContext({
      viewport: { width: viewport.width, height: viewport.height },
    });
    const page = await context.newPage();

    for (const testPage of PAGES) {
      const url = `${BASE_URL}${testPage.path}`;
      const filename = screenshotPath(outputDir, testPage.name, viewport.name);

      process.stdout.write(
        `   ${viewport.name.padEnd(8)} ${testPage.name.padEnd(10)} `
      );

      try {
        await page.goto(url);
        await waitForPageLoad(page);
        await page.screenshot({ path: filename, fullPage: true });
        console.log(`✓ ${filename}`);
      } catch (error) {
        console.log(`✗ Failed: ${error instanceof Error ? error.message : error}`);
      }
    }

    await context.close();
  }
}

// ============================================================================
// Comparison
// ============================================================================

interface ComparisonResult {
  page: string;
  viewport: string;
  baseline: string;
  current: string;
  diffRatio: number;
  passed: boolean;
}

async function compareScreenshots(): Promise<ComparisonResult[]> {
  console.log("\n🔍 Comparing Screenshots");
  console.log(`   Threshold: ${(THRESHOLD * 100).toFixed(2)}%\n`);

  const results: ComparisonResult[] = [];

  for (const viewport of VIEWPORTS) {
    for (const testPage of PAGES) {
      const baselinePath = screenshotPath(
        BASELINES_DIR,
        testPage.name,
        viewport.name
      );
      const currentPath = screenshotPath(
        CURRENT_DIR,
        testPage.name,
        viewport.name
      );

      const result: ComparisonResult = {
        page: testPage.name,
        viewport: viewport.name,
        baseline: baselinePath,
        current: currentPath,
        diffRatio: 0,
        passed: true,
      };

      process.stdout.write(
        `   ${viewport.name.padEnd(8)} ${testPage.name.padEnd(10)} `
      );

      if (!existsSync(baselinePath)) {
        console.log("⚠ No baseline (run 'baseline' first)");
        result.passed = false;
        result.diffRatio = 1.0;
        results.push(result);
        continue;
      }

      if (!existsSync(currentPath)) {
        console.log("⚠ No current screenshot");
        result.passed = false;
        result.diffRatio = 1.0;
        results.push(result);
        continue;
      }

      const baseline = readFileSync(baselinePath);
      const current = readFileSync(currentPath);
      const diffRatio = compareBuffers(baseline, current);

      result.diffRatio = diffRatio;
      result.passed = diffRatio <= THRESHOLD;

      if (result.passed) {
        console.log(`✓ ${(diffRatio * 100).toFixed(3)}% diff`);
      } else {
        console.log(`✗ ${(diffRatio * 100).toFixed(3)}% diff (> ${THRESHOLD * 100}%)`);
      }

      results.push(result);
    }
  }

  return results;
}

// ============================================================================
// Main
// ============================================================================

async function main(): Promise<void> {
  const command = process.argv[2] || "compare";

  console.log("═".repeat(60));
  console.log("  Mouchak Mail - Visual Regression Test Suite");
  console.log("═".repeat(60));

  const browser = await chromium.launch({ headless: true });

  try {
    if (command === "baseline") {
      await captureScreenshots(browser, BASELINES_DIR, true);
      console.log("\n✅ Baselines saved to:", BASELINES_DIR);
      console.log("   Commit these files to track visual changes.\n");
    } else if (command === "compare") {
      // Capture current state
      await captureScreenshots(browser, CURRENT_DIR, false);

      // Compare
      const results = await compareScreenshots();

      // Summary
      const passed = results.filter((r) => r.passed).length;
      const failed = results.filter((r) => !r.passed).length;
      const total = results.length;

      console.log("\n" + "─".repeat(60));
      console.log(`  Results: ${passed}/${total} passed, ${failed} failed`);
      console.log("─".repeat(60));

      if (failed > 0) {
        console.log("\n❌ Visual regression detected!");
        console.log("   Review the differences and update baselines if intentional:\n");
        console.log("   bun run scripts/visual-regression.ts baseline\n");
        process.exit(1);
      } else {
        console.log("\n✅ All visual tests passed!\n");
      }
    } else {
      console.log(`Unknown command: ${command}`);
      console.log("Usage: bun run scripts/visual-regression.ts [baseline|compare]");
      process.exit(1);
    }
  } finally {
    await browser.close();
  }
}

main().catch((error) => {
  console.error("Fatal error:", error);
  process.exit(1);
});
