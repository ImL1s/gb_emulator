import init, { WasmEmulator } from './pkg/gb_emulator.js';

// Application State
let wasmMemory = null;
let emulator = null;
let isPaused = false;
let animFrameId = null;
let frameCount = 0;
let lastFpsUpdate = performance.now();
let currentPalette = 'green';
let currentRomName = '';

// Palette color LUT (Lightness index 0..3 -> RGBA bytes)
const PALETTES = {
  green: [
    [155, 188, 15, 255],  // Color 0: Lightest green #9bbc0f
    [139, 172, 15, 255],  // Color 1: Light green #8bac0f
    [48, 98, 48, 255],    // Color 2: Dark green #306230
    [15, 56, 15, 255]     // Color 3: Darkest green #0f380f
  ],
  grayscale: [
    [255, 255, 255, 255], // Color 0: White
    [170, 170, 170, 255], // Color 1: Light Gray
    [85, 85, 85, 255],   // Color 2: Dark Gray
    [0, 0, 0, 255]        // Color 3: Black
  ],
  pocket: [
    [224, 248, 208, 255], // Color 0: Soft Light
    [136, 192, 112, 255], // Color 1: Mid Light
    [52, 104, 86, 255],   // Color 2: Mid Dark
    [8, 24, 32, 255]      // Color 3: Darkest
  ]
};

// Joypad Key Index Mappings matching WasmEmulator (0:Right, 1:Left, 2:Up, 3:Down, 4:A, 5:B, 6:Select, 7:Start)
const KEY_MAP = {
  'ArrowRight': 0,
  'ArrowLeft': 1,
  'ArrowUp': 2,
  'ArrowDown': 3,
  'KeyZ': 4,
  'KeyJ': 4,
  'KeyX': 5,
  'KeyK': 5,
  'ShiftRight': 6,
  'ShiftLeft': 6,
  'Enter': 7
};

const TOUCH_MAP = {
  'right': 0,
  'left': 1,
  'up': 2,
  'down': 3,
  'a': 4,
  'b': 5,
  'select': 6,
  'start': 7
};

// Canvas context
let canvas = null;
let ctx = null;

// Initialize WASM module and UI event listeners
async function main() {
  canvas = document.getElementById('gb-canvas');
  if (canvas) {
    ctx = canvas.getContext('2d');
  }

  try {
    const wasmModule = await init();
    wasmMemory = wasmModule.memory;
    emulator = new WasmEmulator();
    console.log('WASM Game Boy Emulator initialized successfully.');

    setupUIListeners();
    setupInputListeners();
    setupDragAndDrop();
    
    // Start main animation loop
    requestAnimationFrame(gameLoop);
  } catch (err) {
    console.error('Failed to initialize WASM module:', err);
    const statusState = document.getElementById('status-state');
    if (statusState) {
      statusState.textContent = 'WASM Init Error';
      statusState.style.backgroundColor = '#8b0000';
    }
  }
}

// Render one frame from WASM memory to Canvas
function renderFrame() {
  if (!emulator || !wasmMemory || !ctx) return;

  const ptr = emulator.get_framebuffer_ptr();
  const rawBuffer = new Uint8ClampedArray(wasmMemory.buffer, ptr, 160 * 144 * 4);

  if (currentPalette === 'grayscale') {
    const imageData = new ImageData(rawBuffer, 160, 144);
    ctx.putImageData(imageData, 0, 0);
  } else {
    const lut = PALETTES[currentPalette] || PALETTES.green;
    const remapped = new Uint8ClampedArray(160 * 144 * 4);

    for (let i = 0; i < rawBuffer.length; i += 4) {
      const r = rawBuffer[i]; // Lightness value (0-255)
      let colorIdx = 0;
      if (r > 192) colorIdx = 0;
      else if (r > 128) colorIdx = 1;
      else if (r > 64) colorIdx = 2;
      else colorIdx = 3;

      const [pr, pg, pb, pa] = lut[colorIdx];
      remapped[i]     = pr;
      remapped[i + 1] = pg;
      remapped[i + 2] = pb;
      remapped[i + 3] = pa;
    }
    const imageData = new ImageData(remapped, 160, 144);
    ctx.putImageData(imageData, 0, 0);
  }
}

// Main 60FPS Game Loop
function gameLoop(timestamp) {
  if (!isPaused && emulator) {
    emulator.step_frame();
    renderFrame();

    // FPS Calculation
    frameCount++;
    if (timestamp - lastFpsUpdate >= 1000) {
      const fps = ((frameCount * 1000) / (timestamp - lastFpsUpdate)).toFixed(1);
      const fpsEl = document.getElementById('status-fps');
      if (fpsEl) fpsEl.textContent = `FPS: ${fps}`;
      frameCount = 0;
      lastFpsUpdate = timestamp;
    }
  }
  animFrameId = requestAnimationFrame(gameLoop);
}

// Extract ASCII ROM title from header (0x0134..0x0143)
function extractRomTitle(romBytes) {
  if (romBytes.length < 0x0143) return 'Unknown ROM';
  let title = '';
  for (let i = 0x0134; i <= 0x0143; i++) {
    const c = romBytes[i];
    if (c === 0) break;
    if (c >= 32 && c <= 126) title += String.fromCharCode(c);
  }
  return title.trim() || 'Untitled ROM';
}

// Load ROM bytes into emulator core
async function loadRomBuffer(romBytes, romName = 'Loaded ROM') {
  if (!emulator) return;
  try {
    emulator.load_rom(romBytes);
    const title = extractRomTitle(romBytes);
    currentRomName = title;

    const titleEl = document.getElementById('status-title');
    const stateEl = document.getElementById('status-state');
    const powerLed = document.getElementById('power-led');
    const pauseBtn = document.getElementById('pause-btn');

    if (titleEl) titleEl.textContent = `ROM: ${title}`;
    if (stateEl) {
      stateEl.textContent = 'Running';
      stateEl.style.backgroundColor = '#303525';
    }
    if (powerLed) powerLed.classList.add('on');
    if (pauseBtn) {
      pauseBtn.disabled = false;
      pauseBtn.textContent = '⏸ Pause';
    }
    isPaused = false;
    console.log(`Loaded ROM: ${title} (${romBytes.length} bytes)`);
  } catch (err) {
    console.error('Failed to load ROM:', err);
    alert('Failed to load ROM file into emulator core: ' + err);
  }
}

// Setup Header & UI Button Listeners
function setupUIListeners() {
  // File Picker
  const romInput = document.getElementById('rom-input');
  if (romInput) {
    romInput.addEventListener('change', (e) => {
      const files = e.target.files;
      if (files && files.length > 0) {
        const file = files[0];
        const reader = new FileReader();
        reader.onload = (evt) => {
          const bytes = new Uint8Array(evt.target.result);
          loadRomBuffer(bytes, file.name);
        };
        reader.readAsArrayBuffer(file);
      }
    });
  }

  // Preset 2048 Game Buttons (supports both id variants)
  const presetBtn = document.getElementById('btn-load-2048') || document.getElementById('preset-2048-btn');
  if (presetBtn) {
    presetBtn.addEventListener('click', async () => {
      try {
        let resp = await fetch('2048.gb');
        if (!resp.ok) {
          resp = await fetch('../examples/2048.gb');
        }
        if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
        const buffer = await resp.arrayBuffer();
        await loadRomBuffer(new Uint8Array(buffer), '2048.gb');
      } catch (err) {
        console.error('Error fetching 2048 preset ROM:', err);
        alert('Could not fetch preset 2048.gb ROM file.');
      }
    });
  }

  // Palette Selector
  const paletteSelect = document.getElementById('palette-select');
  if (paletteSelect) {
    paletteSelect.addEventListener('change', (e) => {
      currentPalette = e.target.value;
      renderFrame();
    });
  }

  // Pause Button
  const pauseBtn = document.getElementById('pause-btn');
  if (pauseBtn) {
    pauseBtn.addEventListener('click', () => {
      isPaused = !isPaused;
      pauseBtn.textContent = isPaused ? '▶ Resume' : '⏸ Pause';
      const stateEl = document.getElementById('status-state');
      if (stateEl) {
        stateEl.textContent = isPaused ? 'Paused' : 'Running';
      }
    });
  }
}

// Drag and Drop File Handlers
function setupDragAndDrop() {
  const dropZone = document.getElementById('drop-zone') || document.body;
  const overlay = document.getElementById('drop-overlay');

  window.addEventListener('dragover', (e) => {
    e.preventDefault();
    if (overlay) overlay.classList.remove('hidden');
  });

  window.addEventListener('dragleave', (e) => {
    if (e.target === overlay || e.target === document.body) {
      if (overlay) overlay.classList.add('hidden');
    }
  });

  window.addEventListener('drop', (e) => {
    e.preventDefault();
    if (overlay) overlay.classList.add('hidden');
    const files = e.dataTransfer.files;
    if (files && files.length > 0) {
      const file = files[0];
      const reader = new FileReader();
      reader.onload = (evt) => {
        const bytes = new Uint8Array(evt.target.result);
        loadRomBuffer(bytes, file.name);
      };
      reader.readAsArrayBuffer(file);
    }
  });
}

// Input Listeners (Keyboard + Touch/Click)
function setupInputListeners() {
  // Keyboard Listeners
  window.addEventListener('keydown', (e) => {
    if (KEY_MAP[e.code] !== undefined) {
      e.preventDefault();
      if (emulator) emulator.press_key(KEY_MAP[e.code]);
    }
  });

  window.addEventListener('keyup', (e) => {
    if (KEY_MAP[e.code] !== undefined) {
      e.preventDefault();
      if (emulator) emulator.release_key(KEY_MAP[e.code]);
    }
  });

  // Touch & Mouse On-Screen Buttons
  const buttons = document.querySelectorAll('[data-key]');
  buttons.forEach(btn => {
    const keyName = btn.getAttribute('data-key');
    const bitIndex = TOUCH_MAP[keyName];
    if (bitIndex === undefined) return;

    const handlePress = (e) => {
      e.preventDefault();
      btn.classList.add('pressed');
      if (emulator) emulator.press_key(bitIndex);
    };

    const handleRelease = (e) => {
      e.preventDefault();
      btn.classList.remove('pressed');
      if (emulator) emulator.release_key(bitIndex);
    };

    btn.addEventListener('touchstart', handlePress, { passive: false });
    btn.addEventListener('touchend', handleRelease, { passive: false });
    btn.addEventListener('touchcancel', handleRelease, { passive: false });
    btn.addEventListener('mousedown', handlePress);
    btn.addEventListener('mouseup', handleRelease);
    btn.addEventListener('mouseleave', handleRelease);
  });
}

// Start app when DOM is ready
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', main);
} else {
  main();
}
