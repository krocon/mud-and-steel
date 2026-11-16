// Motion helpers of the unit page: weather over the hero (rain, dust, snow per biome),
// counting numbers and scroll reveals. Everything stands still for visitors who ask
// for reduced motion.

export const calm = window.matchMedia('(prefers-reduced-motion: reduce)');

const WEATHER = {
  // Rain: fast slanted streaks.
  forest: { count: 150, color: '205, 220, 225', make: (w, h) => ({
    x: Math.random() * (w + 200) - 100, y: Math.random() * h,
    v: 900 + Math.random() * 700, len: 10 + Math.random() * 18, a: 0.10 + Math.random() * 0.22,
  }) },
  // Dust: slow motes drifting with the wind.
  desert: { count: 70, color: '240, 205, 150', make: (w, h) => ({
    x: Math.random() * w, y: Math.random() * h,
    v: 14 + Math.random() * 46, r: 0.6 + Math.random() * 2.2, a: 0.08 + Math.random() * 0.3,
    phase: Math.random() * 6.28,
  }) },
  // Snow: flakes that sway while they fall.
  winter: { count: 130, color: '240, 246, 250', make: (w, h) => ({
    x: Math.random() * w, y: Math.random() * h,
    v: 26 + Math.random() * 70, r: 0.8 + Math.random() * 2.1, a: 0.25 + Math.random() * 0.55,
    phase: Math.random() * 6.28,
  }) },
};

export class Weather {
  constructor(canvas) {
    this.canvas = canvas;
    this.g = canvas.getContext('2d');
    this.biome = 'forest';
    this.parts = [];
    this.running = false;
    this.last = 0;
    this.time = 0;
    new ResizeObserver(() => this.resize()).observe(canvas);
    this.resize();
  }

  resize() {
    const ratio = Math.min(window.devicePixelRatio || 1, 2);
    this.w = this.canvas.clientWidth;
    this.h = this.canvas.clientHeight;
    this.canvas.width = Math.max(1, Math.round(this.w * ratio));
    this.canvas.height = Math.max(1, Math.round(this.h * ratio));
    this.g.setTransform(ratio, 0, 0, ratio, 0, 0);
    this.seed();
  }

  seed() {
    const kind = WEATHER[this.biome];
    const count = Math.round(kind.count * Math.min(1, this.w / 1200 + 0.3));
    this.parts = Array.from({ length: count }, () => kind.make(this.w, this.h));
  }

  set(biome) {
    if (!WEATHER[biome] || biome === this.biome) return;
    this.biome = biome;
    this.seed();
  }

  setRunning(on) {
    on = on && !calm.matches;
    if (on === this.running) return;
    this.running = on;
    if (on) {
      this.last = performance.now();
      requestAnimationFrame((t) => this.frame(t));
    } else {
      this.g.clearRect(0, 0, this.w, this.h);
    }
  }

  frame(now) {
    if (!this.running) return;
    const dt = Math.min((now - this.last) / 1000, 0.05);
    this.last = now;
    this.time += dt;
    const { g, w, h, parts, biome } = this;
    const { color } = WEATHER[biome];
    g.clearRect(0, 0, w, h);
    if (biome === 'forest') {
      g.lineWidth = 1;
      for (const p of parts) {
        p.y += p.v * dt;
        p.x -= p.v * 0.18 * dt;
        if (p.y > h) { p.y = -p.len; p.x = Math.random() * (w + 200) - 20; }
        g.strokeStyle = `rgba(${color}, ${p.a})`;
        g.beginPath();
        g.moveTo(p.x, p.y);
        g.lineTo(p.x - p.len * 0.18, p.y + p.len);
        g.stroke();
      }
    } else {
      const wind = biome === 'desert';
      for (const p of parts) {
        const sway = Math.sin(this.time * (wind ? 0.7 : 1.3) + p.phase);
        if (wind) {
          p.x += p.v * dt;
          p.y += sway * 6 * dt;
          if (p.x > w + 4) { p.x = -4; p.y = Math.random() * h; }
        } else {
          p.y += p.v * dt;
          p.x += sway * 18 * dt;
          if (p.y > h + 4) { p.y = -4; p.x = Math.random() * w; }
        }
        g.fillStyle = `rgba(${color}, ${p.a})`;
        g.beginPath();
        g.arc(p.x, p.y, p.r, 0, 6.2832);
        g.fill();
      }
    }
    requestAnimationFrame((t) => this.frame(t));
  }
}

const easeOut = (t) => 1 - (1 - t) ** 4;

/** Counts an element's text up to `value`; `format(number)` returns the text to show. */
export function countUp(node, value, format, duration = 1100) {
  if (calm.matches || !Number.isFinite(value)) { node.textContent = format(value); return; }
  const start = performance.now();
  const step = (now) => {
    const t = Math.min((now - start) / duration, 1);
    node.textContent = format(value * easeOut(t));
    if (t < 1 && node.isConnected) requestAnimationFrame(step);
    else node.textContent = format(value);
  };
  node.textContent = format(0);
  requestAnimationFrame(step);
}

const revealer = new IntersectionObserver((items) => {
  for (const item of items) {
    if (!item.isIntersecting) continue;
    revealer.unobserve(item.target);
    item.target.classList.add('in');
    item.target.dispatchEvent(new CustomEvent('reveal'));
  }
}, { rootMargin: '0px 0px -8% 0px', threshold: 0.08 });

/** Elements with `data-reveal` get the class `in` (and a `reveal` event) once they scroll into view. */
export function reveal(root = document) {
  for (const node of root.querySelectorAll('[data-reveal]:not(.in)')) revealer.observe(node);
}
