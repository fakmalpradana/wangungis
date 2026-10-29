// Screenshot a .hew model from straight above after Zoom Extents (render-precision check).
// Usage: node shot.mjs <playwright-module> <url> <model> <out.png>
const [, , pwPath, url, model, out] = process.argv;
const { chromium } = await import(pwPath);
const browser = await chromium.launch({ headless: true, args: ['--use-angle=metal', '--enable-gpu', '--ignore-gpu-blocklist'] });
const page = await browser.newPage({ viewport: { width: 900, height: 900 }, deviceScaleFactor: 1 });
await page.goto(url);
await page.waitForFunction(() => window.__hew_test && window.__hew_test.isReady(), null, { timeout: 60000 });
const cam = await page.evaluate(async (model) => {
  const h = window.__hew_test;
  h.load(new Uint8Array(await (await fetch(model)).arrayBuffer()));
  h.zoomExtents();
  await new Promise((r) => setTimeout(r, 800));
  const c = h.getCamera().target;
  h.setCamera({ position: [c[0], c[1] - 0.01, c[2] + 14], target: c, up: [0, 1, 0], fovDeg: 45 });
  await new Promise((r) => setTimeout(r, 800));
  return h.getCamera();
}, model);
await page.screenshot({ path: out });
console.log(JSON.stringify(cam));
await browser.close();
