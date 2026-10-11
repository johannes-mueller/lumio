import init, { Sim } from "./lumio-sim.js";

// Geometry of the star. All constants are tunable defaults.
const STRIP_NUM = 24;
const STRIP_LENGTH = 60;

const HUB_RATIO = 0.12; // radius of the center hub, relative to the outer radius
const TIP_RATIO = 0.98; // outer end of a beam, relative to the outer radius
const LED_RATIO = 0.006; // LED dot radius, relative to the outer radius
const ANGLE_OFFSET = -Math.PI / 2; // first strip points up

const canvas = document.getElementById("star");
const ctx = canvas.getContext("2d");

const showSelect = document.getElementById("show-select");
const pauseButton = document.getElementById("pause");
const stepButton = document.getElementById("step");
const speedInput = document.getElementById("speed");
const speedValue = document.getElementById("speed-value");
const gammaInput = document.getElementById("gamma");
const gammaValue = document.getElementById("gamma-value");
const autoLed = document.getElementById("auto-led");
const activeLed = document.getElementById("active-led");
const advanceButton = document.getElementById("btn-advance");
const actionButton = document.getElementById("btn-action");

let sim = null;
let outerRadius = 300;
let hubRadius = 0;
let gamma = 0.4;

function resize() {
  const dpr = window.devicePixelRatio || 1;
  const rect = canvas.getBoundingClientRect();
  const size = Math.max(1, Math.min(rect.width, rect.height));
  canvas.width = Math.round(size * dpr);
  canvas.height = Math.round(size * dpr);
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);

  outerRadius = (size / 2) * TIP_RATIO;
  hubRadius = (size / 2) * HUB_RATIO;
}

function draw() {
  const size = canvas.width / (window.devicePixelRatio || 1);
  ctx.clearRect(0, 0, size, size);

  const cx = size / 2;
  const cy = size / 2;
  const frame = sim.frame();

  const ledRadius = (size / 2) * LED_RATIO;
  const span = outerRadius - hubRadius;

  for (let strip = 0; strip < STRIP_NUM; strip++) {
    const angle = ANGLE_OFFSET + (strip * 2 * Math.PI) / STRIP_NUM;
    const dx = Math.cos(angle);
    const dy = Math.sin(angle);

    // Faint beam guide.
    ctx.strokeStyle = "rgba(255,255,255,0.04)";
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(cx + dx * hubRadius, cy + dy * hubRadius);
    ctx.lineTo(cx + dx * outerRadius, cy + dy * outerRadius);
    ctx.stroke();

    for (let pos = 0; pos < STRIP_LENGTH; pos++) {
      const idx = (strip * STRIP_LENGTH + pos) * 3;
      let r = frame[idx];
      let g = frame[idx + 1];
      let b = frame[idx + 2];
      if (r === 0 && g === 0 && b === 0) continue;

      if (gamma !== 1.0) {
        r = Math.round(Math.pow(r / 255.0, gamma) * 255.0);
        g = Math.round(Math.pow(g / 255.0, gamma) * 255.0);
        b = Math.round(Math.pow(b / 255.0, gamma) * 255.0);
      }

      const radius = hubRadius + (span * pos) / (STRIP_LENGTH - 1);
      const x = cx + dx * radius;
      const y = cy + dy * radius;

      ctx.fillStyle = `rgb(${r},${g},${b})`;
      ctx.beginPath();
      ctx.arc(x, y, ledRadius, 0, 2 * Math.PI);
      ctx.fill();
    }
  }

  // Hub.
  ctx.fillStyle = "#0a0d11";
  ctx.beginPath();
  ctx.arc(cx, cy, hubRadius * 0.7, 0, 2 * Math.PI);
  ctx.fill();
}

function syncHud() {
  const index = sim.current_show();
  if (showSelect.value !== String(index)) {
    showSelect.value = String(index);
  }
  autoLed.classList.toggle("on", sim.auto_led());
  activeLed.classList.toggle("on", sim.active_led());
}

function frameLoop(now) {
  sim.tick(now);
  draw();
  syncHud();
  requestAnimationFrame(frameLoop);
}

function setPaused(paused) {
  sim.set_paused(paused);
  pauseButton.textContent = paused ? "Resume" : "Pause";
  stepButton.disabled = !paused;
}

function hold(id, down) {
  sim.press_button(id, down);
}

function bindHold(button, id) {
  const press = (event) => {
    event.preventDefault();
    button.classList.add("pressed");
    button.setPointerCapture?.(event.pointerId);
    hold(id, true);
  };
  const release = (event) => {
    if (!button.classList.contains("pressed")) return;
    event.preventDefault();
    button.classList.remove("pressed");
    hold(id, false);
  };

  button.addEventListener("pointerdown", press);
  button.addEventListener("pointerup", release);
  button.addEventListener("pointercancel", release);
  button.addEventListener("pointerleave", release);
  // Keyboard accessibility.
  button.addEventListener("keydown", (e) => {
    if (e.key === " " || e.key === "Enter") press(e);
  });
  button.addEventListener("keyup", (e) => {
    if (e.key === " " || e.key === "Enter") release(e);
  });
}

async function main() {
  await init();
  sim = new Sim();

  const names = sim.show_names();
  names.forEach((name, index) => {
    const option = document.createElement("option");
    option.value = String(index);
    option.textContent = name;
    showSelect.appendChild(option);
  });
  showSelect.value = String(sim.current_show());

  gammaInput.value = gamma.toFixed(2);
  gammaValue.textContent = gamma.toFixed(2);

  showSelect.addEventListener("change", () => {
    sim.select_show(Number(showSelect.value));
  });

  pauseButton.addEventListener("click", () => setPaused(!sim.is_paused()));

  stepButton.addEventListener("click", () => {
    if (sim.is_paused()) {
      sim.step_frame();
      draw();
      syncHud();
    }
  });

  speedInput.addEventListener("input", () => {
    const value = Number(speedInput.value);
    sim.set_speed(value);
    speedValue.textContent = `${value.toFixed(1)}×`;
  });

  gammaInput.addEventListener("input", () => {
    gamma = Number(gammaInput.value);
    gammaValue.textContent = gamma.toFixed(2);
  });

  bindHold(advanceButton, "advance");
  bindHold(actionButton, "action");

  resize();
  window.addEventListener("resize", resize);

  requestAnimationFrame(frameLoop);
}

main();
