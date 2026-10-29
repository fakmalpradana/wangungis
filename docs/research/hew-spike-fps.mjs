// WangunGIS spike (ADR-000 criterion #2): orbit FPS of Hew's web viewport on a
// ~1M-face model, in headed-off Chromium forced onto the Metal GPU.
// Usage: node fps.mjs <playwright-module-path> <url> <model> [seconds]
const [, , pwPath, url, model, secs = '10'] = process.argv;
const { chromium } = await import(pwPath);
const browser = await chromium.launch({
  headless: true,
  args: ['--use-angle=metal', '--enable-gpu', '--ignore-gpu-blocklist', '--disable-gpu-vsync', '--disable-frame-rate-limit'],
});
const page = await browser.newPage({ viewport: { width: 1920, height: 1080 }, deviceScaleFactor: 1 });
await page.addInitScript(() => {
  window.__draws = 0;
  for (const proto of [WebGL2RenderingContext.prototype]) {
    for (const fn of ['drawElements', 'drawArrays', 'drawElementsInstanced', 'drawArraysInstanced']) {
      const orig = proto[fn];
      proto[fn] = function (...a) { window.__draws++; return orig.apply(this, a); };
    }
  }
});
await page.goto(url);
await page.waitForFunction(() => window.__hew_test && window.__hew_test.isReady(), null, { timeout: 60000 });
const out = await page.evaluate(async ({ model, secs }) => {
  const h = window.__hew_test;
  const gl = document.createElement('canvas').getContext('webgl2');
  const ext = gl && gl.getExtension('WEBGL_debug_renderer_info');
  const gpu = ext ? gl.getParameter(ext.UNMASKED_RENDERER_WEBGL) : 'n/a';
  const t0 = performance.now();
  const bytes = new Uint8Array(await (await fetch(model)).arrayBuffer());
  const t1 = performance.now();
  h.load(bytes);
  const t2 = performance.now();
  h.zoomExtents();
  await new Promise((r) => setTimeout(r, 1500));
  const cam = h.getCamera();
  const c = cam.target;
  const dx = cam.position[0] - c[0], dy = cam.position[1] - c[1], dz = cam.position[2] - c[2];
  const rad = Math.hypot(dx, dy);
  const d0 = window.__draws;
  const f0 = h.frameCount();
  let raf = 0, last = performance.now();
  const gaps = [];
  const s0 = performance.now();
  await new Promise((done) => {
    function step(t) {
      raf++; gaps.push(t - last); last = t;
      const a = Math.atan2(dy, dx) + ((t - s0) / 1000) * 0.6;
      h.setCamera({ position: [c[0] + rad * Math.cos(a), c[1] + rad * Math.sin(a), c[2] + dz], target: c });
      if (t - s0 < secs * 1000) requestAnimationFrame(step); else done();
    }
    requestAnimationFrame(step);
  });
  const dt = (performance.now() - s0) / 1000;
  gaps.sort((p, q) => p - q);
  return {
    gpu, model, mb: +(bytes.length / 1e6).toFixed(1),
    fetchMs: Math.round(t1 - t0), loadMs: Math.round(t2 - t1),
    heapMB: performance.memory ? Math.round(performance.memory.usedJSHeapSize / 1e6) : null,
    seconds: +dt.toFixed(2), rafFps: +(raf / dt).toFixed(1),
    renderFps: +((h.frameCount() - f0) / dt).toFixed(1),
    drawsPerFrame: Math.round((window.__draws - d0) / Math.max(1, h.frameCount() - f0)),
    medianFrameMs: +gaps[Math.floor(gaps.length / 2)].toFixed(2),
    p95FrameMs: +gaps[Math.floor(gaps.length * 0.95)].toFixed(2),
  };
}, { model, secs: Number(secs) });
console.log(JSON.stringify(out));
await browser.close();
