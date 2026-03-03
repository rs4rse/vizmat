import init from "./vizmat_core.js";

(async () => {
  const loader = document.getElementById("app-loader");
  const status = document.getElementById("loader-status");
  const canvas = document.getElementById("bevy-canvas");

  if (!loader) return;

  if (canvas) {
    canvas.addEventListener("contextmenu", (event) => event.preventDefault());
    canvas.addEventListener("mousedown", () => canvas.focus());
  }

  const hideLoader = () => {
    loader.classList.add("hidden");
    window.setTimeout(() => loader.remove(), 260);
  };

  // Initialize WASM module
  try {
    await init(); // This waits for WASM to load
    hideLoader();
  } catch (e) {
    console.error("Failed to initialize WASM module:", e);
    if (status) {
      status.textContent = "Failed to load WASM module.";
    }
  }

  // Fallback if WASM takes too long
  window.setTimeout(() => {
    if (!loader.classList.contains("hidden") && status) {
      status.textContent = "Still loading... first run can take longer.";
    }
  }, 9000);
})();
