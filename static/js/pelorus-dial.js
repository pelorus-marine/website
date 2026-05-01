"use strict";
(() => {
  // src/pelorus-dial.ts
  function buildTicks() {
    const ticks = [];
    for (let i = 0; i < 360; i++) {
      let len;
      let op;
      if (i % 30 === 0) {
        len = 18;
        op = 1;
      } else if (i % 10 === 0) {
        len = 12;
        op = 0.7;
      } else if (i % 5 === 0) {
        len = 7;
        op = 0.45;
      } else {
        len = 3;
        op = 0.25;
      }
      ticks.push({ deg: i, len, op });
    }
    return ticks;
  }
  var CARDINAL_LABELS = [
    "N",
    "30",
    "60",
    "E",
    "120",
    "150",
    "S",
    "210",
    "240",
    "W",
    "300",
    "330"
  ];
  function labelPosition(deg, r) {
    const rad = (deg - 90) * Math.PI / 180;
    return { x: Math.cos(rad) * r, y: Math.sin(rad) * r };
  }
  function createPelorusDialSvg() {
    const ticks = buildTicks();
    const tickLines = ticks.map((t) => {
      const sw = t.deg % 30 === 0 ? 1.5 : 0.8;
      const y2 = -380 + t.len;
      return `<line x1="0" y1="-380" x2="0" y2="${y2}" stroke="var(--pelorus-brass)" stroke-width="${sw}" opacity="${t.op * 0.5}" transform="rotate(${t.deg})" />`;
    }).join("\n");
    const labelEls = CARDINAL_LABELS.map((label, i) => {
      const deg = i * 30;
      const { x, y } = labelPosition(deg, 354);
      const op = i % 3 === 0 ? 1 : 0.5;
      return `<text x="${x.toFixed(2)}" y="${y.toFixed(2)}" text-anchor="middle" dominant-baseline="middle" opacity="${op}" font-family="var(--pelorus-mono)" font-size="14" fill="var(--pelorus-brass)" font-weight="600">${label}</text>`;
    }).join("\n");
    return `<svg class="pelorus-dial" viewBox="-400 -400 800 800" aria-hidden="true" focusable="false">
<circle r="380" fill="none" stroke="var(--pelorus-ink-3)" stroke-width="1" />
<circle r="340" fill="none" stroke="var(--pelorus-ink-3)" stroke-width="0.6" />
<circle r="290" fill="none" stroke="var(--pelorus-ink-4)" stroke-width="1" />
<circle r="260" fill="none" stroke="var(--pelorus-ink-3)" stroke-width="0.4" />
<circle r="200" fill="none" stroke="var(--pelorus-ink-4)" stroke-width="1" />
<circle r="160" fill="none" stroke="var(--pelorus-ink-3)" stroke-width="0.4" />
<circle r="120" fill="none" stroke="var(--pelorus-ink-4)" stroke-width="0.8" />
<g>${tickLines}</g>
<g>${labelEls}</g>
<line x1="-400" y1="0" x2="400" y2="0" stroke="var(--pelorus-ink-4)" stroke-width="0.5" stroke-dasharray="2 4" />
<line x1="0" y1="-400" x2="0" y2="400" stroke="var(--pelorus-ink-4)" stroke-width="0.5" stroke-dasharray="2 4" />
<g class="pelorus-sight">
<line x1="0" y1="-360" x2="0" y2="360" stroke="var(--pelorus-brass)" stroke-width="1.2" opacity="0.9" />
<circle r="8" fill="var(--pelorus-brass)" />
<circle r="3" fill="var(--pelorus-ink)" />
<polygon points="0,-380 -8,-360 8,-360" fill="var(--pelorus-brass)" />
<polygon points="0,380 -8,360 8,360" fill="var(--pelorus-brass)" opacity="0.4" />
</g>
<circle r="14" fill="var(--pelorus-ink-2)" stroke="var(--pelorus-ink-4)" stroke-width="1" />
<circle r="2" fill="var(--pelorus-brass)" />
</svg>`;
  }
  function mountPelorusDial(container) {
    container.innerHTML = createPelorusDialSvg();
  }

  // src/index.ts
  function mount() {
    const host = document.getElementById("pelorus-dial-root");
    if (!host) {
      return;
    }
    mountPelorusDial(host);
  }
  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", mount);
  } else {
    mount();
  }
})();
